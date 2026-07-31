use crate::db::Database;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub app_handle: AppHandle,
}

impl AppState {
    pub fn app_handle(&self) -> &AppHandle {
        &self.app_handle
    }
}
