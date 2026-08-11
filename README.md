# AuraScribe

**Free, open-source, 100% offline voice dictation for Windows** — a private, local alternative to Wispr Flow, Superwhisper, and Dragon. Press a hotkey, speak, and clean punctuated text appears at your cursor in any app. No account, no subscription, no cloud.

![AuraScribe](https://img.shields.io/badge/version-1.0.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey)

> **v1.0.0 is the first stable release.** It's the first build verified to install and run on a
> clean Windows PC — earlier versions (v0.4.x and before) were **previews** and could fail to launch
> on a fresh machine with a *"VCRUNTIME140_1.dll was not found"* error. That's fixed in 1.0.0 (the
> Visual C++ runtime is now bundled). If you're on an older version, update.

## ⬇️ Download (Windows)

Most people should just grab the installer — no build tools, no cloning the repo.

1. Go to the [**Releases**](https://github.com/JeswinJestin/AuraScribe/releases/latest) page.
2. Under **Assets**, download the installer — **`AuraScribe_x64-setup.exe`** (~8.7 MB, bundles the Moonshine engine).
3. Run it (Windows SmartScreen may warn on a new unsigned app — choose *More info → Run anyway*). AuraScribe installs and opens with a short first-run walkthrough.
4. In **Settings → Voice model**, click **Download & Use** on `moonshine-base-en` (recommended). It downloads once, then works offline forever.
5. Click into any text field, press **Ctrl+Shift+Space**, speak, press again — your words appear at the cursor.

That's it. The app lives in the system tray; close the window and it keeps running. Right-click the tray icon to quit.

> **Windows only for now.** macOS and Linux are not yet supported — the code has honest stubs that return explicit errors rather than pretending to work. Contributions welcome.

## ✨ Features

- 🎤 **On-device transcription** — four speech engines run locally via sherpa-onnx: **Moonshine** (English), **NVIDIA Parakeet** (25 European languages), **Dolphin** (~40 Asian languages), and **AI4Bharat IndicConformer** (Malayalam/Kannada). Audio never leaves your machine.
- ✨ **Automatic cleanup, on by default** — strips filler words, fixes punctuation and sentence casing, all locally, on every engine
- 🗂️ **History** — day-grouped, with a usage heatmap and date-range delete, stored only on your device
- ⌨️ **Global hotkey** — Ctrl+Shift+Space, push-to-talk or toggle mode
- 📋 **Types at your cursor** — text is injected into whatever app has focus
- 🔕 **Lives in the tray** — no persistent window; icon shows idle / listening / processing
- 🆓 **Free forever** — no tiers, no word caps, no account, no telemetry

## 🆚 A free, open-source alternative to Wispr Flow, Superwhisper & Dragon

Looking for a **free, open-source, offline alternative to Wispr Flow, Superwhisper, Dragon
NaturallySpeaking, or Windows Voice Typing (Win+H)**? That's exactly what AuraScribe is. Most voice
dictation tools are a paid subscription, cloud-based, closed-source, or send your audio to someone
else's servers. AuraScribe is none of those — it runs entirely on your PC, for free, forever.

| | **AuraScribe** | Wispr Flow | Superwhisper | Windows Voice Typing | Dragon |
|---|:---:|:---:|:---:|:---:|:---:|
| Price | **Free forever** | Subscription | Paid | Free | Paid ($$$) |
| Open source | **✅** | ❌ | ❌ | ❌ | ❌ |
| Runs offline (no cloud) | **✅** | ❌ | ✅ | ❌ (sends to Microsoft) | ✅ |
| No account required | **✅** | ❌ | ✅ | ❌ | ✅ |
| Types into any app | **✅** | ✅ | ✅ | ✅ | ✅ |
| Platform | Windows | Mac/Win | macOS | Windows | Windows |

On top of that, AuraScribe covers **Indian languages most tools ignore** — accurate, on-device
**Malayalam and Kannada** dictation (via AI4Bharat IndicConformer), plus 25 European and ~40 Asian
languages. Your voice never leaves the machine.

<sub>*Also searched as: free voice dictation, open source speech to text, offline dictation for Windows, local speech recognition, private voice typing, voice to text app, Wispr Flow alternative, Superwhisper alternative, Dragon alternative, Windows Voice Typing alternative.*</sub>

## 🔒 Privacy

This is the whole story, and it's checkable in the source:

- Audio is transcribed **on-device** by the local speech engines (sherpa-onnx: Moonshine, Parakeet, Dolphin, IndicConformer). It is never uploaded.
- The cleanup pass is **plain local string processing** ([`cleanup.rs`](src-tauri/src/cleanup.rs)) — not an LLM, not a network call.
- **The only network request the app ever makes is downloading a model** from HuggingFace, once, when you choose one.
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
2. Under **Voice model**, download one (`moonshine-base-en`, ~286 MB, is recommended;
   `moonshine-tiny-en`, ~110 MB, is the lighter option)
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
- **Transcription**: sherpa-onnx engines (Moonshine, Parakeet, Dolphin, IndicConformer) via `sherpa-rs`, running in-process. whisper.cpp (`whisper-rs`) is still compiled in as an internal fallback, but ships with no models.
- **Audio capture**: `cpal`
- **Storage**: local SQLite via `sqlx`

### Project Structure

```
aurascribe/
├── src-tauri/               # Rust backend
│   ├── migrations/          # SQLite schema
│   └── src/
│       ├── engine.rs        # Engine facade — routes to the right speech engine
│       ├── moonshine.rs     # Moonshine (English) via sherpa-onnx
│       ├── parakeet.rs      # Parakeet (European) + custom transducer bundles
│       ├── dolphin.rs       # Dolphin (~40 Asian languages)
│       ├── nemo_ctc.rs      # IndicConformer (Malayalam/Kannada)
│       ├── asr.rs           # Whisper fallback engine (no models shipped)
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

All of these run faster than real time on a CPU, so dictation never leaves you waiting. Four
engines cover different language families, all via sherpa-onnx:

| Model | Engine | Size | Language | Speed (CPU) | Role |
|-------|--------|------|----------|-------------|------|
| `moonshine-base-en` | Moonshine | ~286 MB | English | ~0.1× | **recommended** |
| `moonshine-tiny-en` | Moonshine | ~110 MB | English | ~0.1× | lightest install |
| `dolphin-base-multilang` | Dolphin | ~105 MB | ~40 Asian langs incl. Hindi/Tamil/Telugu/Bengali (auto-detect) | ~0.3× | Asian languages |
| `parakeet-v3-multilingual` | Parakeet | ~671 MB | 25 European langs (auto-detect) | ~0.5× | European languages |
| `indicconformer-ml` / `-kn` | IndicConformer | ~494 MB | **Malayalam** / **Kannada** (AI4Bharat) | ~0.6× | accurate Malayalam/Kannada |

**Bring your own model.** Drop any sherpa-onnx transducer bundle (encoder/decoder/joiner + tokens)
into `AuraScribe/models/<name>/` and it appears in the list automatically — 100% local. This is how
**Hindi / Malayalam and other Indian languages** work, via AI4Bharat's IndicConformer: run
[scripts/export_indicconformer_colab.ipynb](scripts/export_indicconformer_colab.ipynb) in Google
Colab to produce the bundle, then drop it in. See [docs/INDIC-CONFORMER.md](docs/INDIC-CONFORMER.md).
No cloud, ever.

**Why no Whisper models?** Earlier versions shipped Whisper (`tiny.en`/`base.en`/`small`), but the
sherpa-onnx engines above beat them: Moonshine is faster *and* more accurate on English, and Dolphin
/ Parakeet / IndicConformer cover the other languages far faster than Whisper's all-99 `small` model
could on a CPU. So the Whisper catalogue is now empty — the engine stays only as an internal code
fallback. (Large GPU-class models can return behind a GPU build, `build-vulkan.bat`.)

## 🐛 Troubleshooting

**Model won't download**

Models download from HuggingFace the first time you use one. If a download fails, check your
connection and retry from **Settings → Voice model**. Everything is stored under `AuraScribe/models/`
in your local app-data directory; you can also drop a compatible sherpa-onnx model folder there
manually (see *Bring your own model* above).

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

## ❤️ Support AuraScribe

AuraScribe is free and always will be — no tiers, no caps, no account. If it saves you time and
you'd like to support its development, you can sponsor me on GitHub:

[![Sponsor](https://img.shields.io/badge/Sponsor-%E2%9D%A4-ea4aaa?logo=githubsponsors&logoColor=white)](https://github.com/sponsors/JeswinJestin)

Sponsorships fund maintenance and new features (cross-platform support is next). Thank you 🙏

## 🙏 Acknowledgments

AuraScribe packages excellent open speech models and runs them locally — full credit to their
authors, used under their respective licenses:

- **[Moonshine](https://github.com/usefulsensors/moonshine)** (Useful Sensors) — the fast English engine
- **[NVIDIA Parakeet](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3)** — 25 European languages
- **[Dolphin](https://github.com/DataoceanAI/Dolphin)** (DataoceanAI / Tsinghua) — ~40 Asian languages
- **[AI4Bharat IndicConformer](https://github.com/AI4Bharat/IndicConformerASR)** — Malayalam & Kannada (CC-BY-4.0)
- **[sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx)** (k2-fsa) — the offline inference runtime that runs them all, with model exports by [@csukuangfj](https://huggingface.co/csukuangfj)

And the tooling that makes the app:

- [Tauri](https://tauri.app/) — desktop framework · [Next.js](https://nextjs.org/) — React framework
- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) / [whisper-rs](https://github.com/tazz4843/whisper-rs) — the internal fallback engine (compiled in; no models shipped)

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/JeswinJestin/AuraScribe/issues)
- **Discussions**: [GitHub Discussions](https://github.com/JeswinJestin/AuraScribe/discussions)

If you find this useful, consider giving it a star ⭐
