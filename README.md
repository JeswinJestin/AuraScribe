# AuraScribe

Free, open-source, local-first voice dictation. Press a hotkey, speak, and clean punctuated text appears at your cursor in any app — no account, no subscription, no cloud.

![AuraScribe](https://img.shields.io/badge/version-0.3.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey)

## ⬇️ Download (Windows)

Most people should just grab the installer — no build tools, no cloning the repo.

1. Go to the [**Releases**](https://github.com/JeswinJestin/AuraScribe/releases/latest) page.
2. Download **`AuraScribe_x64-setup.exe`** (~4.6 MB).
3. Run it. AuraScribe installs and opens.
4. In **Settings → Voice model**, click **Download & Use** on `base.en` (recommended). It downloads once, then works offline forever.
5. Click into any text field, press **Ctrl+Shift+Space**, speak, press again — your words appear at the cursor.

That's it. The app lives in the system tray; close the window and it keeps running. Right-click the tray icon to quit.

> **Windows only for now.** macOS and Linux are not yet supported — the code has honest stubs that return explicit errors rather than pretending to work. Contributions welcome.

## ✨ Features

- 🎤 **On-device transcription** — Whisper.cpp runs locally; audio never leaves your machine
- ✨ **Automatic cleanup, on by default** — strips filler words, fixes punctuation and sentence casing, all locally
- ⌨️ **Global hotkey** — push-to-talk or toggle mode, rebindable
- 📋 **Types at your cursor** — text is injected into whatever app has focus
- 🔕 **Lives in the tray** — no persistent window; icon shows idle / listening / processing
- 🆓 **Free forever** — no tiers, no word caps, no account, no telemetry

## 🔒 Privacy

This is the whole story, and it's checkable in the source:

- Audio is transcribed **on-device** by Whisper.cpp. It is never uploaded.
- The cleanup pass is **plain local string processing** ([`cleanup.rs`](src-tauri/src/cleanup.rs)) — not an LLM, not a network call.
- **The only network request the app ever makes is downloading a Whisper model** from HuggingFace, once, when you choose one.
- No telemetry, no analytics, no crash reporting.

After the model is downloaded, dictation works fully offline — you can verify by turning off Wi-Fi.

## 🚀 Build from source

Only needed if you want to modify the app — otherwise use the [Download](#️-download-windows)
above.

### Prerequisites

Building from source requires:

- **Node.js** 18+
- **Rust** (stable, with Cargo)
- **LLVM/libclang** — needed by `whisper-rs` bindgen (`winget install LLVM.LLVM`)
- **CMake** — needed to compile whisper.cpp (`winget install Kitware.CMake`)
- **MSVC build tools** (Visual Studio Build Tools with the C++ workload)

Set `LIBCLANG_PATH` to your LLVM `bin` directory (e.g. `C:\Program Files\LLVM\bin`).

### Installation

```bash
git clone https://github.com/JeswinJestin/AuraScribe.git
```

```bash
npm install
```

```bash
dev.bat
```

`dev.bat` puts MSVC, libclang and CMake on `PATH` before running `tauri dev`; a bare
`npm run dev` fails with `Unable to find libclang`. For a release build and installer, use
`build.bat`.

The first build compiles whisper.cpp from source and takes several minutes.

### First Run

1. AuraScribe opens its window on launch, and keeps running **in the system tray** when you
   close it — click the tray icon any time to bring it back
2. Under **Whisper Model**, download one (`large-v3-turbo-q5_0`, ~574 MB, is recommended;
   `base.en`, ~142 MB, is the lighter option)
3. Set your hotkey (default `Ctrl+Shift+Space`) and choose push-to-talk or toggle mode
4. Place your cursor in any text field, press the hotkey, speak, press again

## 🎯 Usage

1. Put your cursor where you want the text
2. Hold your hotkey (or press once in toggle mode)
3. Speak
4. Release (or press again)

Cleaned text is typed at your cursor. That's the whole flow.

## 🏗️ Architecture

- **Frontend**: Next.js 14 + React 18 + TypeScript (settings window and recording overlay only)
- **Backend**: Tauri 2 + Rust
- **Transcription**: Whisper.cpp via `whisper-rs`, running in-process
- **Audio capture**: `cpal`
- **Storage**: local SQLite via `sqlx`

### Project Structure

```
aurascribe/
├── src-tauri/               # Rust backend
│   ├── migrations/          # SQLite schema
│   └── src/
│       ├── asr.rs           # Whisper.cpp integration
│       ├── cleanup.rs       # Local text cleanup pass
│       ├── audio.rs         # Audio capture + resampling
│       ├── hotkey.rs        # Global hotkey registration
│       ├── injection.rs     # Text injection (Windows SendInput)
│       ├── tray.rs          # Tray icon states
│       ├── overlay.rs       # Recording indicator window
│       ├── db.rs            # SQLite access
│       └── commands.rs      # Tauri command surface
├── src/app/                 # Next.js frontend
└── README.md
```

## 🎨 Models

Downloaded once, then used entirely offline. Stored under your local app data directory,
in `AuraScribe/models/`.

All three run faster than real time on a CPU, so dictation never leaves you waiting.

| Model | Size | Language | Speed (CPU) | Role |
|-------|------|----------|-------------|------|
| `tiny.en` | ~75 MB | English | fastest (~0.2×) | for weak machines |
| `base.en` | ~142 MB | English | fast (~0.5×) | **recommended** |
| `base` | ~142 MB | Multilingual | fast (~0.6×) | other languages |

The `large-v3` family was removed: on a CPU those models ran many times slower than real time
and overheated the machine. They can return behind a GPU build (`build-vulkan.bat`).

## 🐛 Troubleshooting

**Model won't download**

Download the `ggml-<model>.bin` file manually from
[HuggingFace](https://huggingface.co/ggerganov/whisper.cpp/tree/main) and place it in the
`AuraScribe/models/` folder under your local app data directory.

**Audio not working**

1. Check the microphone is allowed in Windows privacy settings
2. Pick the specific device under Settings → Microphone instead of "System default"

**Text isn't appearing at the cursor**

Some applications run elevated and reject synthetic keyboard input from non-elevated
processes. In that case AuraScribe copies the text to your clipboard instead and tells you
so — paste with `Ctrl+V`.

## 🗺️ Platform support

Windows is fully supported today. macOS and Linux build targets exist, but text injection
and permission handling are **not yet implemented** on those platforms — they return an
explicit error rather than silently doing nothing. Contributions welcome.

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes
4. Run `cargo test` and `npm run typecheck`
5. Open a Pull Request

## 📄 License

MIT — see [LICENSE](LICENSE).

## 🙏 Acknowledgments

- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) — high-performance Whisper inference
- [whisper-rs](https://github.com/tazz4843/whisper-rs) — Rust bindings
- [Tauri](https://tauri.app/) — desktop framework
- [Next.js](https://nextjs.org/) — React framework

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/JeswinJestin/AuraScribe/issues)
- **Discussions**: [GitHub Discussions](https://github.com/JeswinJestin/AuraScribe/discussions)

If you find this useful, consider giving it a star ⭐
