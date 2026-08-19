# AuraScribe

**Free, open-source, 100% offline voice dictation for Windows, macOS & Linux** — a private, local alternative to Wispr Flow, Superwhisper, and Dragon. Press a hotkey, speak, and clean punctuated text appears at your cursor in any app. No account, no subscription, no cloud.

![AuraScribe](https://img.shields.io/badge/version-2.0.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)

**🌐 Website: [aurascribe.dev](https://www.aurascribe.dev)** · **[⬇️ Download](https://github.com/JeswinJestin/AuraScribe/releases/latest)** · **[📖 Install guide](docs/INSTALL.md)**

> **v2.0.0 is the first cross-platform release.** Windows is the fully-supported, daily-driver build.
> **macOS (Apple Silicon) and Linux (.deb) are new previews** — they install, launch, and dictate
> (global hotkey + typing into other apps), but loading a voice model on those two platforms is still
> being verified on real hardware. See [**per-OS install instructions**](docs/INSTALL.md) — the macOS
> build in particular needs a one-time Gatekeeper step because it isn't notarized.

## ⬇️ Download

Most people should just grab the installer for their OS — no build tools, no cloning the repo. Get the
latest from the [**Releases**](https://github.com/JeswinJestin/AuraScribe/releases/latest) page.

| OS | Asset | Notes |
|----|-------|-------|
| **Windows** (x64) | `AuraScribe_2.0.0_x64-setup.exe` (~8 MB) | Fully supported. SmartScreen may warn on a new unsigned app → *More info → Run anyway*. |
| **macOS** (Apple Silicon) | `AuraScribe_2.0.0_aarch64.dmg` (~24 MB) | Preview. Not notarized — needs a one-time Gatekeeper step (below). M1/M2/M3/M4. |
| **Linux** (Debian/Ubuntu x64) | `AuraScribe_2.0.0_amd64.deb` (~6 MB) | Preview. `sudo apt install ./AuraScribe_2.0.0_amd64.deb`. |

**Full step-by-step for every OS — including the macOS "unidentified developer" fix and the
permissions dictation needs — is in [docs/INSTALL.md](docs/INSTALL.md).** The short version:

**Windows**
1. Download and run `AuraScribe_2.0.0_x64-setup.exe`.
2. In **Settings → Voice model**, click **Download & Use** on `moonshine-base-en` (recommended).
3. Click into any text field, press **Ctrl+Shift+Space**, speak, press again — your words appear.

**macOS** (Apple Silicon)
1. Open the `.dmg` and drag **AuraScribe** to **Applications**.
2. Clear Gatekeeper once — the app isn't notarized. Either **System Settings → Privacy & Security →
   Open Anyway**, or in Terminal: `xattr -dr com.apple.quarantine /Applications/AuraScribe.app`.
3. Grant **Accessibility** (and Microphone) under **System Settings → Privacy & Security** — this is
   what lets AuraScribe register the hotkey and type your text. The default hotkey is **⌘⇧Space**.

**Linux** (Debian/Ubuntu)
1. `sudo apt install ./AuraScribe_2.0.0_amd64.deb`
2. Launch **AuraScribe**, download a model, dictate with **Ctrl+Shift+Space**. Use an **Xorg** session
   if the hotkey/typing doesn't work under Wayland.

The app lives in the system tray/menu bar; close the window and it keeps running.

## ✨ Features

- 🎤 **On-device transcription** — four speech engines run locally via sherpa-onnx: **Moonshine** (English), **NVIDIA Parakeet** (25 European languages), **Dolphin** (~40 Asian languages), and **AI4Bharat IndicConformer** (Malayalam/Kannada). Audio never leaves your machine.
- 🖥️ **Cross-platform** — Windows, macOS (Apple Silicon), and Linux (Debian/Ubuntu), from one codebase.
- ✨ **Automatic cleanup, on by default** — strips filler words, fixes punctuation and sentence casing, all locally, on every engine.
- 🗂️ **History** — day-grouped, with a usage heatmap and date-range delete, stored only on your device.
- ⌨️ **Global hotkey** — Ctrl+Shift+Space (⌘⇧Space on macOS), push-to-talk or toggle mode.
- 📋 **Types at your cursor** — text is injected into whatever app has focus.
- 🔥 **Streaks & insights** — a local dictation streak, milestones, and a yearly recap, all on-device.
- 🔕 **Lives in the tray** — no persistent window; icon shows idle / listening / processing.
- 🆓 **Free forever** — no tiers, no word caps, no account, no telemetry.

## 🆚 A free, open-source alternative to Wispr Flow, Superwhisper & Dragon

Looking for a **free, open-source, offline alternative to Wispr Flow, Superwhisper, Dragon
NaturallySpeaking, or Windows Voice Typing (Win+H)**? That's exactly what AuraScribe is. Most voice
dictation tools are a paid subscription, cloud-based, closed-source, or send your audio to someone
else's servers. AuraScribe is none of those — it runs entirely on your device, for free, forever.

| | **AuraScribe** | Wispr Flow | Superwhisper | Windows Voice Typing | Dragon |
|---|:---:|:---:|:---:|:---:|:---:|
| Price | **Free forever** | Subscription | Paid | Free | Paid ($$$) |
| Open source | **✅** | ❌ | ❌ | ❌ | ❌ |
| Runs offline (no cloud) | **✅** | ❌ | ✅ | ❌ (sends to Microsoft) | ✅ |
| No account required | **✅** | ❌ | ✅ | ❌ | ✅ |
| Types into any app | **✅** | ✅ | ✅ | ✅ | ✅ |
| Platform | **Win / macOS / Linux** | Mac/Win | macOS | Windows | Windows |

On top of that, AuraScribe covers **Indian languages most tools ignore** — accurate, on-device
**Malayalam and Kannada** dictation (via AI4Bharat IndicConformer), plus 25 European and ~40 Asian
languages. Your voice never leaves the machine.

<sub>*Also searched as: free voice dictation, open source speech to text, offline dictation for Windows / Mac / Linux, local speech recognition, private voice typing, voice to text app, Wispr Flow alternative, Superwhisper alternative, Dragon alternative, Windows Voice Typing alternative.*</sub>

## 🔒 Privacy

This is the whole story, and it's checkable in the source:

- Audio is transcribed **on-device** by the local speech engines (sherpa-onnx: Moonshine, Parakeet, Dolphin, IndicConformer). It is never uploaded.
- The cleanup pass is **plain local string processing** ([`cleanup.rs`](src-tauri/src/cleanup.rs)) — not an LLM, not a network call.
- **The only network request the app ever makes is downloading a model** from HuggingFace, once, when you choose one.
- No telemetry, no analytics, no crash reporting.

After the model is downloaded, dictation works fully offline — you can verify by turning off Wi-Fi.
The restrictive Content-Security-Policy in `tauri.conf.json` is deliberate, so any accidental network
call would break the build loudly.

## 🗺️ Platform support

| Platform | Status | Dictation loop | Notes |
|----------|--------|----------------|-------|
| **Windows** (x64) | ✅ Supported | Hotkey · record · transcribe · inject | The proven, verified build. Auto-start on login supported. |
| **macOS** (Apple Silicon) | 🧪 Preview (v2.0.0) | Hotkey · record · transcribe · inject | Ad-hoc signed (not notarized). Needs Accessibility permission. **Model-loading pending on-device verification.** |
| **Linux** (Debian/Ubuntu x64) | 🧪 Preview (v2.0.0) | Hotkey · record · transcribe · inject | X11 recommended (Wayland limits synthetic input). No auto-start yet. **Model-loading pending on-device verification.** |

The whole dictation loop — global hotkey, recording, transcription, and typing into other apps — is
implemented on **all three** platforms (Windows uses native APIs; macOS/Linux use
[`enigo`](https://github.com/enigo-rs/enigo) for keystrokes and
[`arboard`](https://github.com/1Password/arboard) for clipboard paste). The macOS/Linux installers
bundle the sherpa-onnx / ONNX Runtime libraries. What still needs a real-device check on those two is
whether a downloaded model loads at runtime; if not, `aurascribe.log` names the exact library involved.
That's why the v2.0.0 non-Windows assets are labeled **preview** and the release starts as a draft.

## 🎨 Models

Downloaded once, then used entirely offline. Stored under your local app-data directory, in
`AuraScribe/models/`. All of these run faster than real time on a CPU, so dictation never leaves you
waiting. Four engines cover different language families, all via sherpa-onnx:

| Model | Engine | Size | Language | Speed (CPU) | Role |
|-------|--------|------|----------|-------------|------|
| `moonshine-base-en` | Moonshine | ~286 MB | English | ~0.1× | **recommended** |
| `moonshine-tiny-en` | Moonshine | ~110 MB | English | ~0.1× | lightest install |
| `dolphin-base-multilang` | Dolphin | ~105 MB | ~40 Asian langs incl. Hindi/Tamil/Telugu/Bengali (auto-detect) | ~0.3× | Asian languages |
| `parakeet-v3-multilingual` | Parakeet | ~671 MB | 25 European langs (auto-detect) | ~0.5× | European languages |
| `indicconformer-ml` / `-kn` | IndicConformer | ~494 MB | **Malayalam** / **Kannada** (AI4Bharat) | ~0.6× | accurate Malayalam/Kannada |

**Bring your own model.** Drop any sherpa-onnx transducer bundle (encoder/decoder/joiner + tokens)
into `AuraScribe/models/<name>/` and it appears in the list automatically — 100% local. See
[docs/INDIC-CONFORMER.md](docs/INDIC-CONFORMER.md). No cloud, ever.

**Why no Whisper models?** Earlier versions shipped Whisper, but the sherpa-onnx engines above beat
them: Moonshine is faster *and* more accurate on English, and Dolphin / Parakeet / IndicConformer
cover the other languages far faster than Whisper's `small` could on a CPU. So the Whisper catalogue
is empty — the engine stays only as an internal code fallback.

## 🚀 Build from source

Only needed if you want to modify the app — otherwise use the [Download](#️-download) above.

### Prerequisites (all platforms)

- **Node.js** 18+
- **Rust** (stable, with Cargo)
- **LLVM/libclang** — needed by `whisper-rs` bindgen
- **CMake** — needed to compile whisper.cpp

Per-OS extras:
- **Windows:** MSVC build tools (VS Build Tools, C++ workload). Set `LIBCLANG_PATH` to your LLVM `bin`.
- **macOS:** Xcode Command Line Tools (`xcode-select --install`) + `brew install cmake`.
- **Linux:** `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev libxdo-dev libssl-dev patchelf build-essential cmake clang libclang-dev`.

### Build

```bash
git clone https://github.com/JeswinJestin/AuraScribe.git
```

```bash
npm install
```

```bash
npx tauri build --features moonshine
```

On Windows use the provided `build.bat` / `moonshine-build.bat` (they put MSVC, libclang, and CMake on
`PATH` and pass the Windows DLL-bundling overlay). The first build compiles whisper.cpp from source
and takes several minutes. **Always build with the Tauri CLI, never plain `cargo build --release`** —
plain cargo doesn't embed the frontend assets.

### Project structure

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
│       ├── injection.rs     # Text injection (Windows native / enigo+arboard on macOS/Linux)
│       ├── hotkey.rs        # Global hotkey registration (Tauri global-shortcut)
│       ├── audio.rs         # Audio capture + resampling (cpal)
│       ├── db.rs            # SQLite access (sqlx)
│       └── commands.rs      # Tauri command surface
├── src/app/                 # Next.js frontend (settings window + recording overlay)
└── docs/                    # HANDOFF, ARCHITECTURE, INSTALL, PROJECT-JOURNAL, …
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for how it's built and why.

## 🐛 Troubleshooting

**Model won't download** — models come from HuggingFace the first time you use one. If a download
fails, check your connection and retry from **Settings → Voice model**. You can also drop a compatible
sherpa-onnx model folder into `AuraScribe/models/` manually.

**Text isn't appearing at the cursor**
- **Windows:** some apps run elevated and reject synthetic input from a non-elevated process; AuraScribe
  falls back to the clipboard and tells you — paste with `Ctrl+V`.
- **macOS:** grant **Accessibility** (and **Input Monitoring**) under Privacy & Security, then quit and
  reopen AuraScribe.
- **Linux:** use an **Xorg** session — Wayland restricts synthetic keyboard input.

**macOS says the app is damaged / from an unidentified developer** — it's unsigned/un-notarized, not
damaged. Run `xattr -dr com.apple.quarantine /Applications/AuraScribe.app` once, or use **Open Anyway**
in Privacy & Security. Full steps in [docs/INSTALL.md](docs/INSTALL.md).

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes
4. Run `cargo test --manifest-path src-tauri/Cargo.toml` and `npm run typecheck`
5. Open a Pull Request

Cross-platform testing help is especially welcome — if you run the macOS or Linux preview and a model
loads (or doesn't), please open an issue with your `aurascribe.log`.

## 📄 License

MIT — see [LICENSE](LICENSE).

## ❤️ Support AuraScribe

AuraScribe is free and always will be — no tiers, no caps, no account. If it saves you time and you'd
like to support its development, you can sponsor me on GitHub:

[![Sponsor](https://img.shields.io/badge/Sponsor-%E2%9D%A4-ea4aaa?logo=githubsponsors&logoColor=white)](https://github.com/sponsors/JeswinJestin)

## 🙏 Acknowledgments

AuraScribe packages excellent open speech models and runs them locally — full credit to their authors,
used under their respective licenses:

- **[Moonshine](https://github.com/usefulsensors/moonshine)** (Useful Sensors) — the fast English engine
- **[NVIDIA Parakeet](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3)** — 25 European languages
- **[Dolphin](https://github.com/DataoceanAI/Dolphin)** (DataoceanAI / Tsinghua) — ~40 Asian languages
- **[AI4Bharat IndicConformer](https://github.com/AI4Bharat/IndicConformerASR)** — Malayalam & Kannada (CC-BY-4.0)
- **[sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx)** (k2-fsa) — the offline inference runtime, with model exports by [@csukuangfj](https://huggingface.co/csukuangfj)

And the tooling that makes the app:

- [Tauri](https://tauri.app/) — desktop framework · [Next.js](https://nextjs.org/) — React framework
- [enigo](https://github.com/enigo-rs/enigo) / [arboard](https://github.com/1Password/arboard) — cross-platform keystroke + clipboard injection
- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) / [whisper-rs](https://github.com/tazz4843/whisper-rs) — the internal fallback engine (compiled in; no models shipped)

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/JeswinJestin/AuraScribe/issues)
- **Discussions**: [GitHub Discussions](https://github.com/JeswinJestin/AuraScribe/discussions)

If you find this useful, consider giving it a star ⭐
