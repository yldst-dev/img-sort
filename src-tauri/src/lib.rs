mod core;

use crate::core::commands::*;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle();
            let state = AppState::new(&handle)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            test_ollama,
            list_ollama_models,
            get_clip_model_files,
            get_clip_accel_capabilities,
            start_analysis,
            cancel_analysis,
            list_photos,
            get_photo_detail,
            get_distribution,
            get_progress,
            get_value_stats,
            clear_results
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
