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

### 2026-08-18 — cross-platform release CI (matrix workflow) + config de-Windows-ing

**What & why.** Stood up the multi-OS release pipeline that Project B needs, on GitHub Actions'
free tier for open source. New **`.github/workflows/release.yml`**: on a `v*` tag it fans out a
`strategy.matrix` across **`windows-latest` / `macos-latest` / `ubuntu-latest`**, builds each
bundle **natively in parallel** (Windows mirrors the proven local flow — `--features moonshine
--config tauri.moonshine.conf.json`; macOS/Linux use default features), and attaches every
artifact (`.exe`, `.dmg`, `.deb` + `.AppImage`) to **one** Release. Native runners, not
cross-compilation, because whisper.cpp compiles from source per-host and sherpa-rs downloads a
prebuilt sherpa-onnx/ONNX Runtime per-host — both are far more reliable on the real OS.
`workflow_dispatch` runs the same build without publishing, so a change can be proven to compile
on all three OSes before tagging.

**Config fix (the real "hardcoded Windows assumption").** `tauri.conf.json`'s `bundle.resources`
hardcoded the three MSVC runtime DLLs — which meant a macOS/Linux runner would try to pull Windows
`.dll` files into a `.dmg`/`.deb`/`.AppImage`. Removed that block from the **shared** config; those
DLLs live only in the **Windows-only overlay** `tauri.moonshine.conf.json` (which the owner's build
scripts already pass), so the Windows installer is byte-for-byte unchanged while non-Windows bundles
stay clean. The `bundle.targets` list is left as-is on purpose — Tauri already filters it to the
host's valid bundle types.

**Audit result — the code already ports cleanly (compile-wise).** Every `use windows::` / `use
winreg::` sits inside a `#[cfg(target_os = "windows")]` function; the non-Windows paths in
`injection.rs`, `system.rs`, `commands.rs`, `overlay.rs` are honest `Err("… not yet implemented on
this platform")` stubs (per the CLAUDE.md non-negotiable). `sound.rs`/`audio.rs` use cross-platform
`cpal`. The macOS-only deps in `Cargo.toml` (`objc2`, `core-graphics`) are declared but **not yet
used by any source file** — they're a placeholder for the future injection implementation.

**HONEST LIMITS (do not oversell — this is why the release is a DRAFT):**
- **The macOS/Linux bundles will not dictate yet.** Text injection, hotkey→startup registration,
  "open settings folder", and accessibility all return the "not implemented" stub off Windows. A
  Mac/Linux user could install and open the app, but speaking wouldn't paste text anywhere. Shipping
  these as "production" would violate *"never claim more than the code does."* The workflow drafts the
  release so a human decides how to label the experimental non-Windows assets before publishing.
- **The sherpa-onnx / ONNX Runtime shared libs are not yet bundled on macOS/Linux.** On Windows the
  overlay copies the `.dll`s next to the exe; the `.dylib`/`.so` equivalents on Mac/Linux have never
  been located or bundled. The first real CI run's logs will show their names/paths, and that's the
  follow-up before a Mac/Linux build actually loads a model.
- **Not verifiable from here.** This box is Windows-only and the sandbox blocks the release network
  fetch + git push. The workflow was checked by YAML validation + config parse + a full source audit;
  **the true test is the first `v*` tag push**, which the owner runs. Expect the macOS/Linux rows to
  need one or two iterations (that's exactly what `fail-fast: false` + the draft are for).

**Next (to make Mac/Linux real, not just green):** implement injection + hotkey/startup on macOS
(CGEvent + Accessibility; the `objc2`/`core-graphics` deps are already in place) and Linux
(`xdotool`/`wtype` à la Handy), then bundle the sherpa/ONNX `.dylib`/`.so`. Until then, treat
non-Windows artifacts as experimental previews.

### 2026-08-18 — Spotlight onboarding: interactive walkthrough replaces the modal

**Why.** The owner wanted first-run onboarding to *show* the app, not describe it: highlight one real
UI element at a time, dim + blur the rest, be skippable at every step, and stay short to prevent
drop-off. Brainstormed to a **3-stop** design (Welcome → an animated "how it works" demo → a real
spotlight on the "add a model" action), replayable from Settings.

**Key discovery during design.** On a *fresh install there is no model*, so the Dictate screen shows
only an "Add a voice model to begin" empty state — the record button / hotkey panel appears **after** a
model loads. So a spotlight can reliably anchor to the **Download CTA** on first run, and to the
**record button** on replay. The tour resolves whichever exists. This is exactly the kind of naive
assumption ("spotlight the record button") that reading the code caught before it shipped broken.

**What was built (frontend only):** `SpotlightTour.tsx` (portaled overlay; four blurred panels tile
around the tracked target rect + an indigo ring — no CSS-mask fragility; skip on every step; card is
height-capped + scrollable so controls never fall off a short window) and `HotkeyDemo.tsx` (a JS
phase-machine motion graphic: keys press → mic lights → signal bars → text types itself; reduced-motion
shows the final frame). Wired into `page.tsx` (first-run OR replay, forces the Dictate view so the
anchor exists), `DictateView` (`data-tour` anchors), `SettingsView` (Replay button). Deleted the old
`Onboarding.tsx`.

**Deliberate DESIGN.md exception.** The app's design system restrains motion ("only the signal meter
moves"). The step-2 animation breaks that on purpose — first-run surface only, brief, colour-matched,
reduced-motion-aware. Recorded in the spec so it's intentional, not drift.

**A reusable side-effect: a dev-only preview harness.** `src/app/preview/page.tsx` + a `next dev`
launch config render the real onboarding/streak/recap components against a stubbed Tauri `invoke` with
sample data. This let the owner *watch the onboarding live in a browser* to approve it **without
uninstalling their app or touching their real database** — which was the whole reason they asked to
"show me first." (Caveat: the route is included in a `next build`; remove/gate before shipping.)

**Verified by running,** not by reading: `next dev` → `/preview`, clicked through all three steps —
the animation cycled correctly, the spotlight landed on the real "Choose a model" button with the ring
+ dimmed surround, and Skip dismissed from any step. `npm run typecheck` clean.

**Process note — this is HELD.** Owner's explicit instruction: do **not** push the cross-platform CI
(previous entry) or this onboarding yet. Fix the earlier-reported issues and get per-OS hotkeys working
first, then push everything together. So nothing this session is committed or pushed — the tree carries
the work, waiting.

**Next, per the owner's stated order:** per-OS default hotkeys (macOS needs a Cmd-based, non-alphabet,
conflict-safe default — avoid Cmd+Space/Ctrl+Space; surface a clear error when a global shortcut fails
to register), then window-sizing/DPI responsiveness, the site favicon, and low-voice/VAD audio.

### 2026-08-18 (later) — onboarding round-2 tweaks + per-OS default hotkeys

Owner reviewed the live spotlight tour and asked for: **sound effects** in the step-2 demo, a
**shorter/specific spoken line** (no em dash), and the **Skip button moved inline** next to Next
(muted tone) on steps 1–2 only. Plus: **implement the per-OS default hotkeys.**

- **Sounds** — `src/lib/demoSounds.ts` synthesizes a key-press click and a mic-on chime via Web Audio
  (no dependency, plays once per demo run from within the click gesture, fails silently if blocked).
  `playDemoVoice()` plays `public/onboarding-voice.mp3` if the owner drops an ElevenLabs recording
  there; until then the 404 is swallowed. The demo now **plays once → holds → Replay** instead of
  looping, so the audio isn't repetitive. Spoken line → `Schedule my email for 9 AM.` Keycaps now
  derive from the real hotkey (macOS shows Cmd).
- **Skip inline** — removed the prominent top-right "Skip tour"; Skip is now a muted `btn-ghost` next
  to Next on steps 1 & 2. Step 3 has only "Start dictating."
- **Per-OS hotkeys** — `commands.rs::default_hotkey()` (cfg-based): Windows/Linux `Ctrl+Shift+Space`,
  macOS `Super+Shift+Space` (= Cmd+Shift+Space), avoiding Cmd+Space/Ctrl+Space. Registration failures
  are surfaced: `save_settings` already returned a clear error for a taken combo; `main.rs` startup now
  writes `status.last_error` instead of only logging. The macOS combo is **unvalidated on a real Mac**.

Verified: onboarding tweaks watched live in `/preview` (new text, Replay, inline Skip, sounds wired —
voice 404 graceful); `npm run typecheck` clean; `cargo check --features moonshine` clean (9s, only the
app crate recompiled). Still **HELD** — nothing pushed/committed. Next: window-sizing/DPI, favicon, VAD.

### 2026-08-18 (later) — window-sizing/DPI fix + onboarding voice wired

**Window sizing.** A friend's laptop opened the window a wrong shape (width off, controls near the
edge). Root cause: `main.rs::fit_to_screen` worked in **physical** pixels and only ever *shrank* an
oversized window, so on a high-DPI display the 1480×936 logical design size scaled up past the screen
and got clamped into a bad shape. Rewrote it to compute in **logical** pixels — ~92% of the monitor
work area, clamped to `[min 860×560, design 1480×936]`, DPI-correct via `scale_factor()`. Extracted a
pure `fitted_window_size()` and added **5 unit tests** (1080p, 4K, 1366×768, 150%-DPI laptop, tiny
screen) — `cargo test` 66/66. Honest limit: the maths is proven; the actual on-screen result still
needs the built app on real machines, especially the friend's.

**Onboarding voice.** Owner generated `public/onboarding-voice.mp3` (1.85 s mono, verified 200 +
decodes + `play()` allowed). It was already wired via `demoSounds.playDemoVoice()` at the demo's
"speak" phase; confirmed end-to-end. Keep the recording matching `DEMO_TEXT`
("Schedule my email for 9 AM.").

Still **HELD** — nothing pushed/committed. Remaining: favicon, then low-voice/VAD audio.

### 2026-08-18 (later) — voice timing, disable-hotkey toggle, favicon diagnosis

- **Onboarding voice timing.** Owner wanted the step-2 demo to follow the real flow: after the mic-on
  chime, play the voice fully, then a mic-off cue, THEN the text appears. Reordered `HotkeyDemo`'s
  phase machine to `press → listen(mic on) → speak(voice) → stop(mic off) → type(text) → hold`, added
  `demoSounds.playMicOff()` (falling chime), and the mic now goes dark before the text lands. Verified
  live in `/preview`.
- **Disable-hotkey toggle** (backlog item #2, settings-only). New `hotkey_enabled` setting end-to-end:
  migration `008_hotkey_enabled.sql` (LF), `db.rs` SettingsRow + save query, `commands.rs`
  Settings/Default/mappings/round-trip test, `hotkey::disable()` (unregister_all), `save_settings` and
  `main.rs` startup gated on it, `ipc.ts` + `page.tsx` default + a `Toggle` in Settings → Hotkey. When
  off the app "sleeps" — no keypress triggers dictation. `cargo test` 66/66, `tsc` clean. Floating
  bottom-bar idea stays parked (owner chose settings-toggle-only earlier).
- **Favicon — diagnosed, not a bug.** The blank globe in Google is NOT a broken/missing favicon: the
  live `www.aurascribe.dev/favicon.ico` serves 200 / image/x-icon / 16/32/48 and has since the
  2026-08-14 push. It's **Google's favicon-refresh latency**; the only lever is requesting re-indexing
  in Google Search Console (the landing repo has `docs/SEO-LAUNCH-CHECKLIST.md`). Regenerated the icon
  to 7 sizes (16→256) as hardening (uncommitted in `../aurascribe-landing`). A good reminder to verify
  the live artifact before "fixing" — the file was already correct.

Still **HELD** (app). The only real feature left is **low-voice / noisy-room audio (VAD + gain)** —
the deepest item; it needs real on-device audio testing and deserves its own focused pass.

### 2026-08-18 (later) — per-OS hotkey reaches fresh installs; low-voice gain; GSC diagnosis

- **Per-OS hotkey bug fix.** Discovered (prompted by the owner asking whether onboarding shows the
  right keys per device) that `commands::default_hotkey()` never reached real installs: the settings
  row's hotkey is seeded by migration SQL (`Ctrl+Shift+Space`) on every OS, and the cfg default only
  fired as a Rust fallback. So a fresh macOS install would have had Ctrl+Shift+Space and onboarding
  would show the wrong keys. Fixed in `Database::new`'s fresh-install block (now sets
  `hotkey = default_hotkey()`), and the tour maps `Super → Cmd` in its keycaps. Lesson: a per-OS
  default in Rust means nothing if a migration seeds the value in SQL first — set it on fresh install.
- **Low-voice gain — shipped the doable half of the audio work.** `chunking::normalize_gain()`
  peak-boosts a quiet recording toward 0.9 before transcription: only amplifies, capped at 10×, leaves
  already-loud audio untouched, skips near-silent buffers. Applied after `trim_silence`. 5 unit tests,
  `cargo test` 71/71. The noisy-room half (a real VAD/denoise model + on-device tuning) is genuinely
  large and untestable without real audio, so — per the owner's own if-minimal-now-else-defer rule —
  it's **deferred to a future release**. `chunking.rs` already does energy-based silence VAD for
  splitting/trimming; the gain step complements it.
- **Landing GSC — not a bug.** "Page with redirect" (2) is the intentional apex→www canonicalization;
  "Discovered – currently not indexed" (4 posts) is normal new-site crawl lag (sitemap lists them, blog
  index links them). No `trailingSlash` in next.config, so no stray slash-redirects. Action is time +
  backlinks + the re-index request already made — see the landing `docs/SEO-LAUNCH-CHECKLIST.md`.

Still **HELD**. Next: this is the cut-line for the current release — verify on-device (esp. macOS
hotkey + quiet-speech gain), then the cross-platform push + per-platform releases + docs.

### 2026-08-18 (later) — v1.3.0 release prep (notes + checklist + Windows build)

Cut the release prep for **v1.3.0** (the session's feature set: spotlight onboarding + sounds/voice,
per-OS hotkeys reaching fresh installs, disable-hotkey toggle, low-voice gain, window-sizing fix,
cross-platform CI). Bumped the version to **1.3.0** across `Cargo.toml` / `tauri.conf.json` /
`package.json`. Wrote **`docs/RELEASE-NOTES-v1.3.0.md`** (per-platform, honest: Windows supported;
macOS/Linux **experimental previews that install & launch but do not dictate yet**) and
**`docs/RELEASE-CHECKLIST-v1.3.0.md`** (build → on-device verify → commit/tag → CI builds Mac/Linux on
the tag → draft release → publish). Built the Windows installer locally via `moonshine-build.bat` to
hand the owner a testable `AuraScribe_1.3.0_x64-setup.exe`. **macOS/Linux binaries cannot be built on
the Windows box** — they come only from the `release.yml` cloud runners on a `v1.3.0` tag, and are not
yet worth sharing with friends (no dictation off Windows). Still HELD until the owner verifies on-device
and chooses to push/tag.

### 2026-08-18 (later) — real macOS/Linux dictation implemented (the "future is now")

Owner was firm: ship **working** macOS/Linux, not launch-only previews — friends will install from the
GitHub release and report bugs to iterate on. Correct that cross-platform code is written on Windows and
built by the cloud runners; the runtime constraints (below) are what make it a test-and-iterate loop,
not one-shot.

**Implemented (compiles ONLY on the CI mac/linux runners — this Windows box can't build them):**
- **Text injection** off Windows — `injection.rs` non-Windows path now uses **`enigo`** (keystrokes:
  macOS CGEvent, Linux X11/XTEST) + **`arboard`** (clipboard), mirroring the Windows paste-long/type-short
  strategy (Cmd+V on macOS, Ctrl+V on Linux). Windows keeps its proven native SendInput path untouched.
- **macOS Accessibility permission** — `check/request_accessibility_permission` now use
  `macos-accessibility-client` (`application_is_trusted[_with_prompt]`). This is the gate for BOTH
  injection and the global hotkey on macOS.
- **open_settings_folder** on macOS (`open`) / Linux (`xdg-open`); `.deb` now depends on `libxdo3`.
- Deps: `enigo`/`arboard` under `[target.'cfg(unix)']`, `macos-accessibility-client` under macOS.
  Replaced the never-used `objc2`/`core-graphics` placeholders. `cargo check` (Windows) clean, deps
  resolve, Windows binary unchanged (the sent v1.3.0 installer is still valid).

**HONEST loop (why it's not one-shot):**
1. **I cannot compile the mac/linux code here** — it's `cfg`-gated; the first `v1.3.0` tag push is the
   first real compile check. Fix any errors from CI logs, then binaries exist.
2. **sherpa/ONNX `.dylib`/`.so` bundling** (so models load) is NOT done — I need the first CI build's
   file listing to see where the libs land; guessing filenames blind would just waste a cycle.
3. **macOS**: Accessibility grant is a manual user step; app is unsigned (right-click → Open).
   **Linux**: reliable on X11; **Wayland restricts synthetic input** so it may not type there.
4. Then friends runtime-test and report; iterate.

Release notes updated: macOS/Linux are **🧪 Beta** (newly implemented, needs testing) with the
setup caveats, not "doesn't dictate." Tracked as an open task.
