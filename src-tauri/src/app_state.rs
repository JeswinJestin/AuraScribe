use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::audio::AudioCapture;
use crate::commands::{Settings, Status};
use crate::db::Database;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub app_handle: AppHandle,
    pub status: Arc<Mutex<Status>>,
    pub audio_buffer: Arc<Mutex<Vec<f32>>>,
    pub recording_handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
    pub stop_flag: Arc<Mutex<bool>>,
}

impl AppState {
    pub fn app_handle(&self) -> &AppHandle {
        &self.app_handle
    }
}
