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
use crate::commands::Status;
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

            let db = tauri::async_runtime::block_on(async { Database::new().await })?;
            let db = Arc::new(Mutex::new(db));

            let state = AppState {
                db,
                app_handle,
                status: Arc::new(Mutex::new(Status::default())),
                audio_buffer: Arc::new(Mutex::new(Vec::new())),
                recording_handle: Arc::new(Mutex::new(None)),
                stop_flag: Arc::new(Mutex::new(false)),
            };
            app.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_status,
            commands::start_recording,
            commands::stop_recording,
            commands::load_model,
            commands::download_model,
            commands::get_downloaded_models,
            commands::delete_model,
            commands::cleanup_with_ai,
            commands::get_dictionary,
            commands::add_dictionary_entry,
            commands::update_dictionary_entry,
            commands::delete_dictionary_entry,
            commands::get_snippets,
            commands::add_snippet,
            commands::update_snippet,
            commands::delete_snippet,
            commands::get_app_profiles,
            commands::add_app_profile,
            commands::update_app_profile,
            commands::delete_app_profile,
            commands::get_transcripts,
            commands::clear_transcripts,
            commands::set_start_at_login,
            commands::open_settings_folder,
            commands::check_microphone_permission,
            commands::request_microphone_permission,
            commands::check_accessibility_permission,
            commands::request_accessibility_permission,
            commands::list_conversations,
            commands::delete_conversation,
            commands::load_conversation,
            commands::get_log_file_path,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                tracing::info!("AuraScribe shutting down");
            }
        });
}
