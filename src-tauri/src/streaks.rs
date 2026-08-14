//! The dictation streak + freeze economy for the Insights page.
//!
//! This module is PURE integer logic (no I/O), so the path-dependent freeze rules can be
//! unit-tested exhaustively. Days are represented as `NaiveDate` ordinals (see `db.rs`, which does
//! the SQLite `localtime` grouping and the ordinal conversion). The DB glue lives in `db.rs`; the
//! command in `commands.rs`. Design: docs/superpowers/specs/2026-08-13-insights-streaks-recap-design.md
//!
//! Rules (owner-specified):
//! - A day COUNTS when at least `MIN_WORDS_PER_DAY` words are dictated in it.
//! - Every `DAYS_PER_FREEZE` consecutive counted days earns one freeze, up to `MAX_FREEZES` slots.
//! - A missed day auto-spends one freeze (one per missed day); with none left, the streak resets.
//! - `longest_streak` is a personal best that survives resets.
//! - First launch backfills current/longest from real history; freezes start at 0 (only accrue forward).

use std::collections::BTreeSet;

/// A day "counts" toward the streak once this many words are dictated in it (local calendar day).
pub const MIN_WORDS_PER_DAY: i64 = 25;
/// Consecutive counted days needed to earn one freeze.
pub const DAYS_PER_FREEZE: i64 = 10;
/// Maximum banked freezes.
pub const MAX_FREEZES: i64 = 5;

/// The persisted, FINALIZED streak state — everything up to and including `last_reconciled_day`.
/// Today is never finalized here; the live "today" bonus is applied only when building `StreakInfo`,
/// so today can never break the streak until it is actually over.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StreakState {
    pub current_streak: i64,
    pub longest_streak: i64,
    pub freezes: i64,
    pub earn_progress: i64,
    /// `NaiveDate` ordinal of the last finalized day. `None` = never reconciled.
    pub last_reconciled_day: Option<i64>,
    pub backfilled: bool,
}

impl StreakState {
    /// Bring the finalized state up to date given the set of counted-day ordinals and today's
    /// ordinal. Only whole days strictly before `today` are finalized, so calling this repeatedly
    /// within the same day is a no-op after the first — it is safe to reconcile on every read.
    pub fn reconcile(&self, counted: &BTreeSet<i64>, today: i64) -> StreakState {
        if !self.backfilled {
            return backfill(counted, today);
        }
        let yesterday = today - 1;
        let start = match self.last_reconciled_day {
            Some(d) => d + 1,
            // Backfilled but no cursor should not happen; finalize nothing rather than guess.
            None => yesterday + 1,
        };
        let mut st = self.clone();
        let mut day = start;
        while day <= yesterday {
            step_day(&mut st, counted.contains(&day));
            day += 1;
        }
        // Advance the cursor even if the loop did nothing, so it tracks the calendar.
        if st.last_reconciled_day.map_or(true, |d| d < yesterday) {
            st.last_reconciled_day = Some(yesterday);
        }
        st
    }

    /// Build the view-model the UI reads, applying the live "today" bonus on top of the finalized
    /// state. `today_counted` is whether today has already crossed `MIN_WORDS_PER_DAY`.
    pub fn to_info(&self, today_counted: bool, words_today: i64) -> StreakInfo {
        let streak = self.current_streak + i64::from(today_counted);
        let longest = self.longest_streak.max(streak);
        let days_to_next_freeze = if self.freezes >= MAX_FREEZES {
            0
        } else {
            (DAYS_PER_FREEZE - self.earn_progress).max(0)
        };
        StreakInfo {
            streak,
            longest,
            freezes: self.freezes,
            max_freezes: MAX_FREEZES,
            days_to_next_freeze,
            today_counted,
            words_today,
            min_words_per_day: MIN_WORDS_PER_DAY,
        }
    }
}

/// Apply one finalized day to the running state.
fn step_day(st: &mut StreakState, counted: bool) {
    if counted {
        st.current_streak += 1;
        if st.current_streak > st.longest_streak {
            st.longest_streak = st.current_streak;
        }
        // Earn toward the next freeze, unless the bank is full (then progress simply pauses,
        // preserving whatever value it held — it is never lost).
        if st.freezes < MAX_FREEZES {
            st.earn_progress += 1;
            if st.earn_progress >= DAYS_PER_FREEZE {
                st.freezes += 1;
                st.earn_progress = 0;
            }
        }
    } else if st.freezes > 0 {
        // A banked freeze absorbs the missed day; the streak survives untouched.
        st.freezes -= 1;
    } else {
        // No protection left: the streak breaks. longest_streak is kept.
        st.current_streak = 0;
        st.earn_progress = 0;
    }
}

/// First-ever run: compute the streak from real history WITHOUT simulating a past freeze economy
/// (freezes start at 0 and only accrue forward — see the spec). Finalizes up to `today - 1`.
fn backfill(counted: &BTreeSet<i64>, today: i64) -> StreakState {
    let yesterday = today - 1;

    // Current streak = consecutive counted days ending exactly at yesterday. Today is live and
    // added by `to_info`, so it is intentionally excluded from the finalized count here.
    let mut current = 0i64;
    let mut d = yesterday;
    while counted.contains(&d) {
        current += 1;
        d -= 1;
    }

    // Longest ever = the longest consecutive run anywhere in history (today included; it is real).
    let mut longest = 0i64;
    let mut run = 0i64;
    let mut prev: Option<i64> = None;
    for &day in counted.iter() {
        run = if prev == Some(day - 1) { run + 1 } else { 1 };
        if run > longest {
            longest = run;
        }
        prev = Some(day);
    }

    StreakState {
        current_streak: current,
        longest_streak: longest.max(current),
        freezes: 0,
        earn_progress: current % DAYS_PER_FREEZE,
        last_reconciled_day: Some(yesterday),
        backfilled: true,
    }
}

/// View-model returned to the frontend: the finalized state plus the live "today" bonus.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct StreakInfo {
    /// Current streak, including today if it already counts.
    pub streak: i64,
    pub longest: i64,
    pub freezes: i64,
    pub max_freezes: i64,
    /// Counted days still needed to bank the next freeze; 0 when the bank is full.
    pub days_to_next_freeze: i64,
    pub today_counted: bool,
    pub words_today: i64,
    pub min_words_per_day: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn days(range: std::ops::RangeInclusive<i64>) -> BTreeSet<i64> {
        range.collect()
    }

    // ---- backfill ----

    #[test]
    fn backfill_empty_history() {
        let st = StreakState::default().reconcile(&BTreeSet::new(), 100);
        assert_eq!(st.current_streak, 0);
        assert_eq!(st.longest_streak, 0);
        assert_eq!(st.freezes, 0);
        assert!(st.backfilled);
        assert_eq!(st.last_reconciled_day, Some(99));
    }

    #[test]
    fn backfill_run_ending_yesterday_counts_as_current() {
        // Counted days 91..=99, today = 100. Current run ending at yesterday(99) is 9.
        let st = StreakState::default().reconcile(&days(91..=99), 100);
        assert_eq!(st.current_streak, 9);
        assert_eq!(st.longest_streak, 9);
        assert_eq!(st.earn_progress, 9); // 9 % 10
    }

    #[test]
    fn backfill_gap_yesterday_breaks_current_but_longest_remembers() {
        // A 5-day run long ago, then nothing near today. today=100, yesterday=99 not counted.
        let st = StreakState::default().reconcile(&days(10..=14), 100);
        assert_eq!(st.current_streak, 0);
        assert_eq!(st.longest_streak, 5);
    }

    #[test]
    fn backfill_only_today_counted_leaves_finalized_zero() {
        // today=100 counted, yesterday not. Finalized current is 0; to_info adds the live +1.
        let mut set = BTreeSet::new();
        set.insert(100);
        let st = StreakState::default().reconcile(&set, 100);
        assert_eq!(st.current_streak, 0);
        assert_eq!(st.to_info(true, 30).streak, 1);
    }

    // ---- incremental reconcile ----

    fn backfilled_at(day: i64, current: i64, longest: i64, freezes: i64, earn: i64) -> StreakState {
        StreakState {
            current_streak: current,
            longest_streak: longest,
            freezes,
            earn_progress: earn,
            last_reconciled_day: Some(day),
            backfilled: true,
        }
    }

    #[test]
    fn reconcile_is_idempotent_within_a_day() {
        let st = backfilled_at(99, 5, 5, 0, 5);
        // today still 100 => yesterday 99 already finalized. No change.
        let out = st.reconcile(&days(50..=99), 100);
        assert_eq!(out, st);
    }

    #[test]
    fn earns_a_freeze_after_ten_counted_days() {
        // Start fresh-ish: finalized at day 99 with earn_progress 9. Day 100 counted, today=101.
        let st = backfilled_at(99, 9, 9, 0, 9);
        let out = st.reconcile(&days(1..=100), 101);
        assert_eq!(out.freezes, 1);
        assert_eq!(out.earn_progress, 0);
        assert_eq!(out.current_streak, 10);
        assert_eq!(out.last_reconciled_day, Some(100));
    }

    #[test]
    fn freezes_cap_at_five() {
        // 60 counted days straight from scratch would earn 6; must cap at 5.
        let st = backfilled_at(0, 0, 0, 0, 0);
        let out = st.reconcile(&days(1..=60), 61);
        assert_eq!(out.freezes, MAX_FREEZES);
        assert_eq!(out.current_streak, 60);
        // After the 5th freeze at day 50, earn pauses: days 51..=60 don't advance it.
        assert_eq!(out.earn_progress, 0);
    }

    #[test]
    fn single_missed_day_spends_one_freeze_and_keeps_streak() {
        // Finalized streak 12 with 2 freezes at day 99. Day 100 missed, today=101.
        let st = backfilled_at(99, 12, 12, 2, 2);
        let mut counted = days(1..=99); // 100 is NOT in the set => missed
        counted.remove(&100);
        let out = st.reconcile(&counted, 101);
        assert_eq!(out.freezes, 1);
        assert_eq!(out.current_streak, 12); // preserved
    }

    #[test]
    fn multi_day_gap_spends_multiple_freezes() {
        // Missed days 100,101,102 with 3 freezes banked -> all three spent, streak preserved.
        let st = backfilled_at(99, 20, 20, 3, 4);
        let counted = days(1..=99); // 100..=102 missing
        let out = st.reconcile(&counted, 103); // yesterday = 102
        assert_eq!(out.freezes, 0);
        assert_eq!(out.current_streak, 20);
    }

    #[test]
    fn gap_longer_than_freezes_resets_streak_but_keeps_longest() {
        // 1 freeze, but 3 missed days -> freeze covers one, next breaks it.
        let st = backfilled_at(99, 20, 25, 1, 4);
        let counted = days(1..=99); // 100..=102 missing, today=103
        let out = st.reconcile(&counted, 103);
        assert_eq!(out.freezes, 0);
        assert_eq!(out.current_streak, 0);
        assert_eq!(out.earn_progress, 0);
        assert_eq!(out.longest_streak, 25); // remembered
    }

    #[test]
    fn resume_after_reset_rebuilds_streak() {
        // After a reset, counted days resume and rebuild.
        let st = backfilled_at(99, 0, 30, 0, 0);
        let out = st.reconcile(&days(100..=104), 105); // days 100..=104 counted, yesterday 104
        assert_eq!(out.current_streak, 5);
        assert_eq!(out.longest_streak, 30);
    }

    // ---- to_info ----

    #[test]
    fn to_info_applies_today_bonus_and_days_to_next() {
        let st = backfilled_at(99, 7, 7, 0, 7);
        let info = st.to_info(true, 42);
        assert_eq!(info.streak, 8); // 7 + today
        assert_eq!(info.longest, 8); // max(7, 8)
        assert_eq!(info.days_to_next_freeze, 3); // 10 - 7
        assert!(info.today_counted);
        assert_eq!(info.words_today, 42);
    }

    #[test]
    fn to_info_full_bank_reports_zero_to_next() {
        let st = backfilled_at(99, 50, 50, MAX_FREEZES, 0);
        let info = st.to_info(false, 10);
        assert_eq!(info.days_to_next_freeze, 0);
        assert_eq!(info.streak, 50);
    }
}
