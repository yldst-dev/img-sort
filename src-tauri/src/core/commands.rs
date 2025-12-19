use crate::core::classifier::warmup_clip_engine;
use crate::core::clip::ClipEngine;
use crate::core::config::{load_settings, save_settings};
use crate::core::db::Db;
use crate::core::model::{
    AnalysisEngine, ClipAccelCapabilities, ClipProviderCapability, Distribution, DistributionMode,
    Progress, Settings, StartAnalysisInput, StartAnalysisResult, ValueStats, CATEGORY_KEYS,
};
use crate::core::ollama;
use crate::core::pipeline::{test_ollama_connection, Pipeline};
use anyhow::Result;
use ort::execution_providers::{
    CPUExecutionProvider, CUDAExecutionProvider, CoreMLExecutionProvider,
    DirectMLExecutionProvider, ExecutionProvider, OpenVINOExecutionProvider, ROCmExecutionProvider,
};
use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, State};

pub struct AppState {
    pub db: Arc<Mutex<Db>>,
    pub pipeline: Mutex<Pipeline>,
    pub settings: Mutex<Settings>,
}

impl AppState {
    pub fn new(app: &AppHandle) -> Result<Self> {
        let settings = load_settings(app);
        if settings.analysis_engine == crate::core::model::AnalysisEngine::Clip {
            if let Err(e) = warmup_clip_engine(app, &settings) {
                eprintln!("clip warmup failed: {}", e);
            }
        }
        let db = Db::init(app)?;
        Ok(AppState {
            db: Arc::new(Mutex::new(db)),
            pipeline: Mutex::new(Pipeline::new()),
            settings: Mutex::new(settings),
        })
    }
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    Ok(state.settings.lock().clone())
}

#[tauri::command]
pub async fn set_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    mut settings: Settings,
) -> Result<(), String> {
    let max = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
        .max(1);
    settings.analysis_concurrency = settings.analysis_concurrency.clamp(1, max);
    if settings.analysis_concurrency > 1 {
        settings.ollama_stream = false;
    }
    {
        let mut guard = state.settings.lock();
        *guard = settings.clone();
    }
    save_settings(&app, &settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_ollama(base_url: String) -> Result<String, String> {
    test_ollama_connection(&base_url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_ollama_models(base_url: String) -> Result<Vec<String>, String> {
    ollama::list_models(&base_url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_clip_model_files(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let settings = state.settings.lock().clone();
    let dir = ClipEngine::resolve_model_dir(&app, settings.clip_model_dir.as_deref())
        .map_err(|e| e.to_string())?;
    let onnx_dir = dir.join("onnx");
    let mut out: Vec<String> = Vec::new();
    let rd = std::fs::read_dir(&onnx_dir).map_err(|e| e.to_string())?;
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("onnx") {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            out.push(format!("onnx/{}", name));
        }
    }
    out.sort();
    Ok(out)
}

#[tauri::command]
pub async fn start_analysis(
    app: AppHandle,
    state: State<'_, AppState>,
    input: StartAnalysisInput,
) -> Result<StartAnalysisResult, String> {
    let settings = state.settings.lock().clone();
    let mut pipeline = state.pipeline.lock();
    let job_id = pipeline
        .start(app, state.db.clone(), settings, input)
        .map_err(|e| e.to_string())?;
    Ok(StartAnalysisResult { job_id })
}

#[tauri::command]
pub async fn cancel_analysis(state: State<'_, AppState>, job_id: String) -> Result<(), String> {
    state
        .pipeline
        .lock()
        .cancel(&job_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_photos(
    state: State<'_, AppState>,
) -> Result<Vec<crate::core::model::PhotoRow>, String> {
    state.db.lock().list_photos().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_photo_detail(
    state: State<'_, AppState>,
    id: String,
) -> Result<crate::core::model::PhotoDetail, String> {
    state
        .db
        .lock()
        .get_photo_detail(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_distribution(
    state: State<'_, AppState>,
    mode: DistributionMode,
) -> Result<Distribution, String> {
    if let Some(meta) = state.pipeline.lock().last_job_meta() {
        if meta.engine == AnalysisEngine::Clip {
            if let Ok(dist) = get_folder_distribution(&meta.export_root, mode.clone()) {
                return Ok(dist);
            }
        }
    }
    state
        .db
        .lock()
        .get_distribution(mode)
        .map_err(|e| e.to_string())
}

fn get_folder_distribution(export_root: &str, mode: DistributionMode) -> Result<Distribution> {
    let export_root = std::path::Path::new(export_root);
    let mut counts: std::collections::HashMap<String, f32> = CATEGORY_KEYS
        .iter()
        .map(|c| (c.as_str().to_string(), 0.0f32))
        .collect();

    fn count_files(dir: &std::path::Path) -> f32 {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return 0.0;
        };
        let mut n: f32 = 0.0;
        for entry in rd.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_file() {
                    n += 1.0;
                }
            }
        }
        n
    }

    let mut total: f32 = 0.0;
    for k in CATEGORY_KEYS {
        let leaf = k.dir_name_ko();
        // Support both layouts:
        // 1) export_root/<카테고리>/
        // 2) export_root/가치있음/<카테고리>/ and export_root/가치없음/<카테고리>/
        let n = count_files(&export_root.join(leaf))
            + count_files(&export_root.join("가치있음").join(leaf))
            + count_files(&export_root.join("가치없음").join(leaf));
        total += n;
        if let Some(v) = counts.get_mut(k.as_str()) {
            *v = n;
        }
    }

    if total <= 0.0 {
        return Ok(Distribution {
            mode,
            by_category: counts,
        });
    }

    for v in counts.values_mut() {
        *v = (*v / total * 10000.0).round() / 10000.0;
    }

    Ok(Distribution {
        // For CLIP we return folder-count distribution for both modes, so the radar works reliably.
        mode,
        by_category: counts,
    })
}

#[tauri::command]
pub async fn get_progress(state: State<'_, AppState>) -> Result<Option<Progress>, String> {
    Ok(state.pipeline.lock().current_progress())
}

#[tauri::command]
pub async fn get_value_stats(state: State<'_, AppState>) -> Result<ValueStats, String> {
    state
        .db
        .lock()
        .get_value_stats()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_results(state: State<'_, AppState>) -> Result<(), String> {
    state.db.lock().clear_photos().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_clip_accel_capabilities() -> Result<ClipAccelCapabilities, String> {
    fn cap(name: &str, ep: &impl ExecutionProvider) -> ClipProviderCapability {
        let supported = ep.supported_by_platform();
        let available = if supported {
            ep.is_available().unwrap_or(false)
        } else {
            false
        };
        ClipProviderCapability {
            supported,
            available,
            name: name.to_string(),
        }
    }

    Ok(ClipAccelCapabilities {
        cpu: cap("CPU", &CPUExecutionProvider::default()),
        coreml: cap("CoreML (Apple)", &CoreMLExecutionProvider::default()),
        cuda: cap("CUDA (NVIDIA)", &CUDAExecutionProvider::default()),
        rocm: cap("ROCm (AMD)", &ROCmExecutionProvider::default()),
        directml: cap("DirectML (Windows)", &DirectMLExecutionProvider::default()),
        openvino: cap("OpenVINO (Intel)", &OpenVINOExecutionProvider::default()),
    })
}
