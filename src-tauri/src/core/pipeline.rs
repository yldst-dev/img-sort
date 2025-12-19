use crate::core::classifier::{build_classifier, Classifier, ClassifyInput, OllamaClassifier};
use crate::core::db::Db;
use crate::core::decode::{decode_resize_base64_with_options, DecodeOptions};
use crate::core::events::PROGRESS_EVENT;
use crate::core::export::{copy_to_category, copy_to_category_nested};
use crate::core::model::{
    AnalysisEngine, ExportStatus, JobStatus, PhotoDetail, Progress, Scores, Settings,
    StartAnalysisInput,
};
use crate::core::ollama::test_connection;
use crate::core::scan::scan_sources;
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::async_runtime;
use tauri::{AppHandle, Emitter};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub struct Pipeline {
    pub current: Arc<Mutex<Option<ActiveJob>>>,
    pub latest: Arc<Mutex<Option<Progress>>>,
    pub last_job: Arc<Mutex<Option<JobMeta>>>,
}

#[derive(Clone)]
pub struct ActiveJob {
    pub id: String,
    pub cancel: CancellationToken,
}

#[derive(Clone)]
pub struct JobMeta {
    pub export_root: String,
    pub engine: AnalysisEngine,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            current: Arc::new(Mutex::new(None)),
            latest: Arc::new(Mutex::new(None)),
            last_job: Arc::new(Mutex::new(None)),
        }
    }

    pub fn current_progress(&self) -> Option<Progress> {
        self.latest.lock().clone()
    }

    pub fn last_job_meta(&self) -> Option<JobMeta> {
        self.last_job.lock().clone()
    }

    pub fn cancel(&mut self, job_id: &str) -> Result<()> {
        if let Some(active) = &*self.current.lock() {
            if active.id == job_id {
                active.cancel.cancel();
                return Ok(());
            }
        }
        Err(anyhow!("no running job"))
    }

    pub fn start(
        &mut self,
        app: AppHandle,
        db: Arc<Mutex<Db>>,
        settings: Settings,
        input: StartAnalysisInput,
    ) -> Result<String> {
        if self.current.lock().is_some() {
            return Err(anyhow!("job already running"));
        }
        let job_id = Uuid::new_v4().to_string();
        let job_id_for_state = job_id.clone();
        let job_id_return = job_id.clone();
        {
            let mut guard = self.last_job.lock();
            *guard = Some(JobMeta {
                export_root: input.export_root.clone(),
                engine: settings.analysis_engine,
            });
        }
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        let latest = self.latest.clone();
        let latest_clone = latest.clone();
        let handle_app = app.clone();
        let handle_app_for_err = app.clone();
        let current_ref = self.current.clone();
        async_runtime::spawn(async move {
            if let Err(e) = run_job(
                handle_app,
                db,
                settings,
                input,
                job_id.clone(),
                cancel_clone,
                latest_clone.clone(),
                current_ref.clone(),
            )
            .await
            {
                let progress = Progress {
                    job_id: job_id.clone(),
                    status: JobStatus::Error,
                    current_file: None,
                    processed: 0,
                    total: 0,
                    errors: 1,
                };
                let _ = emit_progress(&handle_app_for_err, latest_clone.clone(), progress);
                eprintln!("pipeline error: {}", e);
            }
        });

        *self.current.lock() = Some(ActiveJob {
            id: job_id_for_state,
            cancel,
        });
        Ok(job_id_return)
    }
}

async fn run_job(
    app: AppHandle,
    db: Arc<Mutex<Db>>,
    settings: Settings,
    input: StartAnalysisInput,
    job_id: String,
    cancel: CancellationToken,
    latest: Arc<Mutex<Option<Progress>>>,
    current_ref: Arc<Mutex<Option<ActiveJob>>>,
) -> Result<()> {
    struct JobCleanup {
        current_ref: Arc<Mutex<Option<ActiveJob>>>,
        job_id: String,
    }
    impl Drop for JobCleanup {
        fn drop(&mut self) {
            let mut guard = self.current_ref.lock();
            if let Some(active) = guard.as_ref() {
                if active.id == self.job_id {
                    *guard = None;
                }
            }
        }
    }
    let _cleanup = JobCleanup {
        current_ref: current_ref.clone(),
        job_id: job_id.clone(),
    };

    let source_root = PathBuf::from(&input.source_root);
    let export_root = PathBuf::from(&input.export_root);
    if !source_root.exists() {
        return Err(anyhow!("source path not found"));
    }
    fs::create_dir_all(&export_root)?;
    let files = scan_sources(&source_root)?;
    let total = files.len();
    let job_started = std::time::Instant::now();
    let mut clip_vision_ms_total: u128 = 0;
    let mut clip_vision_count: u64 = 0;
    let requested_concurrency = settings.analysis_concurrency.max(1) as usize;
    let effective_concurrency = if settings.ollama_stream {
        1usize
    } else {
        requested_concurrency
    };
    let mut progress = Progress {
        job_id: job_id.clone(),
        status: JobStatus::Running,
        current_file: None,
        processed: 0,
        total,
        errors: 0,
    };
    emit_progress(&app, latest.clone(), progress.clone())?;

    #[derive(Debug)]
    enum TaskOutcome {
        Finished {
            path: PathBuf,
            file_name: String,
            duration_ms: i64,
            result: Result<PhotoDetail>,
        },
        Canceled,
    }

    let mut join_set: JoinSet<TaskOutcome> = JoinSet::new();
    let mut pending = files.into_iter();
    let mut running: usize = 0;

    let spawn_next = |join_set: &mut JoinSet<TaskOutcome>,
                          pending: &mut std::vec::IntoIter<PathBuf>,
                          running: &mut usize,
                          progress: &mut Progress|
     -> Option<()> {
        let path = pending.next()?;
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("image")
            .to_string();
        let app = app.clone();
        let job_id = job_id.clone();
        let settings = settings.clone();
        let export_root = export_root.clone();
        let cancel = cancel.clone();
        *running += 1;
        progress.current_file = Some(format!("({}/{}) {}", *running, effective_concurrency, file_name));
        join_set.spawn(async move {
            let started = std::time::Instant::now();
            let result = tokio::select! {
                _ = cancel.cancelled() => {
                    return TaskOutcome::Canceled;
                }
                res = process_one(&app, &job_id, &settings, &export_root, &path, &file_name, &cancel) => res,
            };
            let duration_ms = started.elapsed().as_millis() as i64;
            TaskOutcome::Finished {
                path,
                file_name,
                duration_ms,
                result,
            }
        });
        Some(())
    };

    while running < effective_concurrency {
        if spawn_next(&mut join_set, &mut pending, &mut running, &mut progress).is_none() {
            break;
        }
    }
    emit_progress(&app, latest.clone(), progress.clone())?;

    while progress.processed < total {
        if cancel.is_cancelled() {
            join_set.abort_all();
            progress.status = JobStatus::Canceled;
            progress.current_file = None;
            emit_progress(&app, latest.clone(), progress.clone())?;
            return Ok(());
        }

        let joined = tokio::select! {
            _ = cancel.cancelled() => None,
            res = join_set.join_next() => res,
        };

        let Some(joined) = joined else {
            join_set.abort_all();
            progress.status = JobStatus::Canceled;
            progress.current_file = None;
            emit_progress(&app, latest.clone(), progress.clone())?;
            return Ok(());
        };

        let outcome = match joined {
            Ok(v) => v,
            Err(e) => {
                progress.errors += 1;
                progress.processed += 1;
                running = running.saturating_sub(1);
                progress.current_file = Some(format!("병렬 처리 중: {}개", running));
                emit_progress(&app, latest.clone(), progress.clone())?;
                eprintln!("pipeline task join error: {}", e);
                while running < effective_concurrency {
                    if spawn_next(&mut join_set, &mut pending, &mut running, &mut progress).is_none() {
                        break;
                    }
                }
                continue;
            }
        };

        running = running.saturating_sub(1);

        match outcome {
            TaskOutcome::Canceled => {
                join_set.abort_all();
                progress.status = JobStatus::Canceled;
                progress.current_file = None;
                emit_progress(&app, latest.clone(), progress.clone())?;
                return Ok(());
            }
            TaskOutcome::Finished {
                path,
                file_name,
                duration_ms,
                result,
            } => {
                match result {
                    Ok(mut detail) => {
                        detail.analysis_duration_ms = Some(duration_ms);
                        if detail.model.as_deref() == Some("clip-vit-b32-onnx") {
                            if let Some(ms) =
                                extract_u128_field(detail.analysis_log.as_deref(), "vision_infer_ms")
                            {
                                clip_vision_ms_total += ms;
                                clip_vision_count += 1;
                            }
                        }
                        let guard = db.lock();
                        guard.insert_photo(&detail)?;
                    }
                    Err(e) => {
                        progress.errors += 1;
                        let analysis_log = format!(
                            "engine: {engine:?}\nclip_model_dir: {clip_dir:?}\nclip_fallback_to_ollama: {fallback}\n\nbase_url: {base}\nollama_model: {model}\nthink: {think}\nstream: {stream}\nresize_enabled: {re}\nmax_edge: {me}\njpeg_quality: {q}\n\nerror:\n{err}\n",
                            engine = settings.analysis_engine,
                            clip_dir = settings.clip_model_dir,
                            fallback = settings.clip_fallback_to_ollama,
                            base = settings.ollama_base_url,
                            model = settings.ollama_model,
                            think = settings.ollama_think,
                            stream = settings.ollama_stream,
                            re = settings.analysis_resize_enabled,
                            me = settings.analysis_max_edge,
                            q = settings.analysis_jpeg_quality,
                            err = e
                        );
                        let failed_detail = PhotoDetail {
                            id: Uuid::new_v4().to_string(),
                            file_name: file_name.clone(),
                            path: path.to_string_lossy().to_string(),
                            category: crate::core::model::CategoryKey::Other,
                            top_score: 0.0,
                            scores: Scores::default(),
                            tags: vec![],
                            export_status: ExportStatus::Error,
                            error_message: Some(e.to_string()),
                            analysis_log: Some(analysis_log),
                            analysis_duration_ms: Some(duration_ms),
                            caption: None,
                            text_in_image: None,
                            model: Some(match settings.analysis_engine {
                                crate::core::model::AnalysisEngine::Clip => {
                                    "clip-vit-b32-onnx".to_string()
                                }
                                crate::core::model::AnalysisEngine::Ollama => settings.ollama_model.clone(),
                            }),
                            is_valuable: None,
                            valuable_score: None,
                        };
                        let guard = db.lock();
                        let _ = guard.insert_photo(&failed_detail);
                    }
                }

                progress.processed += 1;
                progress.current_file = Some(format!("병렬 처리 중: {}개", running));
                emit_progress(&app, latest.clone(), progress.clone())?;
            }
        }

        while running < effective_concurrency {
            if spawn_next(&mut join_set, &mut pending, &mut running, &mut progress).is_none() {
                break;
            }
        }
    }

    progress.status = JobStatus::Completed;
    progress.current_file = None;
    emit_progress(&app, latest, progress)?;

    let elapsed = job_started.elapsed().as_secs_f64().max(0.001);
    let ips = (total as f64) / elapsed;
    if clip_vision_count > 0 {
        let avg = (clip_vision_ms_total as f64) / (clip_vision_count as f64);
        eprintln!(
            "clip perf: images={} elapsed={:.2}s throughput={:.2} img/s avg_vision_infer_ms={:.1}",
            total, elapsed, ips, avg
        );
    } else {
        eprintln!(
            "perf: images={} elapsed={:.2}s throughput={:.2} img/s",
            total, elapsed, ips
        );
    }
    Ok(())
}

fn extract_u128_field(log: Option<&str>, key: &str) -> Option<u128> {
    let log = log?;
    let needle = format!("{key}: ");
    let idx = log.find(&needle)?;
    let rest = &log[idx + needle.len()..];
    let num = rest
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    num.parse::<u128>().ok()
}

async fn process_one(
    app: &AppHandle,
    job_id: &str,
    settings: &Settings,
    export_root: &PathBuf,
    path: &PathBuf,
    file_name: &str,
    cancel: &CancellationToken,
) -> Result<PhotoDetail> {
    let (engine, classifier) = build_classifier(settings);
    let mut encoded: Option<String> = None;
    let ensure_encoded = |encoded: &mut Option<String>| -> Result<()> {
        if encoded.is_some() {
            return Ok(());
        }
        let out = decode_resize_base64_with_options(
            path,
            DecodeOptions {
                resize_enabled: settings.analysis_resize_enabled,
                max_edge: settings.analysis_max_edge,
                jpeg_quality: settings.analysis_jpeg_quality,
                resize_filter: image::imageops::FilterType::Triangle,
            },
        )?;
        *encoded = Some(out.base64_jpeg);
        Ok(())
    };

    let mut output = match engine {
        crate::core::model::AnalysisEngine::Clip => {
            classifier
                .classify(ClassifyInput {
                    app,
                    job_id,
                    file_name,
                    path,
                    base64_jpeg: None,
                    cancel,
                })
                .await
        }
        crate::core::model::AnalysisEngine::Ollama => {
            ensure_encoded(&mut encoded)?;
            let b64 = encoded.as_deref().unwrap_or_default();
            classifier
                .classify(ClassifyInput {
                    app,
                    job_id,
                    file_name,
                    path,
                    base64_jpeg: Some(b64),
                    cancel,
                })
                .await
        }
    };

    if let Err(clip_err) = &output {
        if engine == crate::core::model::AnalysisEngine::Clip && settings.clip_fallback_to_ollama {
            ensure_encoded(&mut encoded)?;
            let b64 = encoded.as_deref().unwrap_or_default();
            let ollama = OllamaClassifier {
                settings: settings.clone(),
            };
            output = ollama
                .classify(ClassifyInput {
                    app,
                    job_id,
                    file_name,
                    path,
                    base64_jpeg: Some(b64),
                    cancel,
                })
                .await
                .map_err(|ollama_err| {
                    anyhow!(
                        "clip failed and fallback also failed.\n\nclip:\n{clip}\n\nollama:\n{ollama}",
                        clip = clip_err,
                        ollama = ollama_err
                    )
                });
        }
    }

    let out = output?;
    let analysis_log = format!(
        "engine: {engine:?}\nresize_enabled: {re}\nmax_edge: {me}\njpeg_quality: {q}\n\n{rest}",
        engine = settings.analysis_engine,
        re = settings.analysis_resize_enabled,
        me = settings.analysis_max_edge,
        q = settings.analysis_jpeg_quality,
        rest = out.analysis_log
    );

    let category_dir = out.category.dir_name_ko();
    let export_path = if settings.analysis_value_enabled {
        match out.is_valuable {
            Some(true) => copy_to_category_nested(export_root, &["가치있음", category_dir], file_name, path)?,
            Some(false) => {
                copy_to_category_nested(export_root, &["가치없음", category_dir], file_name, path)?
            }
            None => copy_to_category(export_root, category_dir, file_name, path)?,
        }
    } else {
        copy_to_category(export_root, category_dir, file_name, path)?
    };
    let top = out.scores.top();

    Ok(PhotoDetail {
        id: Uuid::new_v4().to_string(),
        file_name: file_name.to_string(),
        path: export_path.to_string_lossy().to_string(),
        category: out.category,
        top_score: top.1,
        scores: out.scores,
        tags: out.tags,
        export_status: ExportStatus::Success,
        error_message: None,
        analysis_log: Some(analysis_log),
        analysis_duration_ms: None,
        caption: out.caption,
        text_in_image: out.text_in_image,
        model: Some(out.model),
        is_valuable: out.is_valuable,
        valuable_score: out.valuable_score,
    })
}

fn emit_progress(
    app: &AppHandle,
    latest: Arc<Mutex<Option<Progress>>>,
    progress: Progress,
) -> Result<()> {
    {
        let mut guard = latest.lock();
        *guard = Some(progress.clone());
    }
    app.emit(PROGRESS_EVENT, progress)?;
    Ok(())
}

pub async fn test_ollama_connection(base_url: &str) -> Result<String> {
    test_connection(base_url).await
}
