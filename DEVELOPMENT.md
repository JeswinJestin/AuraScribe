# Development Guide for AuraScribe

## 🛠️ Development Setup

### System Requirements

- **Node.js** 18+ with npm
- **Rust** (stable, via rustup)
- **LLVM / libclang** — `whisper-rs` uses bindgen, which needs libclang
- **CMake** — whisper.cpp is compiled from source
- **MSVC Build Tools** — Visual Studio Build Tools with the "Desktop development with C++" workload

### Windows setup

```bash
winget install LLVM.LLVM
```

```bash
winget install Kitware.CMake
```

Then set `LIBCLANG_PATH` so bindgen can find libclang:

```bash
setx LIBCLANG_PATH "C:\Program Files\LLVM\bin"
```

Open a new terminal afterwards so the variable and the updated `PATH` take effect.

> **Note:** `src-tauri/.cargo/config.toml` sets `CMAKE_POLICY_VERSION_MINIMUM=3.5`. The
> whisper.cpp bundled by `whisper-rs-sys` declares a `cmake_minimum_required` below 3.5,
> which CMake 4.x refuses to configure without it. Don't remove this unless whisper-rs is
> upgraded to a version bundling a newer whisper.cpp.

### Install and run

```bash
npm install
```

```bash
npm run dev
```

The first build compiles whisper.cpp and takes several minutes. Later builds are fast.

## 🚀 Development Workflow

### Commands

```bash
npm run dev
```

```bash
npm run build
```

```bash
npm run typecheck
```

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

### Note on the app window

AuraScribe starts hidden in the system tray — this is intentional (`visible: false` in
`tauri.conf.json`). Click the tray icon to open the settings window. Closing that window
hides it rather than quitting; use the tray menu's Quit item to exit.

## 🏛️ Architecture notes

### The dictation pipeline

`hotkey.rs` (or the mic button) → `commands::start_recording` → `cpal` captures f32 mono
samples into `AppState::audio_buffer` → `commands::stop_recording` →
`audio::resample_linear` to 16kHz → `asr::WhisperASR::transcribe` (on a blocking thread) →
`cleanup::clean` → `injection::TextInjector::inject_text` → SQLite `transcripts` row.

Status changes flow through `commands::emit_status`, which is the single place that
updates the tray icon, shows/hides the overlay, and emits `status-changed` to the frontend.
If you add a new state transition, route it through there so all three stay in sync.

### Local-first constraint

The only outbound network request in the entire app is the Whisper model download in
`asr::WhisperASR::download_model`. Cleanup is deterministic local string processing in
`cleanup.rs` — deliberately not an LLM call, both for privacy and because a network round
trip would dominate the latency budget. Please keep it that way; the CSP in
`tauri.conf.json` is intentionally restrictive to make regressions obvious.

### Permissions (Tauri v2 ACL)

`src-tauri/capabilities/default.json` grants the frontend the core and plugin permissions
it needs. Custom commands registered in `generate_handler!` are app-defined and are *not*
ACL-gated for local origins, so adding a new command needs no capability change. Adding a
new **plugin** does.

## 🐛 Debugging

### Backend logs

Logging goes through `tracing`. Adjust verbosity with `RUST_LOG`:

```bash
RUST_LOG=aurascribe=trace npm run dev
```

### Frontend

Right-click in the app window → Inspect Element opens devtools in dev builds.

### Database

SQLite lives at `%LOCALAPPDATA%\AuraScribe\aurascribe.db` on Windows. Schema is in
`src-tauri/migrations/` and applied via `sqlx::migrate!` at startup. To reset state, quit
the app and delete that file.

Models are stored separately under `AuraScribe/models/` in your data directory.

## 🧪 Testing

`cleanup.rs` has unit tests covering the text-transformation rules — these are the cheapest
thing to test and the most likely to regress, so add cases there when you change cleanup
behavior.

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

### Manual testing checklist

- [ ] Model downloads, with progress shown
- [ ] Model auto-loads on next launch without revisiting Settings
- [ ] Hotkey works in both push-to-talk and toggle modes
- [ ] Text lands at the cursor in Notepad, a browser field, and a code editor
- [ ] Tray icon shows idle → listening → processing → idle
- [ ] Overlay appears while recording and disappears afterwards
- [ ] Settings persist across a restart
- [ ] Dictation still works with Wi-Fi off (after the model is downloaded)

## 🤝 Code Style

### Rust

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
```

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml
```

### TypeScript

- Strict mode is on; keep it that way
- All IPC goes through `src/lib/ipc.ts` — don't call `invoke` directly from components, so
  the command surface stays in one auditable place

## 📝 Git Workflow

```bash
git checkout -b feature/your-feature
```

Commit messages follow conventional commits (`feat:`, `fix:`, `docs:`, `refactor:`).

## 🔗 Useful Links

- [Tauri v2 docs](https://v2.tauri.app/)
- [whisper.cpp](https://github.com/ggerganov/whisper.cpp)
- [whisper-rs](https://github.com/tazz4843/whisper-rs)
- [cpal](https://github.com/RustAudio/cpal)
