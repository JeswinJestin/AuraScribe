use std::sync::Arc;
use tokio::sync::Mutex;

use crate::asr::WhisperASR;
use crate::commands::Status;
use crate::db::Database;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub status: Arc<Mutex<Status>>,
    pub audio_buffer: Arc<Mutex<Vec<f32>>>,
    pub audio_sample_rate: Arc<Mutex<u32>>,
    pub recording_handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
    pub stop_flag: Arc<Mutex<bool>>,
    pub asr: Arc<WhisperASR>,
}
