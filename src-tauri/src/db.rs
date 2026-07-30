// src-tauri/src/db.rs
//! Database layer using SQLx with SQLite + SQLCipher encryption

use anyhow::{Context, Result};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool, Row};
use std::path::PathBuf;
use directories::ProjectDirs;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct Database {
    pool: SqlitePool,
    master_key: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Settings {
    pub key: String,
    pub value: String,
    pub encrypted: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DictionaryEntry {
    pub id: i64,
    pub word: String,
    pub replacement: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SnippetEntry {
    pub id: i64,
    pub trigger: String,
    pub expansion: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AppProfile {
    pub id: i64,
    pub app_name: String,
    pub app_identifier: Option<String>,
    pub style: String,
    pub custom_prompt: Option<String>,
    pub ai_cleanup: bool,
    pub auto_punctuation: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TranscriptEntry {
    pub id: i64,
    pub raw_text: String,
    pub cleaned_text: Option<String>,
    pub app_name: Option<String>,
    pub model_used: String,
    pub processing_time_ms: i64,
    pub created_at: i64,
}

impl Database {
    pub async fn new(master_key: String) -> Result<Self> {
        let data_dir = Self::get_data_dir()?;
        std::fs::create_dir_all(&data_dir).context("Failed to create data directory")?;

        let db_path = data_dir.join("aurascribe.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

        // Create pool with encryption
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(db_path)
                    .create_if_missing(true)
                    .pragma("key", format!("'{}'", master_key))
                    .pragma("cipher", "aes-256-cbc")
                    .pragma("kdf_iter", "256000")
                    .pragma("page_size", "4096")
            )
            .await
            .context("Failed to connect to database")?;

        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await.context("Migration failed")?;

        Ok(Self { pool, master_key })
    }

    fn get_data_dir() -> Result<PathBuf> {
        let proj = ProjectDirs::from("dev", "aurascribe", "AuraScribe")
            .context("Could not determine project directories")?;
        Ok(proj.data_dir().to_path_buf())
    }

    pub fn get_data_dir_path(&self) -> Result<PathBuf> {
        Self::get_data_dir()
    }

    // Settings
    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT value, encrypted FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| {
            let value: String = r.get("value");
            let encrypted: bool = r.get("encrypted");
            if encrypted {
                crate::crypto::decrypt(&value, &self.master_key).unwrap_or(value)
            } else {
                value
            }
        }))
    }

    pub async fn set_setting(&self, key: &str, value: &str, encrypted: bool) -> Result<()> {
        let stored_value = if encrypted {
            crate::crypto::encrypt(value, &self.master_key)?
        } else {
            value.to_string()
        };

        sqlx::query(
            "INSERT INTO settings (key, value, encrypted, updated_at) VALUES (?, ?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, encrypted = excluded.encrypted, updated_at = excluded.updated_at"
        )
        .bind(key)
        .bind(&stored_value)
        .bind(encrypted)
        .bind(chrono::Utc::now().timestamp())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_all_settings(&self) -> Result<Vec<Settings>> {
        sqlx::query_as::<_, Settings>("SELECT * FROM settings")
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    // Dictionary
    pub async fn get_dictionary(&self) -> Result<Vec<DictionaryEntry>> {
        sqlx::query_as::<_, DictionaryEntry>("SELECT * FROM dictionary ORDER BY word")
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn add_dictionary_entry(&self, entry: &DictionaryEntry) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO dictionary (word, replacement, case_sensitive, whole_word) VALUES (?, ?, ?, ?)"
        )
        .bind(&entry.word)
        .bind(&entry.replacement)
        .bind(entry.case_sensitive)
        .bind(entry.whole_word)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn update_dictionary_entry(&self, id: i64, entry: &DictionaryEntry) -> Result<()> {
        sqlx::query(
            "UPDATE dictionary SET word = ?, replacement = ?, case_sensitive = ?, whole_word = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&entry.word)
        .bind(&entry.replacement)
        .bind(entry.case_sensitive)
        .bind(entry.whole_word)
        .bind(chrono::Utc::now().timestamp())
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_dictionary_entry(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM dictionary WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Snippets
    pub async fn get_snippets(&self) -> Result<Vec<SnippetEntry>> {
        sqlx::query_as::<_, SnippetEntry>("SELECT * FROM snippets ORDER BY trigger")
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn add_snippet(&self, snippet: &SnippetEntry) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO snippets (trigger, expansion, description) VALUES (?, ?, ?)"
        )
        .bind(&snippet.trigger)
        .bind(&snippet.expansion)
        .bind(&snippet.description)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn update_snippet(&self, id: i64, snippet: &SnippetEntry) -> Result<()> {
        sqlx::query(
            "UPDATE snippets SET trigger = ?, expansion = ?, description = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&snippet.trigger)
        .bind(&snippet.expansion)
        .bind(&snippet.description)
        .bind(chrono::Utc::now().timestamp())
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_snippet(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM snippets WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // App Profiles
    pub async fn get_app_profiles(&self) -> Result<Vec<AppProfile>> {
        sqlx::query_as::<_, AppProfile>("SELECT * FROM app_profiles ORDER BY app_name")
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn add_app_profile(&self, profile: &AppProfile) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO app_profiles (app_name, app_identifier, style, custom_prompt, ai_cleanup, auto_punctuation) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&profile.app_name)
        .bind(&profile.app_identifier)
        .bind(&profile.style)
        .bind(&profile.custom_prompt)
        .bind(profile.ai_cleanup)
        .bind(profile.auto_punctuation)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn update_app_profile(&self, id: i64, profile: &AppProfile) -> Result<()> {
        sqlx::query(
            "UPDATE app_profiles SET app_name = ?, app_identifier = ?, style = ?, custom_prompt = ?, ai_cleanup = ?, auto_punctuation = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&profile.app_name)
        .bind(&profile.app_identifier)
        .bind(&profile.style)
        .bind(&profile.custom_prompt)
        .bind(profile.ai_cleanup)
        .bind(profile.auto_punctuation)
        .bind(chrono::Utc::now().timestamp())
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_app_profile(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM app_profiles WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Transcripts
    pub async fn save_transcript(
        &self,
        raw_text: &str,
        cleaned_text: Option<&str>,
        app_name: Option<&str>,
        model_used: &str,
        processing_time_ms: i64,
    ) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO transcripts (raw_text, cleaned_text, app_name, model_used, processing_time_ms) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(raw_text)
        .bind(cleaned_text)
        .bind(app_name)
        .bind(model_used)
        .bind(processing_time_ms)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn get_transcripts(&self, limit: i64, offset: i64) -> Result<Vec<TranscriptEntry>> {
        sqlx::query_as::<_, TranscriptEntry>(
            "SELECT * FROM transcripts ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn clear_transcripts(&self) -> Result<()> {
        sqlx::query("DELETE FROM transcripts").execute(&self.pool).await?;
        Ok(())
    }

    // Save/Load settings struct
    pub async fn save_settings(&self, settings: &crate::commands::Settings) -> Result<()> {
        self.set_setting("hotkey", &settings.hotkey, false).await?;
        self.set_setting("hotkey_mode", &settings.hotkey_mode, false).await?;
        self.set_setting("whisper_model", &settings.whisper_model, false).await?;
        self.set_setting("openrouter_key", &settings.openrouter_key, true).await?;
        self.set_setting("openrouter_model", &settings.openrouter_model, false).await?;
        self.set_setting("ai_cleanup_enabled", &settings.ai_cleanup_enabled.to_string(), false).await?;
        self.set_setting("auto_punctuation", &settings.auto_punctuation.to_string(), false).await?;
        self.set_setting("language", &settings.language, false).await?;
        self.set_setting("theme", &settings.theme, false).await?;
        self.set_setting("start_at_login", &settings.start_at_login.to_string(), false).await?;
        Ok(())
    }

    pub async fn load_settings(&self) -> Result<crate::commands::Settings> {
        let defaults = crate::commands::Settings::default();

        Ok(crate::commands::Settings {
            hotkey: self.get_setting("hotkey").await?.unwrap_or(defaults.hotkey),
            hotkey_mode: self.get_setting("hotkey_mode").await?.unwrap_or(defaults.hotkey_mode),
            whisper_model: self.get_setting("whisper_model").await?.unwrap_or(defaults.whisper_model),
            openrouter_key: self.get_setting("openrouter_key").await?.unwrap_or(defaults.openrouter_key),
            openrouter_model: self.get_setting("openrouter_model").await?.unwrap_or(defaults.openrouter_model),
            ai_cleanup_enabled: self.get_setting("ai_cleanup_enabled").await?.unwrap_or(defaults.ai_cleanup_enabled.to_string()).parse().unwrap_or(defaults.ai_cleanup_enabled),
            auto_punctuation: self.get_setting("auto_punctuation").await?.unwrap_or(defaults.auto_punctuation.to_string()).parse().unwrap_or(defaults.auto_punctuation),
            language: self.get_setting("language").await?.unwrap_or(defaults.language),
            theme: self.get_setting("theme").await?.unwrap_or(defaults.theme),
            start_at_login: self.get_setting("start_at_login").await?.unwrap_or(defaults.start_at_login.to_string()).parse().unwrap_or(defaults.start_at_login),
        })
    }
}