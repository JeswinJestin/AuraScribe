-- Opt-in fuzzy (near-miss) correction toward the user's own dictionary terms. Off by default so
-- normal dictation is unchanged; see src-tauri/src/expand.rs::fuzzy_correct.
ALTER TABLE settings ADD COLUMN fuzzy_dictionary INTEGER NOT NULL DEFAULT 0;
