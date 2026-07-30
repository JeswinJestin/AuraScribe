-- src-tauri/migrations/002_encrypted_settings.sql
-- Encrypted custom settings table

CREATE TABLE IF NOT EXISTS settings_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT UNIQUE NOT NULL,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes for faster queries
CREATE INDEX IF NOT EXISTS idx_settings_keys_key ON settings_keys(key);
CREATE INDEX IF NOT EXISTS idx_settings_keys_created_at ON settings_keys(created_at);