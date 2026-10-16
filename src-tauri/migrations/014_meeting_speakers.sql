-- Notetaker speaker intelligence. See
-- docs/superpowers/specs/2026-10-16-notetaker-speaker-intelligence-design.md.
--
-- Each meeting learns who is speaking: "You" (your mic) vs guests (the loopback audio), split
-- further into Speaker 1/2/3…, and relabelled to real names once the conversation makes them
-- clear. A segment stores its speaker *cluster* (not a frozen label) so renaming or merging a
-- speaker updates every one of their lines at once. All on-device; no audio is persisted.

CREATE TABLE IF NOT EXISTS meeting_speakers (
    meeting_id  INTEGER NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    cluster     INTEGER NOT NULL,            -- 0 = You (self/mic); 1..N = guests
    label       TEXT    NOT NULL,            -- "You" / "Speaker 1" / a resolved or edited name
    is_self     INTEGER NOT NULL DEFAULT 0,
    is_named    INTEGER NOT NULL DEFAULT 0,  -- 1 once a real name is resolved or the user edits it
    PRIMARY KEY (meeting_id, cluster)
);

-- Attribute each segment to a speaker cluster. -1 keeps every existing row valid (renders as an
-- unknown speaker) until a meeting is captured with speaker intelligence on.
ALTER TABLE meeting_segments ADD COLUMN speaker_cluster INTEGER NOT NULL DEFAULT -1;
