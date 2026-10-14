-- Per-meeting free-text notes ("My notes" / "My thoughts"), so a meeting can be maintained like a
-- real document alongside its transcript. Local only, like everything else.
ALTER TABLE meetings ADD COLUMN notes TEXT NOT NULL DEFAULT '';
