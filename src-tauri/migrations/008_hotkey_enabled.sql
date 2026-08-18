-- Lets the user disable the global dictation hotkey from Settings without uninstalling
-- ("sleep" the app). On by default; existing installs get 1 (enabled) via the default.
ALTER TABLE settings ADD COLUMN hotkey_enabled INTEGER NOT NULL DEFAULT 1;
