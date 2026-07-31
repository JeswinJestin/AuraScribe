use crate::app_state::AppState;
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub openai_api_key: Option<String>,
    pub openai_model: Option<String>,
    pub openai_base_url: Option<String>,
    pub openai_vo_model: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub use_ollama: bool,
    pub ollama_base_url: Option<String>,
    pub ollama_model: Option<String>,
    pub language: String,
    pub push_to_talk_key: Option<String>,
    pub theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            openai_api_key: None,
            openai_model: Some("whisper-large-v3".to_string()),
            openai_base_url: None,
            openai_vo_model: Some("gpt-4o".to_string()),
            openrouter_api_key: None,
            use_ollama: false,
            ollama_base_url: Some("http://localhost:11434".to_string()),
            ollama_model: Some("llama3".to_string()),
            language: "auto".to_string(),
            push_to_talk_key: None,
            theme: "system".to_string(),
        }
    }
}

#[command]
pub async fn save_settings(state: tauri::State<'_, AppState>, settings: Settings) -> Result<(), String> {
    let db = state.db.lock().await;
    db.save_settings(
        &settings.openai_api_key,
        &settings.openai_model,
        &settings.openai_base_url,
        &settings.openai_vo_model,
        &settings.openrouter_api_key,
        settings.use_ollama,
        &settings.ollama_base_url,
        &settings.ollama_model,
        &settings.language,
        &settings.push_to_talk_key,
        &settings.theme,
    )
    .await
    .map_err(|e| e.to_string())
}

#[command]
pub async fn load_settings(state: tauri::State<'_, AppState>) -> Result<Settings, String> {
    let db = state.db.lock().await;
    let row = db.load_settings().await.map_err(|e| e.to_string())?;
    Ok(Settings {
        openai_api_key: row.openai_api_key,
        openai_model: row.openai_model,
        openai_base_url: row.openai_base_url,
        openai_vo_model: row.openai_vo_model,
        openrouter_api_key: row.openrouter_api_key,
        use_ollama: row.use_ollama != 0,
        ollama_base_url: row.ollama_base_url,
        ollama_model: row.ollama_model,
        language: row.language,
        push_to_talk_key: row.push_to_talk_key,
        theme: row.theme,
    })
}

#[command]
pub async fn start_dictation() -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn stop_dictation() -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn list_models() -> Result<Vec<String>, String> {
    Ok(vec![])
}

#[command]
pub async fn download_model(_model: String) -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn get_model_status() -> Result<String, String> {
    Ok("ready".to_string())
}

#[command]
pub async fn open_model_directory() -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn get_log_file_path() -> Result<String, String> {
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("AuraScribe");
    Ok(data_dir.join("aurascribe.log").to_string_lossy().to_string())
}

#[command]
pub async fn delete_conversation(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.delete_conversation(&conversation_id)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn list_conversations(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<crate::db::ConversationRow>, String> {
    let db = state.db.lock().await;
    db.list_conversations().await.map_err(|e| e.to_string())
}

#[command]
pub async fn load_conversation(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<crate::db::MessageRow>, String> {
    let db = state.db.lock().await;
    db.load_conversation_messages(&conversation_id)
        .await
        .map_err(|e| e.to_string())
}
