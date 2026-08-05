-- 002_settings_rebuild.sql
--
-- Databases created by earlier AuraScribe builds carry a `settings` table with a
-- completely different shape (OpenAI/Ollama/OpenRouter credentials, push_to_talk_key),
-- plus `conversations`/`messages` tables left over from an unrelated chat prototype.
-- Because 001 uses CREATE TABLE IF NOT EXISTS, it silently no-ops against those, leaving
-- the app querying columns that don't exist.
--
-- Rebuild `settings` unconditionally so fresh and upgraded installs converge on one
-- schema. The dropped tables never held anything meaningful: dictation history lives in
-- `transcripts`, and the discarded credential columns belong to the cloud architecture
-- this version deliberately removed.

DROP TABLE IF EXISTS conversations;
DROP TABLE IF EXISTS messages;
DROP TABLE IF EXISTS settings;

CREATE TABLE settings (
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
