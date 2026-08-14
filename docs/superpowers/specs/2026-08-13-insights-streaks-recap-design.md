# Insights upgrade — streaks, milestones, yearly recap

**Date:** 2026-08-13 · **Status:** approved design, ready to implement
**Scope:** Project A of a three-part roadmap (A: this · C: prompt-optimization engine · B: macOS/Linux).
Built in two shippable stages.

## Goal

Turn the existing [Insights page](../../../src/components/views/InsightsView.tsx) from a static stat
grid into a lightweight habit surface: a daily **streak** with a Duolingo-style **freeze** economy,
**milestones**, and a Spotify-Wrapped-style **yearly recap** with a locally-rendered shareable image.

Everything is computed from data already on disk (`transcripts` rows: `timestamp`, `audio_ms`, text).
**No new data is collected, no network call is added, the CSP stays as-is.** "Share" means AuraScribe
draws a PNG the user saves and posts themselves — nothing is uploaded.

## Non-negotiables carried in

- Never a cloud call; stay lightweight; never claim more than the code does.
- Streak/recap numbers must be **real** — verified against actual history, not asserted.
- Honour the "instrument" design system: colour = state, monospace for machine values, minimal motion,
  `prefers-reduced-motion` respected. The streak is a **quiet readout**, not a bold game.

---

## Stage 1 — Streak engine + display + milestones

### The streak rule (owner-specified)

- A **day counts** when the user dictates **≥ 25 words total that day** (summed across all dictations,
  grouped by **local** calendar day). Fewer than 25 words → the day does not count.
- **Today is shown live** and never breaks the streak until it is over; it becomes "safe" the moment
  the 25-word bar is crossed.
- **Streak freezes** are earned: every **10 consecutive counted days** earns **1 freeze**, up to
  **5 slots**.
- **A missed day auto-spends one freeze** (one freeze per missed day) so the streak survives. With no
  freeze left, the streak **resets to 0** (and `earn_progress` resets to 0).
- Freezes **cap at 5**. When all slots are full the earn counter pauses (progress is not lost); after a
  freeze is spent, a fresh run of 10 counted days earns the next.
- A **longest-ever streak** is recorded and survives resets.

### Backfill (first launch of this feature)

Chosen: **count existing history.** `current_streak` and `longest_streak` are computed from real past
dictations (consecutive counted-day runs). `freezes` start at **0** and `earn_progress` at
`current_streak mod 10` — the freeze economy only *runs forward* from launch (simulating a past
economy would be guesswork). This matches what the owner approved.

### Reconciliation (single source of truth)

Runs on **app start** and **when Insights opens** (events are never trusted alone — CLAUDE.md rule).
Algorithm, walking each *finished* day from `last_reconciled_day + 1` up to **yesterday**:

```
for day in (last_reconciled_day+1 ..= yesterday):
    if counted(day):                      # ≥25 words that local day
        current += 1
        earn_progress += 1
        if earn_progress == 10 and freezes < 5:
            freezes += 1; earn_progress = 0
        longest = max(longest, current)
    else:                                 # missed day
        if freezes > 0: freezes -= 1      # streak survives, earn_progress unchanged
        else: current = 0; earn_progress = 0
persist(current, longest, freezes, earn_progress, last_reconciled_day = yesterday)
```

Display then adds today live: if today is counted, show `current + 1` and today's slot as safe.
Edge cases covered by tests: earn at exactly 10, cap at 5, multi-day gap spending multiple freezes,
gap longer than banked freezes → reset, empty history, all-timezone grouping via `localtime`.

### Persistence

New migration **`007_streaks.sql`** — a singleton table (mirrors the `settings` pattern):

```sql
CREATE TABLE IF NOT EXISTS streak_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    current_streak      INTEGER NOT NULL DEFAULT 0,
    longest_streak      INTEGER NOT NULL DEFAULT 0,
    freezes             INTEGER NOT NULL DEFAULT 0,
    earn_progress       INTEGER NOT NULL DEFAULT 0,
    last_reconciled_day INTEGER,              -- epoch-day (unix / 86400) of last finalized day, NULL = never
    backfilled          INTEGER NOT NULL DEFAULT 0
);
INSERT OR IGNORE INTO streak_state (id) VALUES (1);
```

Never edit an applied migration (sqlx checksum) — this is a new file.

### Backend

- New `streaks.rs`: `StreakState` struct + `reconcile()` (pure function over a list of counted epoch-days
  → new state, fully unit-testable) + a thin DB wrapper.
- New DB query `counted_days()` → the set of local epoch-days with ≥25 words. Word count is
  `split_whitespace().count()` over `COALESCE(cleaned_text, raw_text)`, aggregated per local day in Rust
  (fetch `(local_day, text)`, sum). Reuses the `date(timestamp,'unixepoch','localtime')` grouping the
  heatmap already uses.
- New command `get_streak_state()` → returns current/longest/freezes/earn_progress + today-safe flag,
  wired through `src/lib/ipc.ts` (all IPC goes through there).

### Frontend (Stage 1)

- **Insights page:** a streak readout above the existing stat grid — current streak with a flame glyph,
  longest-ever, and 5 freeze slots as filled/empty pips. Numbers in monospace `tabular-nums`.
- **Sidebar status rail:** a small `🔥 N` next to the live state, glanceable without opening Insights.
  No animation beyond what the design system allows; hidden when streak is 0.
- **Milestones:** streak **7 / 30 / 100 / 365 days**; lifetime **words** 10k / 100k / 1M; **time saved**
  10h / 100h. Crossing one shows a **subtle** one-time mark on Insights (no confetti). Earned milestones
  render as quiet badges. Which have been seen is tracked (a small `milestones_seen` set — settings row
  or a tiny table) so the mark shows once.

---

## Stage 2 — Yearly recap + shareable cards

### "Your Year" recap

A panel computed over the calendar year (rolling to `now`): total words, **hours saved** (vs 40 wpm
typing, the existing formula), hours spoken, dictations, active days, longest streak, busiest day, and
top app (from `transcripts.app_name`). All from local data.

**Seasonal prominence (owner-specified):** the recap is **accessible year-round** (a link/section on the
Insights page), but from **December 1 through January 31** it surfaces on its own as a **prominent
sidebar entry** ("Your Year"), then recedes. Implemented as a date check that adds the sidebar nav item
during that window; outside it, the recap remains reachable from Insights. No data or behaviour changes
by date — only placement/prominence.

### Shareable card (local PNG, no upload)

AuraScribe renders a card (streak or recap) onto an offscreen `<canvas>` — **no new dependency** — then
`toBlob('image/png')` and a Tauri save dialog writes the file to disk. Nothing leaves the machine; the
restrictive CSP is untouched so any regression is loud. Card shows the headline number(s) + AuraScribe
wordmark, in the instrument palette.

---

## Verification

- **`cargo test`** on `reconcile()` — the path-dependent freeze economy is exactly what needs tests
  (earn/cap/spend/reset/backfill/empty). This is the "verify by running, not reading" discipline.
- A real build + a live read of `get_streak_state()` against the owner's actual history, confirming the
  streak number matches reality (not a hardcoded value).
- `tsc --noEmit` clean; Insights + sidebar checked in a preview.
- Update `docs/HANDOFF.md` when done.

## Explicitly out of scope (own specs later)

- **Project C — prompt-optimization engine.** Decided direction: an **optional, separately-downloaded**
  small/fast local instruct model with a persona/context/task/format system prompt; the app stays
  4.6 MB and the feature only works if the model is downloaded. Needs its own research + spec (model
  choice, speed, prompt strategy).
- **Project B — macOS/Linux ports.**
- Any cloud sync, leaderboards, or social features (violates local-first; never).
