# AuraScribe — Project Handoff

> **This is the single source of truth for project state.** If you are an AI assistant
> starting a fresh session, read this file first, then `docs/ARCHITECTURE.md`. Update this
> file at the end of every task — see `docs/MAINTAINING-DOCS.md` for the rules.

**Last updated:** 2026-08-05
**Status:** Working end-to-end on Windows. Ships a ~4.6 MB installer.
**Owner:** Jeswin Thomas Jestin

### Round 5 (2026-08-05) — the 404 overlay and the window size

The owner reported a **"404 — This page could not be found"** box appearing on top of
everything during dev testing. Root cause found and fixed:

- **`overlay.rs` asked for `overlay/index.html`.** That is correct for the exported bundle
  but wrong for `next dev`, which serves the route as `/overlay/` and returns **404** for
  `/overlay/index.html`. Verified directly against a running dev server:

  | Request | Dev server |
  |---|---|
  | `/overlay/index.html` | **404** |
  | `/overlay/` | 200 |
  | `/` | 200 |

  Next's 404 page then rendered inside a 220×56 transparent, undecorated, always-on-top
  window — which is exactly the undismissable box that was reported. The path is now chosen
  at runtime with `tauri::is_dev()`. Both branches verified: `/overlay/` returns 200 in dev,
  and `next build` emits `out/overlay/index.html` for release.

- **The "overlay refuses to display if its page failed to load" claim was not true.** The
  guard only checked for a localhost URL in a release build, so it caught bug #9 and nothing
  else — including this. Replaced with positive confirmation: the overlay page calls a new
  `overlay_ready` command on mount, and `overlay::show` returns early until that arrives.
  Only the real page can set that flag, so *any* failed load now fails silently instead of
  parking an error box on screen. This is the second time a broken overlay load reached the
  owner; the guard now matches what the docs claim.

- **Testing guidance corrected.** `npm run dev` is **not** the way to run this — it calls
  `tauri dev` without the MSVC / libclang / CMake environment that whisper-rs needs to
  compile. Use **`dev.bat`**. See `docs/TESTING.md` §0.

### Round 5b — the window opened at the wrong size

The owner reported having to drag the window bigger by hand on every launch. **Round 4's
"window is now 1080x720" change had never actually taken effect**, and reading the config
would never have revealed why:

`tauri-plugin-window-state` restored a persisted size on every launch, overriding both the
configured default *and* `minWidth`/`minHeight`. The saved file on the owner's machine held
`505x758` — below the declared 860 minimum — left over from when the default was 480x720.
Every subsequent config change appeared to do nothing, because the saved state always won.

**The plugin is now removed entirely** (dependency, plugin registration, and the
`window-state:default` capability). For a tray app whose settings window is opened and
hidden constantly it bought nothing, and its restored *position* also fought `center: true`
and could park the window off-screen after a monitor change. The window now opens at its
configured size, centred, every launch.

`%APPDATA%\dev.aurascribe.app\.window-state.json` is now unused. It is harmless, and
deleting it is optional.

**Lesson, same shape as the 404:** a declared config value is not an observed one. Round 4
recorded the window size as fixed on the strength of the config diff alone.

### Round 5c — window sized against a measured reference

With the override gone, the size itself was still wrong: 1080x720 was a guess. The owner
pointed at Wispr Flow as the layout reference, so its window was **measured** rather than
estimated from a screenshot — `EnumWindows` + `GetWindowRect` over the running process:

| Window | Physical rect |
|---|---|
| Flow "Hub" (the reference) | **1565x987** at (152, 27) |
| Flow "Status" (its small pill) | 556x660 |
| AuraScribe before | 1152x798 |

At the owner's 101 DPI (1.052x) that makes the reference **≈1488x938 logical** — 81% of a
1920px width, 91% of its height. The default is now **1480x936**.

A fixed default that suits 1080p is too large for a 1366x768 laptop, so `fit_to_screen` in
`main.rs` shrinks the window to the monitor's work area and re-centres when it doesn't fit.
It only ever shrinks; `minWidth`/`minHeight` still apply. This is what makes the size
*scalable* rather than merely large.

**`fit_to_screen` was wrong on its first attempt, and only running it showed that.** It
capped at 90% of the full screen height — a guess. The log said:

```
Window 1573x1025 doesn't fit 1920x1080 monitor; fitting to 1573x972
```

It was shrinking the window below its design size on an ordinary 1080p desktop for no
reason. The real constraint is `Monitor::work_area()` (tauri 2.11.5), the screen minus the
taskbar: **1920x1031** on that machine, which fits 1025 exactly. Now uses `work_area()`, so
the clamp only fires when a window genuinely would not fit.

Also added **`build.bat`**, the release counterpart to `dev.bat`. A bare `npm run build`
fails at `whisper-rs-sys`'s bindgen step with `Unable to find libclang` unless MSVC, libclang
and CMake are on `PATH`. (Watch the quoting: `set VAR=value && ...` in cmd captures the
trailing space into the value — `build.bat` uses the quoted `set "VAR=value"` form.)

Installer rebuilt and verified: **4.82 MB**, 2026-08-05 16:18.

**Note on the reference:** Flow is the commercial competitor this product is positioned
against. Matching window geometry and layout structure is fair; copying its visual identity
is not, and is also against `docs/DESIGN.md`'s deliberate "instrument" direction. See the
open question at the end of §6.

### Round 6 (2026-08-05) — the window wouldn't open, and injection was corrupting text

Two product-breaking bugs from the owner's first real dictation session.

**1. Clicking the app icon did nothing.** There was no single-instance guard, so launching
from the Start Menu while the app was already in the tray started a *second* process. That
process auto-loaded the model, concluded it was already set up, and — under the old
"only show the window if no model is loaded" rule — never showed a window. Two fixes:

- `tauri-plugin-single-instance` now surfaces the running window on a second launch.
- **The app always shows its window on launch.** Withholding it because a model happened to
  be loaded meant deliberately opening the app produced no window and no feedback. The tray
  is what keeps it alive after you close it; that is not a reason to refuse to open.
- `show_main_window` now logs its failures instead of discarding every `Result` with `let _ =`.
  "The window didn't open" was an unexplainable mystery precisely because of that.

Verified: with a model loaded — the exact case that used to fail — `EnumWindows` reports
`VISIBLE=True 1573x1025 at (173,7)`.

**2. Injected text was mangled.** Real captured output, against a two-minute dictation:

```text
7.cccchose my uuuuuu uuurself,MMMMMM…Mumbai.………………………………
```

The fragments appear *in the right order*, so Whisper was fine — the delivery was destroying
it. `inject_text` built one `SendInput` call carrying ~3,000 key events. Windows delivers
those asynchronously into the target's input queue; it overflows, KEYUPs get dropped, and the
key auto-repeats. Hence `cccc`, `MMMM`, and a tail of thousands of dots.

`injection.rs` was rewritten with two strategies:

- **Paste** (clipboard + Ctrl+V) for anything over 120 characters — instant regardless of
  length, and impossible to corrupt. The previous clipboard contents are restored afterwards.
- **Typing** (`SendInput`) for short text only, now in 40-event batches with a 1 ms gap.

Clipboard access also moved off `powershell -Command Set-Clipboard` — which cost hundreds of
milliseconds per dictation and broke on quotes and newlines — onto the Win32 API directly.
That is most of the "it takes forever" complaint: the old path typed 1,500 characters one
keystroke at a time *and* spawned PowerShell.

Two tests cover it: a clipboard round-trip over quotes, newlines and unicode, and a guard on
the paste/type threshold so a future edit can't route transcripts back onto the typing path.

**Repo hygiene, before the first push to GitHub:**

- **Deleted `SETUP_GUIDE.md`.** It was a survivor of the fake first version: it instructed
  users to get an **OpenRouter API key** for a cleanup feature that no longer exists, claimed
  an "encrypted database" (it is plain SQLite), and suggested backing up to "cloud storage".
  Publishing that would have been the exact "never claim more than the code does" failure
  this project keeps warning about.
- **Replaced `.github/workflows/ci-cd.yml`.** It could not have run: `path: *.msi` is a YAML
  alias and fails to parse, it triggered on `main`/`develop` (the branch is `master`),
  referenced the `msi` bundle target removed in bug #8, and used the retired
  `upload-artifact@v3`. The new `ci.yml` is frontend-only and its steps are verified to pass
  locally. No fake green ticks.
- README corrected: hotkey was still documented as `Ctrl+Space`, install still said
  `npm run dev`, and the model table listed `small.en`/`medium` at roughly half their real
  sizes — the same wrong-sizes bug Round 2 fixed in the UI but never here.
- `.gitignore` now covers `.claude/`, `*.local.json`, `*.log`, and `.window-state.json`.

**Known, not fixed:** `cargo check --no-default-features` fails — `asr.rs` imports
`whisper_rs` unconditionally, so the `whisper` feature flag gates nothing. It is decorative,
which is the same shape as the original fake build. Worth fixing; it is also what stops CI
from building the Rust side without a 10-minute native toolchain setup.

### Round 3 (2026-08-05) — models, UI, and the status bug

- **Upgraded whisper-rs 0.7 → 0.16.** The old crate bundled a 2023 whisper.cpp with no
  `large-v3-turbo`. Turbo is a distilled 4-layer decoder: multilingual, accuracy near
  `large-v3`, but a fraction of the runtime — it genuinely breaks the
  accuracy-versus-speed tradeoff rather than sitting on it.
- **Curated the model list** to `tiny.en`, `base.en`, `large-v3-turbo-q5_0` (recommended),
  `large-v3-turbo`, `large-v3`. **`small.en` and `medium` were removed on purpose**:
  turbo-q5_0 is smaller, faster *and* more accurate than `small.en`, so keeping the old
  tiers would only invite users to pick a strictly worse option. The owner independently
  reported `small.en` as "not working" — it worked, it was just slow for its quality.
- **Thread count now matches the machine.** whisper.cpp defaults to 4 threads regardless of
  hardware; using the real core count is a large, free CPU speedup.
- `use_gpu(true)` is requested. It is a no-op unless a GPU backend feature (`vulkan`,
  `cuda`) is compiled in — enabling one is a follow-up, and the single biggest remaining
  latency lever.
- **Full UI redesign** — sidebar app shell (Dictate / History / Words / Snippets /
  Insights / Settings) replacing the two-tab layout. Design direction is "instrument":
  cool graphite panels, hairline rules, one signal-cyan accent used *only* to mean live,
  monospace for technical readouts. Signature element is the **signal meter**, which is
  flat when idle, ticks while listening, and sweeps while transcribing.
- **Dictionary, Snippets, History and Insights now have real UIs.** The backend CRUD and
  the `transcripts` table already existed and were simply never surfaced.
- **Fixed the "still shows Setup Required" bug.** Status arrived only via `status-changed`
  events; a single missed event stranded the UI claiming no model was loaded while the
  backend had one. Status is now re-read on every view change, so that state is
  unrecoverable-by-design no more.
- Added `audio_ms` to transcripts (migration `004`) so words-per-minute reflects real
  speaking time rather than processing time.

### Round 4 (2026-08-05) — the desync, properly fixed

The "still shows Setup Required" bug survived two attempted fixes. Root cause and remedy:

- **`Status` now carries `loaded_model`.** The UI was deriving "is this model active?" from
  the *saved setting* plus a boolean. Those diverge the moment a load fails or is in
  flight, so every model could render as inactive while one was loaded. The backend now
  reports which model is genuinely in memory, and the UI trusts only that.
- **Status is polled every 1.5s**, not just pushed via events. A dropped `status-changed`
  previously left the UI permanently wrong with no user-recoverable path. `get_status` is
  an in-memory read, so this is cheap and makes the desync self-healing. Events remain the
  fast path.
- **Download progress no longer jitters.** Progress was emitted once per network chunk —
  thousands of IPC messages a second, which made the bar shake rather than advance. Now
  throttled to visible movement (0.5%).
- The active model is shown on the Dictate screen and in the sidebar rail, so "which model
  am I using?" is always answerable.

**Measured after upgrading to whisper-rs 0.16** (same 6.59s clip, same transcript):
1.98s (0.7 debug) → 1.68s (0.7 release) → **1.19s (0.16 debug)**. Roughly 5.5x realtime
before release optimisation, from the newer whisper.cpp plus using the machine's real core
count.

### Round 4 UI work

- Window default was **480x720** — far too small for a six-section app. Now **1080x720**,
  minimum 860x560.
- Custom scrollbars: slim, rounded, inset. The default Windows bar made an otherwise quiet
  interface look unfinished.
- **Collapsible sidebar** (212px ↔ 60px) with icon-only mode and tooltips.

---

## 1. What this is

A free, open-source, **local-first** voice dictation app. Press a hotkey, speak, and clean
punctuated text appears at your cursor in any application.

It exists because the market has a gap:

| | Cloud? | Free? | Cleanup built in? |
|---|---|---|---|
| **Wispr Flow** (commercial leader) | Yes | No — subscription | Yes |
| **Handy** (best OSS alternative) | No | Yes | **No — raw transcript** |
| **AuraScribe** | **No** | **Yes, forever** | **Yes, on by default** |

Nobody had shipped free + open source + local + a cleanup layer that's on by default and
fast. That combination is the entire product.

### Non-negotiable principles

1. **Local-first.** Audio and text never leave the machine. The *only* network call in the
   entire app is a one-time Whisper model download.
2. **Free forever.** No tiers, no word caps, no account. Donations never gate a feature.
3. **Lightweight.** 4.6 MB installer, ~40 MB idle RAM (~180 MB with a model loaded).
4. **Honest security posture.** No telemetry, no analytics. Claims must be checkable in
   source — never claim more than the code does.
5. **Cross-platform intent.** Windows works today; macOS/Linux are stubbed honestly.

### Explicit non-goals (v1)

No meeting transcription, no mobile app, no cloud option even opt-in, no team features, no
LLM "agent command" mode, no wake words, no paid tiers. These are v2+ conversations *after*
daily use is proven.

---

## 2. Current state — what actually works

Verified by running it, not by reading code:

| Capability | Status | Evidence |
|---|---|---|
| Local Whisper transcription | ✅ Working | `"The quick brown fox jumps over the lazy dog…"` transcribed exactly |
| Transcription speed | ✅ 1.68s for 6.59s audio (~3.9× realtime, release build) | integration test |
| Local cleanup pass | ✅ Working | 12 unit tests, incl. real captured output |
| Global hotkey (toggle + push-to-talk) | ✅ Registered | 4 real recording sessions in logs |
| Text injection at cursor (Windows) | ✅ `SendInput` w/ clipboard fallback | manual |
| Tray icon w/ 3 states | ✅ Built | manual |
| Recording overlay | ✅ Built | dev path fixed + guarded, Round 5 |
| Model download + auto-load on startup | ✅ Working | logs |
| Settings persistence | ✅ SQLite | verified across restarts |
| Dictionary / snippets / history | ✅ Real CRUD **and UI** | schema + commands + views |
| Usage insights (words, wpm, streak) | ✅ Computed from local history | `db::stats` |
| Production installer | ✅ 4.58 MB NSIS | `AuraScribe_1.0.0_x64-setup.exe` |
| macOS / Linux | ❌ Not implemented | returns explicit errors, never fake success |

### Measured numbers

- Installer: **4.58 MB** · release binary: **13.08 MB**
- RAM: **~40 MB** idle, **~180 MB** with `base.en` loaded (the model is ~140 MB of that)
- Transcription: **~3.9× realtime** (a 3-second phrase ≈ under a second)
- Cleanup: pure string ops, negligible next to transcription
- Full release build: **~4–6 min** (whisper.cpp compiles from source)

---

## 3. History — why the code looks like it does

The project had a **first attempt that never worked**, then a full rebuild.

### What the first attempt actually was

A polished UI shell over a backend that was mostly stubs:

- Whisper code **had never once compiled** — it called `WhisperContextParameters` and
  `new_with_params`, which don't exist in whisper-rs 0.7. The feature flag was off.
- The only working pipeline **uploaded audio to OpenRouter's cloud** and required an API
  key — the exact opposite of the stated product.
- Text injection only ran `Set-Clipboard`; it never pasted, so text never reached the cursor.
- Dictionary, snippets, history, permissions, model management were **hardcoded fakes**
  (`get_dictionary` → `[]`, `add_dictionary_entry` → always `1`).
- **No `capabilities/` directory existed at all**, so Tauri v2's ACL denied every core and
  plugin IPC call, including the `listen()` the UI depends on.
- The UI showed an "AES-256 encrypted" privacy card while storing the API key **in plaintext**.
- The README credited "Silero VAD" that was never implemented.

### The rebuild (2026-08-04 / 08-05)

Removed the entire cloud path (`llm.rs`, `ollama.ts`, `crypto.rs`, OpenRouter settings),
deleted dead modules (`vad.rs`, `models.rs`, `events.rs`, `db.ts`), and built the real
runtime: hotkey registration, local Whisper, local cleanup, real `SendInput` injection,
tray-first window model, overlay, and genuine DB-backed CRUD.

**Net −1,126 lines while adding functionality.**

### Bugs found only by actually running it

These are worth remembering — several were invisible to code review:

1. **CMake 4.x rejects whisper.cpp's `cmake_minimum_required < 3.5`.** Pinned via committed
   `src-tauri/.cargo/config.toml` so clones build without per-machine env setup.
2. **Migrations silently no-opped.** `CREATE TABLE IF NOT EXISTS settings` did nothing
   against a legacy table from an older architecture → app crashed on launch. Fixed with
   `002_settings_rebuild.sql`.
3. **Stale schema across a migration.** Pool connections opened *before* a schema-changing
   migration held an old schema → "no column found for name: hotkey". Migrations now run on
   a dedicated connection that is closed first.
4. **Models were written to Roaming AppData** — GB-sized files would sync on roaming
   profiles. Moved to Local.
5. **`load_model` returned `Ok(())` even when loading failed**, so clicking "Load" did
   nothing with no error shown. This is why models appeared un-loadable.
6. **App was invisible on first run** (`visible: false` + `skipTaskbar: true`) — no window,
   no taskbar entry, tray icon buried in Windows' overflow. Looked like a failed launch.
7. `normalize_punctuation` had an unreachable branch, so `"the store ."` kept its space.
8. Tauri's `msi` bundle target downloads the WiX toolset mid-build and **hung**. Removed;
   NSIS is the standard Tauri Windows installer.
9. **`cargo test --release` silently breaks the app.** Plain cargo rebuilds
   `target/release/aurascribe.exe` *without* embedding frontend assets, so the binary falls
   back to the dev-server URL. Running it then shows "localhost refused to connect" in every
   window — most visibly in the always-on-top overlay, which looks like a stuck error box
   the user can't dismiss. Always rebuild with `npm run build` afterwards.
10. **Dev server and static export disagree on page filenames.** `next dev` serves
    `/overlay/` and 404s on `/overlay/index.html`; the export only contains
    `overlay/index.html`. A hardcoded path is right in exactly one of the two. Any *new*
    secondary window must pick its path with `tauri::is_dev()` — see `overlay.rs`.
11. **`Ctrl+Space` is a bad default hotkey.** Windows IME claims it for input-language
    switching, so registration can silently do nothing. Default is now `Ctrl+Shift+Space`
    (what the PRD originally suggested), with migration `003` moving anyone still on the
    old default.

### Round 2 fixes (2026-08-05, after first real user test)

The owner's first hands-on test surfaced UX failures that testing-by-developer had missed:

- **"I can't see it as an open tab"** — the app was `visible: false` + `skipTaskbar: true`,
  so on first run there was no window, no taskbar entry, and the tray icon was buried in
  Windows' overflow chevron. It looked like the app hadn't launched. Now the settings window
  opens automatically when no model is loaded, and a taskbar entry appears when visible.
- **"I downloaded the model but it still says setup required"** — caused by bug #5
  (`load_model` returning `Ok` on failure). Download and load are now a single
  **"Download & Use"** action, and failures appear in red in the Settings panel.
- **"I don't understand what a Whisper model is"** — the UI now explains it in plain
  language and states that it downloads once and then runs offline free forever.
- Model sizes shown in the UI were roughly half the real download size (e.g. `base.en`
  listed as 74 MB, actually ~142 MB). Corrected.
- The overlay now refuses to display if its page failed to load, so a broken build can
  never again park an undismissable error box on screen.
- **Whisper's non-speech annotations were being typed into the user's document.** Real
  captured output included `[Music] [Music] [Music]`, `[typing sounds]`,
  `[indistinct chatter]` and `[BLANK_AUDIO]`. Whisper narrates silence and background
  noise this way; cleanup passed it straight through. Now stripped, and a recording that
  contains nothing but annotations injects nothing at all rather than typing junk at the
  cursor. Parenthesised spans are only removed when they look like audio descriptions, so
  a genuinely dictated aside like "(roughly ten)" survives.

**Quality note from that same session:** actual speech transcribed excellently —
*"Hi, hi, hello. This is a test run that I'm testing. So I hope this works perfectly."*
The engine was never the problem; presentation and edge cases were.

---

## 4. How to run it

### Prerequisites (Windows)

```bash
winget install LLVM.LLVM
```

```bash
winget install Kitware.CMake
```

Plus **Visual Studio Build Tools** with the C++ workload, Node 18+, and Rust stable.
Set `LIBCLANG_PATH` to `C:\Program Files\LLVM\bin`.

### Development

```bash
dev.bat
```

`dev.bat` sets up MSVC, libclang, and CMake, then runs `npx tauri dev`. First build takes
several minutes (whisper.cpp compiles from source); later builds are fast.

**Do not use `npm run dev`.** It runs `tauri dev` without that environment, so whisper-rs
fails to compile. `dev.bat` is the wrapper that makes the same command work.

### Production build

```bash
build.bat
```

Wrapper around `npm run build` that sets up the same toolchain `dev.bat` does. A bare
`npm run build` fails with `Unable to find libclang`.

Produces `src-tauri/target/release/bundle/nsis/AuraScribe_1.0.0_x64-setup.exe`.

### Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

The real transcription test is `#[ignore]` by default (needs a model + sample audio):

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test transcription -- --ignored --nocapture
```

Set `AURASCRIBE_TEST_WAV` to a 16 kHz mono WAV first.

---

## 5. How to use it (and what to tell users)

**"Whisper model" means:** the speech-recognition file that turns your voice into text.
It's downloaded **once** (~574 MB for the recommended `large-v3-turbo-q5_0`, or ~142 MB
for the lighter `base.en`), then everything runs
offline on your own machine, free, forever. Bigger models are more accurate but slower.

### First run

1. The settings window **opens automatically** when no model is installed.
2. Settings → Whisper Model → **Download & Use** on `large-v3-turbo-q5_0` (recommended),
   or `base.en` if you want a smaller download.
3. It downloads once and loads immediately — Home should then say **Ready**.
4. Close the window; the app keeps running in the system tray.

### Daily use

1. Click into any text field — Notepad, Chrome, VS Code, Slack, Word, anything.
2. Press **Ctrl+Space** (default), speak, press again to stop (toggle mode).
3. Cleaned text is typed at your cursor.

### Testing across applications

It works in any app that accepts keyboard input, because it synthesizes real keystrokes
via `SendInput`. Good things to try: Notepad, a browser address bar or textarea, VS Code,
Word, a chat app.

**Known limitation:** apps running *elevated* (as Administrator) reject synthetic input
from a non-elevated process. AuraScribe detects this, copies the text to your clipboard
instead, and tells you — paste with Ctrl+V. Run AuraScribe as admin if you need this.

### Where things live

- Database + settings: `%LOCALAPPDATA%\AuraScribe\aurascribe.db`
- Models: `%LOCALAPPDATA%\AuraScribe\models\`
- To fully reset: quit the app and delete that folder.

---

## 6. Roadmap

### Immediate next steps

- [ ] **Interactive verification by the owner** — hotkey in both modes, injection across
      several apps, tray state colors, overlay visibility. See `docs/TESTING.md`.
      **Overlay specifically:** run `dev.bat`, dictate, and confirm a "Listening…" pill
      appears bottom-centre — not a 404, and not nothing. Round 5 fixed the path and added
      a guard, but neither has been confirmed on screen by a human.
- [ ] Commit the rebuild (currently uncommitted — the whole rebuild plus docs).
- [ ] Daily-use trial: one week without switching back to another tool (the PRD's real
      definition of "v1 done").
- [ ] Consider installing via the NSIS installer rather than running the dev binary — it
      gives a Start Menu entry and avoids the dev-server class of problem entirely.

### Phase 2 (only after daily use is proven)

- [ ] **Personal dictionary** — DB table and CRUD commands already exist; needs to be
      applied to transcripts in `cleanup.rs` and exposed in the UI.
- [ ] **Per-app formatting rules** — `app_profiles` table exists; needs foreground-window
      detection (Windows `GetForegroundWindow`) and profile matching.
- [ ] **Dictation history UI** — `transcripts` table is already populated; needs a view and
      a "copy last" shortcut.
- [ ] Multilingual support (Whisper's multilingual models already download).

### Upgrades worth doing

- [ ] **GPU acceleration (Vulkan/CUDA).** whisper-rs is now on 0.16 (done in Round 3), and
      `use_gpu(true)` is already requested — but it is a no-op until a backend feature is
      compiled in. This is the single biggest remaining latency lever.
- [ ] **macOS support** — needs `CGEvent`-based injection and Accessibility permission
      handling. Interfaces already exist and return explicit errors.
      **Do this first:** the bundle identifier is `dev.aurascribe.app`, and Tauri warns that
      ending in `.app` conflicts with the macOS application bundle extension. Change it
      before any macOS build (and before wide distribution — changing it later makes
      existing installs look like a different app). Harmless on Windows, so it was left
      alone rather than forcing another release rebuild.
- [ ] Streaming/partial transcription for perceived latency.
- [ ] Auto-stop on silence (a VAD existed but was unused and removed; recover from git if
      wanted).

### Ideas for a richer UI (asked about; deliberately deferred)

A dashboard with dictation stats (words dictated, time saved, accuracy trends) is
appealing, but check it against the PRD's own warning: **scope creep is the single biggest
risk to finishing.** The app is a background utility, not something users stare at. Suggested
order if pursued: history view first (data already exists) → dictionary management → then
stats. Keep the settings window a single scrollable pane; multi-tab settings at this scope
would be over-engineering.

---

## 7. Things to be careful about

- **Never add a cloud fallback.** It breaks the core promise and the entire reason the
  product exists. The restrictive CSP in `tauri.conf.json` is deliberate so regressions are
  obvious.
- **Never claim more than the code does.** The previous build's fake "AES-256 encrypted"
  card is exactly the failure mode to avoid.
- **Don't fake success on unimplemented platforms.** Return an explicit error.
- **Route all status changes through `commands::emit_status`** — it is the single place
  that updates the tray icon, the overlay, and the frontend. Bypassing it desyncs them.
- **Don't edit an already-applied migration.** sqlx records a checksum; changing 001 breaks
  every existing install. Add a new migration instead.
- **All IPC goes through `src/lib/ipc.ts`** so the command surface stays auditable.
- Commands must **return `Err` on failure**, not log-and-return-`Ok` — bug #5 above.
- **New secondary windows must pick their page path with `tauri::is_dev()`.** `next dev`
  serves `/route/`; the export contains `route/index.html`. A hardcoded path is correct in
  exactly one of the two — bug #10 above.
- **Don't re-add `tauri-plugin-window-state`.** It was removed in Round 5b because it
  overrode the configured window size *and* the declared minimum, making config changes
  look like no-ops. If per-window persistence is ever genuinely wanted, restore position
  only — never size.
- **Never show a window before its page confirms it loaded.** Always-on-top, undecorated
  windows turn any load failure into an error box the user cannot dismiss. The overlay's
  `overlay_ready` handshake is the pattern to copy.
