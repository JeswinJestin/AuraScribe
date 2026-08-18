# AuraScribe — Project Journal

> **What this is:** the running story of the project — the path we've taken, every meaningful
> experiment (the ones that worked *and* the ones that didn't), the decisions and why we made them.
> `docs/HANDOFF.md` answers *"what is true right now?"*; this file answers *"how did we get here, and
> what have we already tried?"* so we never re-run a dead end or lose the thread.
>
> **Maintenance rule (for Claude and humans):** after every task that makes a **major** change —
> a feature shipped, an experiment run (success or failure), a release, an architecture decision, a
> reverted change, a significant bug root-caused — **append a dated entry to the "Timeline" below.**
> Keep entries honest: record what failed and why, not just wins. This is part of finishing the work,
> like updating HANDOFF. Small/cosmetic changes don't need an entry; use judgement.

---

## Current status (snapshot — keep short; full detail in HANDOFF)

- **Shipped:** v1.1.0 (streaks) — live, stable, Windows. v1.0.0 remains the prior stable release.
- **Working well:** local dictation for English (Moonshine), 25 European langs (Parakeet), ~40 Asian
  langs (Dolphin). Streaks/insights. 100% offline.
- **Known weak spot:** Malayalam/Kannada (IndicConformer NeMo-CTC) — accurate only on short, clearly
  spoken, pure-Malayalam input. Degrades on longer utterances and cannot handle English (code-mixing).
- **Next big things:** C = local prompt-optimization engine (optional model download); a **better
  multilingual Indic model** (the real fix for Malayalam robustness + code-mixing); B = macOS/Linux.

---

## Timeline

### Before 2026-08-14 (condensed — see HANDOFF "Round" history for detail)

- **Genesis & the honesty reset.** The first version *looked* complete but was almost entirely fake:
  Whisper had never compiled, CRUD returned hardcoded data. Rebuilt for real; adopted the rule
  **"verify by running, not by reading."**
- **Speech engines, layered on a `sherpa-onnx` seam.** Whisper (fallback) → Moonshine (fast English)
  → Parakeet (25 European) → Dolphin (~40 Asian) → NeMo-CTC IndicConformer (Malayalam/Kannada). Each
  routes through the `engine.rs` `Asr` facade.
- **Malayalam breakthrough (Round 26–27).** No one had published a sherpa-onnx IndicConformer. We
  packaged the community CTC ONNX export by appending the metadata sherpa needs; it loaded and decoded.
  Owner verified a full Malayalam paragraph transcribed cleanly **in the app** (which chunks input).
- **v1.0.0 (Round 34).** First stable release. Bundled the sherpa/ONNX/MSVC DLLs into the installer
  (Round 33 fixed "VCRUNTIME140 not found" on clean PCs). README/SEO/sponsors. Started the
  open-source contribution: Malayalam model on HuggingFace + sherpa-onnx discussion #3199.
- **Landing page** built as a sibling project (`../aurascribe-landing`), editorial cream/dark design.

### 2026-08-14 — the big session

**Roadmap set.** Post-v1 feature order agreed with the owner: **A = Insights streaks → C =
prompt-optimization engine → B = macOS/Linux.** (Order later reaffirmed: do A first, then C.)

**A — Insights streaks (Stage 1): SHIPPED.**
- Designed the streak + freeze economy with the owner: a day counts at ≥25 words; 1 freeze per 10
  consecutive days (cap 5); a miss auto-spends a freeze else resets; longest-ever kept; first launch
  backfills from real history.
- Built `streaks.rs` (pure engine, 13 unit tests), migration `007_streaks.sql`, `get_streak_state`
  command + IPC, Insights streak card + milestones, sidebar flame.
- **Verified against the owner's real DB:** a genuine 10-day streak. All 61 Rust tests pass. Spec at
  `docs/superpowers/specs/2026-08-13-insights-streaks-recap-design.md`. Stage 2 (yearly recap +
  shareable cards) designed but not built.

**Open-source contribution — the sherpa docs PR.**
- Discovered the docs live in `k2-fsa/sherpa` (not `sherpa-onnx`); the old guide pointed at the wrong
  repo. Wrote a correct `malayalam.rst`. Owner opened PR #847 (and a stray #848, folded in).
- CodeRabbit nits fixed. Then the maintainer **csukuangfj (fangjun)** asked for the *full* real
  terminal output via `literalinclude`. That request kicked off the investigation below.

**v1.0.0 stability scare → root-caused and fixed.**
- After a local rebuild, the app **crashed on every launch**: `migration 6 was previously applied but
  has been modified`. Root cause: `006_onboarding.sql` had drifted to **CRLF** while every other
  migration + the DB's stored checksum is **LF**; sqlx checksums the bytes, so the rebuilt binary
  rejected its own DB. **The released v1.0.0 was never affected** (fresh installs store their own
  checksums; the drift was local). Fixed by normalizing `006` to LF and adding `.gitattributes`
  pinning `migrations/*.sql` to LF so it can never recur. Verified by running (log reached
  `Database initialized`).

**v1.1.0 released.** Streaks + the LF fix. **Packaging catch:** the first build used `npm run build`
(base config) and produced a **4.9 MB installer missing the bundled DLLs** — would fail on a clean
PC. Rebuilt correctly with the moonshine config → **8.6 MB, all 7 DLLs bundled**, verified. Released,
marked Latest; v1.0.0 preserved.

**Landing page SEO/favicon.** Diagnosed the live site (`www.aurascribe.dev`): `/favicon.ico` 404'd
(only an app-router `icon.png` existed). Added a real root `favicon.ico`, `alternateName` brand schema
(to disambiguate from "Aura AI Scribe"), and a full `docs/SEO-LAUNCH-CHECKLIST.md` (GSC + Bing steps).

**Malayalam deep investigation — the important, humbling one.**
- fangjun's request for real decode output exposed that the model's **one-shot** output is wrong.
- Ran the real model (Python `sherpa_onnx`) on the test wav on the owner's machine, repeatedly:
  - The model **degrades on longer inputs.** Decisive evidence: the *identical* word "നമസ്കാരം"
    decodes **cleanly at 1.5 s but garbled at 3.2 s** — only the total length changed. Cause: NeMo
    **per-feature normalization** is computed over the whole window.
  - It is **monolingual** — English words ("Hi", "demo", "test", "run") are dropped/mangled. This is
    the "it stops transcribing when I say an English word" bug the owner hit in the app.
- **Experiment: engine-aware short chunks (FAILED, reverted).** Hypothesis: feed NeMo-CTC short
  1.2–2.5 s chunks (it degrades past ~2.5 s) instead of the Whisper-tuned 6–15 s. Implemented +
  unit-tested + shipped a test build. **The owner tested it and it was WORSE:** short windows
  force-cut mid-word and `trim_silence` shaved them to 0.2–1.6 s fragments → "first two letters of
  each word." **Reverted.** Lesson re-learned: don't ship a fix that can't be verified end-to-end
  before the owner installs it; and **chunking cannot fix this model** — short chunks fragment,
  long chunks degrade, there is no winning size.
- **Conclusion:** the limitation is the *model*, not the chunker. The real fix is a **better
  multilingual Indic model** (handles code-switching, robust to length). Tracked as a v2 task.
- **PR decision:** rather than put a degraded example in the official docs, **close PR #847 honestly**
  (the model isn't ready as a standalone offline example), revisit later with a stronger model. The
  HuggingFace model + #3199 answer stay up for anyone who wants them.

### 2026-08-14 (later) — Insights Stage 2: yearly recap + shareable cards

Built the second half of the Insights feature (Stage 1 = streaks shipped in v1.1.0).
- **"Your Year" recap.** New `db.year_recap(year)` aggregates a local calendar year from existing
  transcripts (words, dictations, active days, hours spoken, hours saved vs 40 wpm, wpm, busiest day,
  top app) — no new data collected. Command `get_year_recap` + IPC. New `RecapView` with a headline
  (hours saved) + stat grid; reachable **year-round from an Insights card**, and given its **own
  sidebar entry only in Dec–Jan** (`navItems()` in `Sidebar.tsx`). Default year = current, except in
  January it shows the year that just finished.
- **Shareable cards (local PNG, no upload).** `lib/shareCard.ts` renders a card on an offscreen
  Canvas (instrument palette, system fonts); new Rust command `save_share_image` writes it to the
  user's Pictures folder and reveals it in Explorer — **dependency-free** (no dialog/fs plugin, follows
  the existing `explorer` pattern). Nothing leaves the machine.
- **Verified:** recap numbers checked against the real DB (300 dictations, 25,614 words, 10 active
  days, ~6.5 hr saved, busiest day Aug 13); `tsc` clean, 61 Rust tests pass, moonshine build clean.
  **Shipped as v1.2.0.** The card *visual* and save are additive/cosmetic (can't affect dictation), so
  released with a note for the owner to glance in-app; the streak-card share button is deferred.

---

## Standing learnings (hard-won; don't relearn these)

- **Verify by running, never by reading.** Every "looks right" that shipped unverified this project
  has bitten us (fake v1; the chunking regression).
- **A fix the owner must install is not verified until the owner runs it.** Flagging risk is not a
  substitute for validation.
- **Migrations are byte-checksummed** — never let their line endings drift (`.gitattributes` pins LF).
- **Release builds must use the moonshine config** (`build.bat`), never plain `npm run build`, or the
  installer ships without the ONNX/sherpa DLLs and dies on clean machines.
- **The IndicConformer model works only on short, clear, pure-Malayalam input.** Its good in-app
  results come from VAD chunking; it is not a robust general offline model.
