#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod audio;
mod asr;
mod commands;
mod crypto;
mod db;
mod events;
mod injection;
mod llm;
mod models;
mod system;
mod vad;

use crate::app_state::AppState;
use crate::db::Database;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "aurascribe=debug,tauri=info".into()),
        )
        .init();

    let _app = tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .setup(|app| {
            let app_handle = app.handle().clone();

            let db = tauri::async_runtime::block_on(async {
                Database::new().await
            })?;
            let db = Arc::new(Mutex::new(db));

            let state = AppState { db, app_handle };
            app.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::save_settings,
            commands::load_settings,
            commands::start_dictation,
            commands::stop_dictation,
            commands::list_models,
            commands::download_model,
            commands::get_model_status,
            commands::open_model_directory,
            commands::get_log_file_path,
            commands::delete_conversation,
            commands::list_conversations,
            commands::load_conversation,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                tracing::info!("AuraScribe shutting down");
            }
        });
}
