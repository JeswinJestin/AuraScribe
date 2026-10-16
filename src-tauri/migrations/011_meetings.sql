-- Notetaker (local meeting transcription). See docs/superpowers/specs/2026-10-13-notetaker-design.md.
-- Stores meeting transcripts produced entirely on-device. No audio is persisted (text is cheap;
-- raw-audio retention is a deliberate later opt-in). No cloud, no summary model — MVP is a faithful
-- local transcript only.

CREATE TABLE IF NOT EXISTS meetings (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    title       TEXT    NOT NULL DEFAULT 'Untitled meeting',
    started_at  INTEGER NOT NULL,                       -- unix seconds
    ended_at    INTEGER,                                -- null while still recording
    duration_ms INTEGER NOT NULL DEFAULT 0,
    word_count  INTEGER NOT NULL DEFAULT 0,
    source      TEXT    NOT NULL DEFAULT 'system+mic'   -- what was captured (system+mic / mic / system)
);

CREATE TABLE IF NOT EXISTS meeting_segments (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id  INTEGER NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    offset_ms   INTEGER NOT NULL,                       -- ms from meeting start
    text        TEXT    NOT NULL,
    created_at  INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_meeting_segments_meeting ON meeting_segments(meeting_id, offset_ms);
