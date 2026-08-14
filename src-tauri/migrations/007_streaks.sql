-- 007_streaks.sql
-- Persisted state for the dictation streak + freeze economy (Insights page).
-- Singleton row (mirrors the `settings` pattern). Everything here is derived from local
-- `transcripts` history; this table only persists the path-dependent freeze economy and the
-- reconciliation cursor so gaps are counted exactly once. No new data is collected.

CREATE TABLE IF NOT EXISTS streak_state (
    id                  INTEGER PRIMARY KEY CHECK (id = 1),
    current_streak      INTEGER NOT NULL DEFAULT 0,   -- consecutive counted days, finalized up to last_reconciled_day
    longest_streak      INTEGER NOT NULL DEFAULT 0,   -- best ever, survives resets
    freezes             INTEGER NOT NULL DEFAULT 0,   -- banked streak freezes (0..=5)
    earn_progress       INTEGER NOT NULL DEFAULT 0,   -- counted days toward the next freeze (0..=9)
    last_reconciled_day INTEGER,                       -- last FINALIZED local day, as NaiveDate ordinal; NULL = never reconciled
    backfilled          INTEGER NOT NULL DEFAULT 0    -- 1 once history has been backfilled into current/longest
);

INSERT OR IGNORE INTO streak_state (id) VALUES (1);
