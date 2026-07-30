-- 001_initial.sql
-- Core tables for AuraScribe

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    encrypted INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

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
    custom_prompt TEXT,
    ai_cleanup INTEGER NOT NULL DEFAULT 0,
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

-- Default settings
INSERT OR IGNORE INTO settings (key, value, encrypted) VALUES
    ('hotkey', 'Ctrl+Space', 0),
    ('hotkey_mode', 'press-hold', 0),
    ('whisper_model', 'base.en', 0),
    ('openrouter_key', '', 1),
    ('openrouter_model', 'nvidia/nemotron-3-ultra', 0),
    ('ai_cleanup_enabled', 'false', 0),
    ('auto_punctuation', 'true', 0),
    ('language', 'en', 0),
    ('theme', 'system', 0),
    ('start_at_login', 'false', 0);