# AuraScribe v1.1.0 — Insights streaks

Everything that made v1.0.0 the stable release is unchanged — still free, still open source, still
**100% offline** (the only network request in the whole app remains the one-time model download).
This release adds a habit layer to the Insights page.

## What's new

- **Daily streak.** A day counts once you dictate 25+ words. Your current streak and your longest-ever
  streak show on the Insights page, and a small flame in the sidebar keeps it glanceable.
- **Streak freezes.** Every 10 days in a row banks a freeze (up to 5). If you miss a day, a freeze is
  spent automatically to keep the streak alive; with none left, it resets. Your longest streak is kept
  regardless.
- **Milestones.** Quiet badges light up at 7 / 30 / 100 / 365-day streaks, at 10k / 100k / 1M words
  dictated, and at 10 / 100 hours saved.

Everything is computed from your existing local history — no new data is collected, nothing is uploaded
or shared, and the strict offline policy is unchanged.

## Under the hood

- New streak engine with a full unit-test suite for the freeze rules (earn, cap, spend, reset,
  backfill). On first launch your streak is computed from your real dictation history.
- New database migration for the streak state (applied automatically on first launch).

## Fixes

- **Migration robustness.** Pinned the database migration files to LF line endings (`.gitattributes`)
  so a rebuilt app can never reject its own database with a spurious "migration modified" error. This
  did not affect any released build — fresh installs were always safe — but it hardens future upgrades.

## Install

Download `AuraScribe_1.1.0_x64-setup.exe` below and run it. Windows 10/11, ~9 MB, no account, no
subscription. A first-time run downloads your chosen voice model once; everything after that is offline.
