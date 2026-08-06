-- Audible start/stop cue for dictation. On by default; a user in a shared space can turn it
-- off. Existing installs get 1 (on) via the default.
ALTER TABLE settings ADD COLUMN sound_cues INTEGER NOT NULL DEFAULT 1;
