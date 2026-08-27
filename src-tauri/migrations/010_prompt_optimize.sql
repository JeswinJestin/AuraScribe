-- Optional on-device prompt optimization (optimize.rs / the `prompt` feature). A second global
-- hotkey rewrites the current selection into a better prompt, in place. OFF by default; the hotkey
-- is stored empty until set (the code resolves a per-OS default). Existing installs get 0 / ''.
ALTER TABLE settings ADD COLUMN prompt_optimize_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN prompt_optimize_hotkey TEXT NOT NULL DEFAULT '';
