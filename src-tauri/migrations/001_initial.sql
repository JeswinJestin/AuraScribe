-- 001_initial.sql
-- Core tables for AuraScribe (local-first dictation: no cloud keys, nothing to encrypt)

CREATE TABLE IF NOT EXISTS settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    hotkey TEXT NOT NULL DEFAULT 'Ctrl+Space',
    hotkey_mode TEXT NOT NULL DEFAULT 'toggle',
    whisper_model TEXT NOT NULL DEFAULT 'base.en',
    mic_device TEXT,
    ai_cleanup_enabled INTEGER NOT NULL DEFAULT 1,
    remove_fillers INTEGER NOT NULL DEFAULT 1,
    language TEXT NOT NULL DEFAULT 'en',
    theme TEXT NOT NULL DEFAULT 'dark',
    start_at_login INTEGER NOT NULL DEFAULT 0
);

INSERT OR IGNORE INTO settings (id) VALUES (1);

CREATE TABLE IF NOT EXISTS dictionary (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word TEXT NOT NULL,
    replacement TEXT NOT NULL,
    case_sensitive INTEGER NOT NULL DEFAULT 0,
    whole_word INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    UNIQUE(word, case_sensitive, whole_word)
);

CREATE INDEX IF NOT EXISTS idx_dictionary_word ON dictionary(word);

CREATE TABLE IF NOT EXISTS snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    trigger TEXT NOT NULL UNIQUE,
    expansion TEXT NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_snippets_trigger ON snippets(trigger);

CREATE TABLE IF NOT EXISTS app_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    app_name TEXT NOT NULL,
    app_identifier TEXT,
    style TEXT NOT NULL DEFAULT 'casual',
    ai_cleanup INTEGER NOT NULL DEFAULT 1,
    auto_punctuation INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    UNIQUE(app_name, app_identifier)
);

CREATE TABLE IF NOT EXISTS transcripts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    raw_text TEXT NOT NULL,
    cleaned_text TEXT,
    app_name TEXT,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    model_used TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_transcripts_timestamp ON transcripts(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_transcripts_app ON transcripts(app_name);
