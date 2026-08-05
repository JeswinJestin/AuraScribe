-- 003_default_hotkey.sql
--
-- Ctrl+Space is widely claimed by Windows IME (input-language switching) and by editor
-- autocomplete, so registering it often silently does nothing. Move anyone still sitting
-- on the old default to Ctrl+Shift+Space, which the PRD suggested and which is far less
-- contended. A user who deliberately picked their own combo is left alone.

UPDATE settings SET hotkey = 'Ctrl+Shift+Space' WHERE hotkey = 'Ctrl+Space';
