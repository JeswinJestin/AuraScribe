# AuraScribe — Architecture

Companion to `docs/HANDOFF.md`. This describes how the app is put together and why.

## Stack

- **Backend:** Rust + Tauri 2 (the app's real logic lives here)
- **Frontend:** Next.js 14 + React 18 + TypeScript + Tailwind — used *only* for the
  settings window and the recording overlay. It is not where dictation happens.
- **Speech recognition:** whisper.cpp via `whisper-rs`, compiled in-process
- **Audio capture:** `cpal`
- **Storage:** SQLite via `sqlx`

The frontend is deliberately thin. If you find yourself putting dictation logic in React,
it belongs in Rust — the hotkey can fire when no window exists at all.

## The dictation pipeline

This is the core path. Everything else is support.

```
hotkey.rs (global hotkey)  ─┐
                            ├─→ commands::start_recording
mic button in UI ──────────┘         │
                                     ↓
                     cpal captures f32 mono into AppState::audio_buffer
                                     │
                     commands::stop_recording
                                     ↓
                     audio::resample_linear → 16 kHz
                                     ↓
                     asr::WhisperASR::transcribe   (spawn_blocking)
                                     ↓
                     cleanup::clean                (local string ops)
                                     ↓
                     injection::TextInjector       (Windows SendInput / macOS+Linux enigo+arboard)
                                     ↓
                     db.add_transcript             (history)
```

### Why each piece is the way it is

- **`spawn_blocking` around transcription.** Whisper inference is CPU-bound and would
  otherwise stall the async runtime and freeze the UI.
- **Linear resampling** rather than a resampling crate. Whisper needs 16 kHz mono; mics
  typically deliver 48 kHz stereo. For speech input a linear interpolation is
  indistinguishable in accuracy and avoids a dependency.
- **Cleanup is deterministic string processing, not an LLM.** Two reasons: an LLM call
  would be a network round trip (breaking local-first) or a multi-GB local model (breaking
  lightweight), and either would dominate the latency budget that is the product's main
  advantage.
- **Recording runs on a dedicated OS thread** with a stop flag, because `cpal` streams are
  not `Send` in a way that survives async task migration.

## State and the single status path

`AppState` (in `app_state.rs`) holds the DB handle, the ASR instance, the audio buffer, and
`Status`.

**All status changes must go through `commands::emit_status`.** It is the one place that:

1. emits `status-changed` to the frontend,
2. updates the tray icon (idle / listening / processing),
3. shows or hides the recording overlay.

Bypassing it desyncs those three. If you add a new state transition, route it here.

## Windows

Three windows, all optional:

- **`main`** — the settings window. Starts hidden *if a model is already loaded*; shown on
  first run so the app doesn't look like it failed to launch. Closing it hides rather than
  quits.
- **`overlay`** — small always-on-top borderless indicator, shown only while listening or
  processing. It refuses to show if its page failed to load, because a failed webview load
  renders an opaque browser error page that would sit on top of the user's work.
- **tray icon** — the real "home" of the app. Left click opens settings; menu has
  Open Settings / Quit.

## Cross-platform (Windows / macOS / Linux)

As of v2.0.0 the app builds and runs on all three desktop OSes from one codebase. The platform-specific
surface is small and lives behind `#[cfg(...)]`:

- **Text injection (`injection.rs`).** Windows uses native clipboard + `SendInput`. macOS and Linux use
  [`enigo`](https://github.com/enigo-rs/enigo) (synthetic keystrokes — CGEvent on macOS, X11/XTEST on
  Linux) with [`arboard`](https://github.com/1Password/arboard) for the clipboard-paste path (⌘V on
  macOS, Ctrl+V on Linux), restoring the previous clipboard afterwards.
- **Global hotkey (`hotkey.rs`).** Tauri's cross-platform `global-shortcut` plugin. The default combo is
  per-OS (`commands::default_hotkey`): Windows/Linux `Ctrl+Shift+Space`, macOS `⌘⇧Space` (dodging
  Spotlight/​input-source combos).
- **Still Windows-only (`system.rs`), non-core.** Auto-start-on-login and foreground-window capture are
  Windows-only; off Windows they return an explicit error or are a safe no-op. Missing them does not stop
  dictation.
- **macOS Accessibility (TCC).** Synthetic input *and* global hotkeys require the Accessibility
  permission; `macos-accessibility-client` is used to detect/prompt. The overlay's `.transparent(true)`
  needs Tauri's `macos-private-api` (enabled in `Cargo.toml` + `macOSPrivateApi: true`), which is why
  that build is self-distributed, not App-Store-bound.

### Shipping the sherpa/ONNX shared libraries

`sherpa-rs-sys` links against `onnxruntime` + `sherpa-onnx-c/cxx-api` shared libraries and copies them
into `target/<profile>/` during its build script, but it adds **no runtime rpath** to the final binary
(desktop). So each OS bundles them differently and `src-tauri/build.rs` adds the rpath:

- **Windows** — the DLLs are listed in `tauri.moonshine.conf.json` `bundle.resources`, so NSIS drops
  them next to `aurascribe.exe` (Windows resolves DLLs from the exe's own dir). The 3 MSVC runtime DLLs
  ride along too (see Round 33).
- **macOS** — CI builds only the `.app` (`--bundles app`), embeds `libonnxruntime*` / `libsherpa-onnx-*`
  dylibs into `Contents/Frameworks`, ad-hoc signs, then makes the `.dmg` with `hdiutil`. `build.rs`
  adds rpath `@executable_path/../Frameworks`. (Tauri's own dmg step deletes the `.app` right after
  packaging, which is why we build the app and make the dmg ourselves.)
- **Linux** — the `.deb` currently ships without the `.so` (a documented preview limitation);
  `build.rs` already adds rpath `$ORIGIN` / `$ORIGIN/../lib/AuraScribe` for when they're bundled.

### CI packaging (`.github/workflows/release.yml`)

A matrix over `windows-latest` / `macos-latest` / `ubuntu-latest` builds each native bundle on a tag
push and attaches them to one draft release. Two hard-won CI rules:

- **Do not cache the `target` dir** (`Swatinem/rust-cache` `cache-targets: false`). On a warm cache
  `sherpa-rs-sys` isn't rebuilt and its runtime libs are pruned from `target`, so they exist nowhere —
  breaking the Windows resource check and the macOS embed. A cold target makes every run behave like the
  proven first build.
- **`tsconfig.json` excludes `src-tauri`.** Otherwise `next build` type-checks whisper's CMake
  `compiler_depend.ts` under `target/` and fails with "Invalid character" (only on a warm cache).

## Permissions (Tauri v2 ACL)

`src-tauri/capabilities/default.json` grants the frontend the core and plugin permissions
it needs.

Important subtlety: **custom commands registered in `generate_handler!` are not ACL-gated**
for local origins. Core and *plugin* commands are — including `listen()`, which the UI
depends on. So adding a new command needs no capability change; adding a new **plugin**
does. The original build shipped with no capabilities file at all, which silently denied
every plugin IPC call.

## Database

SQLite at `%LOCALAPPDATA%\AuraScribe\aurascribe.db`, schema in `src-tauri/migrations/`,
applied by `sqlx::migrate!` at startup.

Two hard-won rules:

1. **Migrations run on a dedicated connection that is closed before the app pool opens.**
   A connection opened before a schema-changing migration keeps a stale schema and fails
   later reads with "no column found".
2. **Never edit an applied migration.** sqlx stores a checksum; changing `001` breaks every
   existing install. Add a new file.

`CREATE TABLE IF NOT EXISTS` is not sufficient for upgrades — it silently no-ops against a
differently-shaped legacy table. That is what `002_settings_rebuild.sql` exists to repair.

## Models

Models live in the per-OS app-data dir under `AuraScribe/models/` — Windows
`%LOCALAPPDATA%\AuraScribe\models\`, macOS `~/Library/Application Support/dev.aurascribe.app/models/`,
Linux `~/.local/share/dev.aurascribe.app/models/`. (Whisper's, when used, are `ggml-<id>.bin`.)

- **Local**, not Roaming (Windows) — they reach gigabytes and would otherwise sync on roaming profiles.
- Downloaded to a `.part` file and renamed on completion, so an interrupted download can't
  masquerade as a valid model.
- Auto-loaded at startup if the configured model is already on disk.

## Build notes

- `src-tauri/.cargo/config.toml` pins `CMAKE_POLICY_VERSION_MINIMUM=3.5` because the
  bundled whisper.cpp declares a `cmake_minimum_required` that CMake 4.x refuses. Committed
  deliberately so clones build without per-machine setup.
- `whisper-rs` needs **libclang** (bindgen) and **CMake**; `dev.bat` sets both up.
- **Build the app with the Tauri CLI (`npm run build` / `npx tauri build`), not plain
  `cargo build --release`.** Plain cargo does not embed the frontend assets, producing a
  binary that falls back to the dev server URL and shows a connection error. Note that
  `cargo test --release` will overwrite `target/release/aurascribe.exe` with such a binary.
- The `msi` bundle target is disabled: it downloads the WiX toolset mid-build and hangs.
  NSIS is the standard Tauri Windows installer.
- **Quit any running AuraScribe before building.** A running instance holds a lock on
  `target/release/aurascribe.exe` and the build fails with
  `failed to remove file … Access is denied. (os error 5)`. That error means "the app is
  open", not "the code is broken":

  ```bash
  taskkill /IM aurascribe.exe /F
  ```
