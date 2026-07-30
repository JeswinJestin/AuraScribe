//! AuraScribe - Main entry point
//!
//! Free, open-source, privacy-first voice dictation for everyone.

mod audio;
mod asr;
mod commands;
mod crypto;
mod db;
mod events;
mod injection;
mod llm;
pub mod models;
mod system;
mod vad;

use std::sync::Arc;
use tauri::{Manager, Runtime};
use tauri_plugin_sql::{Migration, MigrationKind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = vec![
        Migration {
            version: 1,
            description: "create_initial_tables",
            sql: include_str!("../migrations/001_initial.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 2,
            description: "add_encrypted_settings",
            sql: include_str!("../migrations/002_encrypted_settings.sql"),
            kind: MigrationKind::Up,
        },
    ];

    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::new()
            .add_migrations("sqlite:aurascribe.db", migrations)
            .build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_path::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .setup(|app| {
            // Initialize system tray
            system::init_tray(app.handle().clone())?;

            // Initialize global hotkey
            system::init_global_shortcut(app.handle().clone())?;

            // Initialize audio system
            audio::init(app.handle().clone())?;

            // Initialize ASR
            asr::init(app.handle().clone())?;

            // Initialize text injection
            injection::init(app.handle().clone())?;

            // Initialize LLM cleanup
            llm::init(app.handle().clone())?;

            // Load settings
            commands::load_settings(app.handle().clone())?;

            // Set up window
            if let Some(window) = app.get_webview_window("main") {
                window.hide()?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Settings
            commands::get_settings,
            commands::save_settings,
            commands::load_settings,
            // Audio/Recording
            commands::start_recording,
            commands::stop_recording,
            commands::get_status,
            // Model management
            commands::download_model,
            commands::load_model,
            commands::list_models,
            commands::delete_model,
            // Dictionary
            commands::get_dictionary,
            commands::add_dictionary_entry,
            commands::update_dictionary_entry,
            commands::delete_dictionary_entry,
            // Snippets
            commands::get_snippets,
            commands::add_snippet,
            commands::update_snippet,
            commands::delete_snippet,
            // App profiles
            commands::get_app_profiles,
            commands::add_app_profile,
            commands::update_app_profile,
            commands::delete_app_profile,
            // Transcripts
            commands::get_transcripts,
            commands::clear_transcripts,
            // AI Cleanup
            commands::cleanup_with_ai,
            // System
            commands::set_start_at_login,
            commands::open_settings_folder,
            commands::check_microphone_permission,
            commands::request_microphone_permission,
            commands::check_accessibility_permission,
            commands::request_accessibility_permission,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}