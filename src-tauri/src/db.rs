use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;
use tracing::info;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Result<Self, sqlx::Error> {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("AuraScribe");
        std::fs::create_dir_all(&data_dir).ok();

        let db_path = data_dir.join("aurascribe.db");
        let url = format!("sqlite:{}?mode=rwc", db_path.display());
        let options = SqliteConnectOptions::from_str(&url)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePool::connect_with(options).await?;

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;

        Self::run_migrations(&pool).await?;

        info!("Database initialized at {}", db_path.display());

        Ok(Self { pool })
    }

    async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            )",
        )
        .execute(pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                openai_api_key TEXT,
                openai_model TEXT,
                openai_base_url TEXT,
                openai_vo_model TEXT,
                openrouter_api_key TEXT,
                use_ollama INTEGER NOT NULL DEFAULT 0,
                ollama_base_url TEXT,
                ollama_model TEXT,
                language TEXT NOT NULL DEFAULT 'auto',
                push_to_talk_key TEXT,
                theme TEXT NOT NULL DEFAULT 'system'
            )",
        )
        .execute(pool)
        .await?;

        sqlx::query("INSERT OR IGNORE INTO settings (id) VALUES (1)")
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn save_settings(
        &self,
        openai_api_key: &Option<String>,
        openai_model: &Option<String>,
        openai_base_url: &Option<String>,
        openai_vo_model: &Option<String>,
        openrouter_api_key: &Option<String>,
        use_ollama: bool,
        ollama_base_url: &Option<String>,
        ollama_model: &Option<String>,
        language: &str,
        push_to_talk_key: &Option<String>,
        theme: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE settings SET
                openai_api_key = $1, openai_model = $2, openai_base_url = $3,
                openai_vo_model = $4, openrouter_api_key = $5, use_ollama = $6,
                ollama_base_url = $7, ollama_model = $8, language = $9,
                push_to_talk_key = $10, theme = $11
            WHERE id = 1",
        )
        .bind(openai_api_key)
        .bind(openai_model)
        .bind(openai_base_url)
        .bind(openai_vo_model)
        .bind(openrouter_api_key)
        .bind(use_ollama as i32)
        .bind(ollama_base_url)
        .bind(ollama_model)
        .bind(language)
        .bind(push_to_talk_key)
        .bind(theme)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn load_settings(
        &self,
    ) -> Result<SettingsRow, sqlx::Error> {
        let row = sqlx::query_as::<_, SettingsRow>("SELECT * FROM settings WHERE id = 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(row)
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn save_conversation(
        &self,
        id: &str,
        title: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT OR REPLACE INTO conversations (id, title, updated_at) VALUES ($1, $2, datetime('now'))",
        )
        .bind(id)
        .bind(title)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn save_message(
        &self,
        id: &str,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO messages (id, conversation_id, role, content) VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(conversation_id)
        .bind(role)
        .bind(content)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_conversations(
        &self,
    ) -> Result<Vec<ConversationRow>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ConversationRow>(
            "SELECT id, title, created_at, updated_at FROM conversations ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn load_conversation_messages(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MessageRow>, sqlx::Error> {
        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT id, conversation_id, role, content, created_at FROM messages WHERE conversation_id = $1 ORDER BY created_at ASC",
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn delete_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM conversations WHERE id = $1")
            .bind(conversation_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct SettingsRow {
    pub id: i32,
    pub openai_api_key: Option<String>,
    pub openai_model: Option<String>,
    pub openai_base_url: Option<String>,
    pub openai_vo_model: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub use_ollama: i32,
    pub ollama_base_url: Option<String>,
    pub ollama_model: Option<String>,
    pub language: String,
    pub push_to_talk_key: Option<String>,
    pub theme: String,
}

#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ConversationRow {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct MessageRow {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}
