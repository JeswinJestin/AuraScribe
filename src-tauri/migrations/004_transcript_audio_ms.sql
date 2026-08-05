-- 004_transcript_audio_ms.sql
--
-- `duration_ms` records how long processing took, which says nothing about how much the
-- user actually spoke. Storing the length of the captured audio lets us report a real
-- words-per-minute speaking rate instead of guessing from processing time.

ALTER TABLE transcripts ADD COLUMN audio_ms INTEGER NOT NULL DEFAULT 0;
