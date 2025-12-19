use crate::core::clip::{preprocess::preprocess_clip_image, ClipEngine, ClipEngineOptions};
use crate::core::events::STREAM_EVENT;
use crate::core::model::{AnalysisEngine, CategoryKey, Scores, Settings, StreamChunk};
use crate::core::ollama::{classify_image_streaming_with_options, classify_image_with_options};
use anyhow::Result;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

pub struct ClassificationOutput {
    pub model: String,
    pub scores: Scores,
    pub category: CategoryKey,
    pub tags: Vec<String>,
    pub caption: Option<String>,
    pub text_in_image: Option<String>,
    pub analysis_log: String,
    pub is_valuable: Option<bool>,
    pub valuable_score: Option<f32>,
}

pub struct ClassifyInput<'a> {
    pub app: &'a AppHandle,
    pub job_id: &'a str,
    pub file_name: &'a str,
    pub path: &'a Path,
    pub base64_jpeg: Option<&'a str>,
    pub cancel: &'a CancellationToken,
}

pub trait Classifier: Send + Sync {
    fn classify<'a>(
        &'a self,
        input: ClassifyInput<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<ClassificationOutput>> + Send + 'a>>;
}

pub struct OllamaClassifier {
    pub settings: Settings,
}

impl Classifier for OllamaClassifier {
    fn classify<'a>(
        &'a self,
        input: ClassifyInput<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<ClassificationOutput>> + Send + 'a>> {
        Box::pin(async move {
            let b64 = input
                .base64_jpeg
                .ok_or_else(|| anyhow::anyhow!("missing base64 jpeg"))?;

            if self.settings.ollama_stream {
                let app = input.app.clone();
                let job_id = input.job_id.to_string();
                let file_name = input.file_name.to_string();
                let mut stream_text = String::new();

                let _ = app.emit(
                    STREAM_EVENT,
                    StreamChunk {
                        job_id: job_id.clone(),
                        file_name: file_name.clone(),
                        delta: String::new(),
                        done: false,
                        reset: true,
                    },
                );

            let (model_out, analysis_log) = classify_image_streaming_with_options(
                    &self.settings.ollama_base_url,
                    &self.settings.ollama_model,
                    self.settings.ollama_think,
                    b64,
                    input.cancel,
                    |delta| {
                        stream_text.push_str(delta);
                        let _ = app.emit(
                            STREAM_EVENT,
                            StreamChunk {
                                job_id: job_id.clone(),
                                file_name: file_name.clone(),
                                delta: delta.to_string(),
                                done: false,
                                reset: false,
                            },
                        );
                    },
                )
                .await?;

                let _ = app.emit(
                    STREAM_EVENT,
                    StreamChunk {
                        job_id,
                        file_name,
                        delta: String::new(),
                        done: true,
                        reset: false,
                    },
                );

                return Ok(ClassificationOutput {
                    model: self.settings.ollama_model.clone(),
                    scores: model_out.scores,
                    category: model_out.category,
                    tags: model_out.tags_ko,
                    caption: Some(model_out.caption_ko),
                    text_in_image: Some(model_out.text_in_image_ko),
                    analysis_log,
                    is_valuable: None,
                    valuable_score: None,
                });
            }

            let (model_out, analysis_log) = classify_image_with_options(
                &self.settings.ollama_base_url,
                &self.settings.ollama_model,
                self.settings.ollama_think,
                b64,
                input.cancel,
            )
            .await?;

            Ok(ClassificationOutput {
                model: self.settings.ollama_model.clone(),
                scores: model_out.scores,
                category: model_out.category,
                tags: model_out.tags_ko,
                caption: Some(model_out.caption_ko),
                text_in_image: Some(model_out.text_in_image_ko),
                analysis_log,
                is_valuable: None,
                valuable_score: None,
            })
        })
    }
}

pub struct ClipClassifier {
    pub opts: ClipEngineOptions,
}

static CLIP_ENGINE: Lazy<Mutex<Option<(String, Arc<ClipEngine>)>>> = Lazy::new(|| Mutex::new(None));

fn get_clip_engine(app: &AppHandle, opts: &ClipEngineOptions) -> Result<Arc<ClipEngine>> {
    let key = format!(
        "dir={:?};file={};pool={};intra={};value={};auto={};coreml={};cuda={};rocm={};directml={};openvino={}",
        opts.model_dir.as_deref().unwrap_or("<auto>"),
        opts.model_file,
        opts.session_pool_size,
        opts.intra_threads,
        opts.enable_value,
        opts.ep_auto,
        opts.ep_coreml,
        opts.ep_cuda,
        opts.ep_rocm,
        opts.ep_directml,
        opts.ep_openvino
    );
    let mut guard = CLIP_ENGINE.lock();
    if let Some((k, eng)) = guard.as_ref() {
        if k == &key {
            return Ok(Arc::clone(eng));
        }
    }
    let eng = Arc::new(ClipEngine::new(app, opts.clone())?);
    *guard = Some((key, Arc::clone(&eng)));
    Ok(eng)
}

fn derive_clip_threads(settings: &Settings) -> (usize, usize) {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .max(1);
    let requested = settings.analysis_concurrency.max(1) as usize;
    let pool = requested.min(cores).max(1);
    let intra = ((cores + pool - 1) / pool).max(1);
    (pool, intra)
}

pub fn warmup_clip_engine(app: &AppHandle, settings: &Settings) -> Result<()> {
    let (pool, intra) = derive_clip_threads(settings);
    let opts = ClipEngineOptions {
        model_dir: settings.clip_model_dir.clone(),
        model_file: settings.clip_model_file.clone(),
        session_pool_size: pool,
        intra_threads: intra,
        enable_value: settings.analysis_value_enabled,
        ep_auto: settings.clip_ep_auto,
        ep_coreml: settings.clip_ep_coreml,
        ep_cuda: settings.clip_ep_cuda,
        ep_rocm: settings.clip_ep_rocm,
        ep_directml: settings.clip_ep_directml,
        ep_openvino: settings.clip_ep_openvino,
        ..ClipEngineOptions::default()
    };
    let _ = get_clip_engine(app, &opts)?;
    Ok(())
}

impl Classifier for ClipClassifier {
    fn classify<'a>(
        &'a self,
        input: ClassifyInput<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<ClassificationOutput>> + Send + 'a>> {
        Box::pin(async move {
            let pre = preprocess_clip_image(input.path)?;
            let engine = get_clip_engine(input.app, &self.opts)?;
            let (scores, category, valuable, analysis_log, _infer_ms) = engine.classify(&pre.nchw)?;
            let (is_valuable, valuable_score) = valuable
                .map(|(b, p)| (Some(b), Some(p)))
                .unwrap_or((None, None));

            Ok(ClassificationOutput {
                model: "clip-vit-b32-onnx".to_string(),
                scores,
                category,
                tags: vec![category.dir_name_ko().to_string()],
                caption: Some("".to_string()),
                text_in_image: Some("".to_string()),
                analysis_log,
                is_valuable: if self.opts.enable_value { is_valuable } else { None },
                valuable_score: if self.opts.enable_value { valuable_score } else { None },
            })
        })
    }
}

pub fn build_classifier(settings: &Settings) -> (AnalysisEngine, Box<dyn Classifier>) {
    match settings.analysis_engine {
        AnalysisEngine::Ollama => (
            AnalysisEngine::Ollama,
            Box::new(OllamaClassifier {
                settings: settings.clone(),
            }),
        ),
        AnalysisEngine::Clip => (
            AnalysisEngine::Clip,
            Box::new(ClipClassifier {
                opts: {
                    let (pool, intra) = derive_clip_threads(settings);
                    ClipEngineOptions {
                        model_dir: settings.clip_model_dir.clone(),
                        model_file: settings.clip_model_file.clone(),
                        session_pool_size: pool,
                        intra_threads: intra,
                        enable_value: settings.analysis_value_enabled,
                        ep_auto: settings.clip_ep_auto,
                        ep_coreml: settings.clip_ep_coreml,
                        ep_cuda: settings.clip_ep_cuda,
                        ep_rocm: settings.clip_ep_rocm,
                        ep_directml: settings.clip_ep_directml,
                        ep_openvino: settings.clip_ep_openvino,
                        ..ClipEngineOptions::default()
                    }
                },
            }),
        ),
    }
}
