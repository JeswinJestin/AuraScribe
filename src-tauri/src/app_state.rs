use std::sync::Arc;
use tokio::sync::Mutex;

use crate::commands::Status;
use crate::db::Database;
use crate::engine::Asr;

/// Transcripts of the pieces already processed while the user was still speaking, in
/// order, plus enough information to report how long they actually spoke.
#[derive(Default)]
pub struct ChunkState {
    pub texts: Vec<String>,
    pub raw_samples: usize,
    pub sample_rate: u32,
}

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub status: Arc<Mutex<Status>>,
    pub audio_buffer: Arc<Mutex<Vec<f32>>>,
    pub audio_sample_rate: Arc<Mutex<u32>>,
    pub recording_handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
    pub stop_flag: Arc<Mutex<bool>>,
    pub asr: Arc<Asr>,
    /// Results accumulated by the chunker during a recording.
    pub chunk_state: Arc<Mutex<ChunkState>>,
    /// The chunker itself. `stop_recording` awaits this; the task flushes the tail of the
    /// recording before exiting, so awaiting it is what guarantees nothing is lost.
    pub chunk_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}
