use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
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
        // Whether we are creating the database this launch. Captured *before* connecting, since
        // `create_if_missing` is about to bring the file into existence. A fresh install is the
        // only time we impose the new defaults (Glass appearance, show onboarding); returning
        // users keep whatever they had.
        let is_fresh = !db_path.exists();
        let url = format!("sqlite:{}?mode=rwc", db_path.display());
        let options = SqliteConnectOptions::from_str(&url)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        // Migrations run on their own short-lived connection which is then closed.
        // A migration that drops and recreates a table (see 002) leaves any connection
        // opened beforehand holding a stale schema, which surfaces on the first read as
        // "no column found for name: hotkey". Opening the app pool only after the schema
        // is final avoids that entirely.
        {
            let migrator_pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(options.clone())
                .await?;
            sqlx::migrate!("./migrations").run(&migrator_pool).await?;
            migrator_pool.close().await;
        }

        let pool = SqlitePool::connect_with(options).await?;

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;

        // Brand-new install: adopt the current defaults that the migrations can't express
        // (migration 002's row was created with the old `dark`/already-onboarded defaults, and
        // it must stay editable for upgrades). Glass is the intended first-impression look, and
        // onboarding should run once. Returning users are never touched by this.
        if is_fresh {
            // The settings row's hotkey is seeded by migration SQL as `Ctrl+Shift+Space` on every
            // platform, so a fresh install needs the platform-appropriate default applied here
            // (macOS → Cmd+Shift+Space). Onboarding reads this value, so this is also what makes
            // the walkthrough show the right keys per device. Existing users are never touched.
            sqlx::query("UPDATE settings SET theme = 'glass', onboarded = 0, hotkey = ? WHERE id = 1")
                .bind(crate::commands::default_hotkey())
                .execute(&pool)
                .await?;
            info!(
                "Fresh install: Glass appearance, onboarding on, hotkey = {}",
                crate::commands::default_hotkey()
            );
        }

        info!("Database initialized at {}", db_path.display());

        Ok(Self { pool })
    }

    // ---- Settings ----

    pub async fn save_settings(&self, s: &SettingsRow) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE settings SET
                hotkey = $1, hotkey_mode = $2, whisper_model = $3, mic_device = $4,
                ai_cleanup_enabled = $5, remove_fillers = $6, language = $7,
                theme = $8, start_at_login = $9, sound_cues = $10, onboarded = $11,
                hotkey_enabled = $12
            WHERE id = 1",
        )
        .bind(&s.hotkey)
        .bind(&s.hotkey_mode)
        .bind(&s.whisper_model)
        .bind(&s.mic_device)
        .bind(s.ai_cleanup_enabled)
        .bind(s.remove_fillers)
        .bind(&s.language)
        .bind(&s.theme)
        .bind(s.start_at_login)
        .bind(s.sound_cues)
        .bind(s.onboarded)
        .bind(s.hotkey_enabled)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn load_settings(&self) -> Result<SettingsRow, sqlx::Error> {
        sqlx::query_as::<_, SettingsRow>("SELECT * FROM settings WHERE id = 1")
            .fetch_one(&self.pool)
            .await
    }

    // ---- Dictionary ----

    pub async fn list_dictionary(&self) -> Result<Vec<DictionaryRow>, sqlx::Error> {
        sqlx::query_as::<_, DictionaryRow>(
            "SELECT id, word, replacement, case_sensitive, whole_word, created_at FROM dictionary ORDER BY word",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn add_dictionary_entry(
        &self,
        word: &str,
        replacement: &str,
        case_sensitive: bool,
        whole_word: bool,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO dictionary (word, replacement, case_sensitive, whole_word) VALUES ($1, $2, $3, $4)",
        )
        .bind(word)
        .bind(replacement)
        .bind(case_sensitive as i32)
        .bind(whole_word as i32)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn delete_dictionary_entry(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM dictionary WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Snippets ----

    pub async fn list_snippets(&self) -> Result<Vec<SnippetRow>, sqlx::Error> {
        sqlx::query_as::<_, SnippetRow>(
            "SELECT id, trigger, expansion, description, created_at FROM snippets ORDER BY trigger",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn add_snippet(
        &self,
        trigger: &str,
        expansion: &str,
        description: &Option<String>,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO snippets (trigger, expansion, description) VALUES ($1, $2, $3)",
        )
        .bind(trigger)
        .bind(expansion)
        .bind(description)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn delete_snippet(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM snippets WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- App profiles ----

    pub async fn list_app_profiles(&self) -> Result<Vec<AppProfileRow>, sqlx::Error> {
        sqlx::query_as::<_, AppProfileRow>(
            "SELECT id, app_name, app_identifier, style, ai_cleanup, auto_punctuation, created_at FROM app_profiles ORDER BY app_name",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn add_app_profile(
        &self,
        app_name: &str,
        app_identifier: &Option<String>,
        style: &str,
        ai_cleanup: bool,
        auto_punctuation: bool,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO app_profiles (app_name, app_identifier, style, ai_cleanup, auto_punctuation) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(app_name)
        .bind(app_identifier)
        .bind(style)
        .bind(ai_cleanup as i32)
        .bind(auto_punctuation as i32)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn delete_app_profile(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM app_profiles WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Transcripts ----

    #[allow(clippy::too_many_arguments)]
    pub async fn add_transcript(
        &self,
        raw_text: &str,
        cleaned_text: &str,
        app_name: &Option<String>,
        duration_ms: i64,
        audio_ms: i64,
        model_used: &str,
    ) -> Result<i64, sqlx::Error> {
        let timestamp = chrono::Utc::now().timestamp();
        let result = sqlx::query(
            "INSERT INTO transcripts (timestamp, raw_text, cleaned_text, app_name, duration_ms, audio_ms, model_used) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(timestamp)
        .bind(raw_text)
        .bind(cleaned_text)
        .bind(app_name)
        .bind(duration_ms)
        .bind(audio_ms)
        .bind(model_used)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    /// Aggregate usage figures for the Insights view.
    pub async fn stats(&self) -> Result<UsageStats, sqlx::Error> {
        let rows = sqlx::query_as::<_, (String, i64, i64)>(
            "SELECT COALESCE(cleaned_text, raw_text), audio_ms, timestamp FROM transcripts",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut total_words = 0i64;
        let mut total_audio_ms = 0i64;
        let today_start = chrono::Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|d| d.and_utc().timestamp())
            .unwrap_or(0);
        let mut words_today = 0i64;

        for (text, audio_ms, ts) in &rows {
            let words = text.split_whitespace().count() as i64;
            total_words += words;
            total_audio_ms += audio_ms;
            if *ts >= today_start {
                words_today += words;
            }
        }

        // Only count sessions that actually recorded a duration, so early rows written
        // before audio_ms existed don't drag the rate toward zero.
        let timed_words: i64 = rows
            .iter()
            .filter(|(_, audio_ms, _)| *audio_ms > 0)
            .map(|(t, _, _)| t.split_whitespace().count() as i64)
            .sum();
        let words_per_minute = if total_audio_ms > 0 {
            (timed_words as f64 / (total_audio_ms as f64 / 60_000.0)).round() as i64
        } else {
            0
        };

        // Distinct days with at least one dictation, most recent first.
        let mut days: Vec<i64> = rows.iter().map(|(_, _, ts)| ts / 86_400).collect();
        days.sort_unstable();
        days.dedup();

        Ok(UsageStats {
            total_dictations: rows.len() as i64,
            total_words,
            words_today,
            words_per_minute,
            total_audio_ms,
            active_days: days.len() as i64,
        })
    }

    pub async fn list_transcripts(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TranscriptRow>, sqlx::Error> {
        sqlx::query_as::<_, TranscriptRow>(
            "SELECT id, timestamp, raw_text, cleaned_text, app_name, duration_ms, audio_ms, model_used, created_at
             FROM transcripts ORDER BY timestamp DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Search transcripts matching `query` in either cleaned or raw text, newest-first.
    pub async fn search_transcripts(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TranscriptRow>, sqlx::Error> {
        let pattern = format!("%{}%", query);
        sqlx::query_as::<_, TranscriptRow>(
            "SELECT id, timestamp, raw_text, cleaned_text, app_name, duration_ms, audio_ms, model_used, created_at
             FROM transcripts
             WHERE cleaned_text LIKE $1 OR raw_text LIKE $1
             ORDER BY timestamp DESC LIMIT $2 OFFSET $3",
        )
        .bind(pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Delete a single transcript by its primary key.
    pub async fn delete_transcript(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM transcripts WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn clear_transcripts(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM transcripts").execute(&self.pool).await?;
        Ok(())
    }

    /// Dictation count per **local** calendar day since `since_unix`, for the usage heatmap.
    /// Grouping uses `localtime` so a day matches what the user sees in their timezone.
    pub async fn daily_counts(&self, since_unix: i64) -> Result<Vec<DailyCount>, sqlx::Error> {
        sqlx::query_as::<_, DailyCount>(
            "SELECT date(timestamp, 'unixepoch', 'localtime') AS day, COUNT(*) AS count
             FROM transcripts WHERE timestamp >= $1
             GROUP BY day ORDER BY day",
        )
        .bind(since_unix)
        .fetch_all(&self.pool)
        .await
    }

    /// Delete transcripts whose timestamp is within `[start_unix, end_unix]` (inclusive), for the
    /// "delete a date range" control. Returns the number of rows removed.
    pub async fn delete_transcripts_between(
        &self,
        start_unix: i64,
        end_unix: i64,
    ) -> Result<u64, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM transcripts WHERE timestamp >= $1 AND timestamp <= $2")
                .bind(start_unix)
                .bind(end_unix)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected())
    }

    // ---- Streaks (Insights) ----

    /// Per-local-day word data for the streak engine: the set of days (as `NaiveDate` ordinals)
    /// that reached `min_words`, today's ordinal, and today's running word count. Word counts use
    /// the same whitespace split as `stats()` for consistency. Today's date is computed by SQLite
    /// so its day boundary matches the `localtime` grouping exactly.
    pub async fn streak_day_data(&self, min_words: i64) -> Result<StreakDayData, sqlx::Error> {
        use chrono::Datelike;
        use std::collections::HashMap;

        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT date(timestamp, 'unixepoch', 'localtime') AS day,
                    COALESCE(cleaned_text, raw_text) AS text
             FROM transcripts",
        )
        .fetch_all(&self.pool)
        .await?;

        let today_str: String = sqlx::query_scalar("SELECT date('now', 'localtime')")
            .fetch_one(&self.pool)
            .await?;

        let mut per_day: HashMap<String, i64> = HashMap::new();
        for (day, text) in &rows {
            *per_day.entry(day.clone()).or_insert(0) += text.split_whitespace().count() as i64;
        }

        let to_ord = |s: &str| -> Option<i64> {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .ok()
                .map(|d| d.num_days_from_ce() as i64)
        };

        let counted: Vec<i64> = per_day
            .iter()
            .filter(|(_, &w)| w >= min_words)
            .filter_map(|(day, _)| to_ord(day))
            .collect();

        Ok(StreakDayData {
            counted,
            today_ordinal: to_ord(&today_str).unwrap_or(0),
            words_today: per_day.get(&today_str).copied().unwrap_or(0),
        })
    }

    pub async fn load_streak_state(&self) -> Result<StreakStateRow, sqlx::Error> {
        sqlx::query_as::<_, StreakStateRow>("SELECT * FROM streak_state WHERE id = 1")
            .fetch_one(&self.pool)
            .await
    }

    pub async fn save_streak_state(&self, s: &StreakStateRow) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE streak_state SET
                current_streak = $1, longest_streak = $2, freezes = $3,
                earn_progress = $4, last_reconciled_day = $5, backfilled = $6
             WHERE id = 1",
        )
        .bind(s.current_streak)
        .bind(s.longest_streak)
        .bind(s.freezes)
        .bind(s.earn_progress)
        .bind(s.last_reconciled_day)
        .bind(s.backfilled)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Aggregate stats for the "Your Year" recap, scoped to a local calendar year. Everything is
    /// computed from existing transcript history — nothing new is collected, nothing leaves the box.
    pub async fn year_recap(&self, year: i32) -> Result<YearRecap, sqlx::Error> {
        use std::collections::HashMap;

        let rows = sqlx::query_as::<_, (String, Option<String>, i64, String)>(
            "SELECT date(timestamp, 'unixepoch', 'localtime') AS day,
                    app_name,
                    audio_ms,
                    COALESCE(cleaned_text, raw_text) AS text
             FROM transcripts
             WHERE strftime('%Y', timestamp, 'unixepoch', 'localtime') = $1",
        )
        .bind(format!("{year:04}"))
        .fetch_all(&self.pool)
        .await?;

        let mut total_words = 0i64;
        let mut total_audio_ms = 0i64;
        let mut per_day: HashMap<String, i64> = HashMap::new();
        let mut per_app: HashMap<String, i64> = HashMap::new();
        for (day, app, audio_ms, text) in &rows {
            let w = text.split_whitespace().count() as i64;
            total_words += w;
            total_audio_ms += audio_ms;
            *per_day.entry(day.clone()).or_insert(0) += w;
            if let Some(a) = app.as_deref().filter(|a| !a.is_empty()) {
                *per_app.entry(a.to_string()).or_insert(0) += 1;
            }
        }

        let (busiest_day, busiest_day_words) = per_day
            .iter()
            .max_by_key(|(_, &w)| w)
            .map(|(d, &w)| (Some(d.clone()), w))
            .unwrap_or((None, 0));
        let (top_app, top_app_dictations) = per_app
            .iter()
            .max_by_key(|(_, &c)| c)
            .map(|(a, &c)| (Some(a.clone()), c))
            .unwrap_or((None, 0));

        let spoken_minutes = total_audio_ms as f64 / 60_000.0;
        let typed_minutes = total_words as f64 / 40.0; // 40 wpm typing baseline
        let hours_saved = (typed_minutes - spoken_minutes).max(0.0) / 60.0;
        let words_per_minute = if total_audio_ms > 0 {
            (total_words as f64 / spoken_minutes.max(f64::MIN_POSITIVE)).round() as i64
        } else {
            0
        };

        Ok(YearRecap {
            year,
            total_words,
            total_dictations: rows.len() as i64,
            active_days: per_day.len() as i64,
            hours_spoken: spoken_minutes / 60.0,
            hours_saved,
            words_per_minute,
            busiest_day,
            busiest_day_words,
            top_app,
            top_app_dictations,
        })
    }
}

/// The "Your Year" recap (Insights, Stage 2). All fields derive from local transcript history.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct YearRecap {
    pub year: i32,
    pub total_words: i64,
    pub total_dictations: i64,
    pub active_days: i64,
    pub hours_spoken: f64,
    pub hours_saved: f64,
    pub words_per_minute: i64,
    /// Local `YYYY-MM-DD` of the day with the most words, if any.
    pub busiest_day: Option<String>,
    pub busiest_day_words: i64,
    pub top_app: Option<String>,
    pub top_app_dictations: i64,
}

/// Raw per-day word data used to drive the streak engine (`streaks.rs`).
pub struct StreakDayData {
    /// Ordinals (`NaiveDate::num_days_from_ce`) of local days that reached the word threshold.
    pub counted: Vec<i64>,
    pub today_ordinal: i64,
    pub words_today: i64,
}

/// The persisted singleton streak row (see migration 007). `backfilled`/booleans are stored as
/// integers, matching the rest of this schema.
#[derive(sqlx::FromRow, Clone, Debug)]
pub struct StreakStateRow {
    // `id` (always 1) is intentionally not mapped — sqlx `FromRow` ignores unselected columns, and
    // we only ever write `WHERE id = 1`. Omitting it keeps the struct warning-clean.
    pub current_streak: i64,
    pub longest_streak: i64,
    pub freezes: i64,
    pub earn_progress: i64,
    pub last_reconciled_day: Option<i64>,
    pub backfilled: i64,
}

/// One row of the usage heatmap: a local calendar day and how many dictations landed on it.
#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct DailyCount {
    /// Local date as `YYYY-MM-DD`.
    pub day: String,
    pub count: i64,
}

#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct SettingsRow {
    pub id: i32,
    pub hotkey: String,
    pub hotkey_mode: String,
    pub whisper_model: String,
    pub mic_device: Option<String>,
    pub ai_cleanup_enabled: i32,
    pub remove_fillers: i32,
    pub language: String,
    pub theme: String,
    pub start_at_login: i32,
    pub sound_cues: i32,
    pub onboarded: i32,
    pub hotkey_enabled: i32,
}

#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct DictionaryRow {
    pub id: i64,
    pub word: String,
    pub replacement: String,
    pub case_sensitive: i32,
    pub whole_word: i32,
    pub created_at: i64,
}

#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct SnippetRow {
    pub id: i64,
    pub trigger: String,
    pub expansion: String,
    pub description: Option<String>,
    pub created_at: i64,
}

#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AppProfileRow {
    pub id: i64,
    pub app_name: String,
    pub app_identifier: Option<String>,
    pub style: String,
    pub ai_cleanup: i32,
    pub auto_punctuation: i32,
    pub created_at: i64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct UsageStats {
    pub total_dictations: i64,
    pub total_words: i64,
    pub words_today: i64,
    pub words_per_minute: i64,
    pub total_audio_ms: i64,
    pub active_days: i64,
}

#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct TranscriptRow {
    pub id: i64,
    pub timestamp: i64,
    pub raw_text: String,
    pub cleaned_text: Option<String>,
    pub app_name: Option<String>,
    pub duration_ms: i64,
    pub audio_ms: i64,
    pub model_used: Option<String>,
    pub created_at: i64,
}
