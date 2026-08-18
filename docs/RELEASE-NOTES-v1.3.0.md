# AuraScribe v1.3.0

Free, open-source, 100% on-device voice dictation. Press a hotkey, speak, and clean text appears
where your cursor is — in any app. Nothing leaves your machine except the one-time model download.

## ✨ What's new

- **A brand-new interactive walkthrough.** First launch now *shows* you how AuraScribe works instead
  of describing it: a short spotlight tour with a little animation of the hotkey being pressed, the
  mic switching on, a voice line, and the text landing at your cursor — with matching sound. Three
  quick steps, skippable at any point, and replayable anytime from **Settings → Hotkey → Replay
  walkthrough**.
- **Per-OS dictation hotkey.** Sensible defaults per platform, avoiding each OS's reserved shortcuts:
  - **Windows / Linux:** `Ctrl + Shift + Space`
  - **macOS:** `Cmd + Shift + Space`
  - If a shortcut can't be registered (another app already owns it), the app now tells you clearly
    instead of silently doing nothing.
- **Turn the hotkey off.** New **Settings → Hotkey → "Enable the dictation hotkey"** toggle lets you
  put AuraScribe to sleep without uninstalling — no keypress will trigger dictation until you switch
  it back on.
- **Softer voices are heard.** Quiet recordings are now automatically boosted to a healthy level
  before transcription, so soft-spoken dictation comes through as well as loud dictation. (Only quiet
  audio is amplified — a normal, loud setup is untouched.)
- **Correct window size on every display.** Fixed a bug where the window could open the wrong shape
  (too wide, controls near the edge) on some laptops and high-DPI screens. It now fits any display's
  resolution and scaling.

## 💻 Platform support — read this

| Platform | Status | Dictation works? |
|---|---|---|
| **Windows (x64)** | ✅ **Supported** | **Yes** — the proven, daily-use build |
| **macOS** | 🧪 **Beta** | **Newly implemented** — please test & report (see setup below) |
| **Linux (x64)** | 🧪 **Beta** | **Newly implemented, X11** — please test & report (Wayland limited) |

**Windows is the proven, supported product.** **macOS and Linux dictation is brand-new in this
release** and we need your feedback — it's built and shipped, but hasn't had the years of real-world use
Windows has. If something's off, please open an issue with your OS/version. Two setup notes that are
**required** on macOS, and one on Linux:

- **macOS — grant Accessibility.** The first time you dictate, macOS will ask you to allow AuraScribe in
  **System Settings → Privacy & Security → Accessibility** (this is what lets any app type for you and
  use a global hotkey). Toggle AuraScribe on, then try again. Also, the app is **unsigned**, so the
  first launch needs **right-click → Open** (or `xattr -dr com.apple.quarantine /Applications/AuraScribe.app`).
- **Linux — X11 works best.** Typing into other apps is reliable on **X11**. On **Wayland** (GNOME's
  default) the desktop restricts synthetic input, so dictation may not type into other apps yet — an X11
  session is the safe bet for now.

## ⬇️ Install

**Windows** — download `AuraScribe_1.3.0_x64-setup.exe` and run it.
- It's unsigned, so Windows SmartScreen may warn: click **More info → Run anyway**.
- Nothing else to install — the required runtime is bundled.
- On first launch, download a voice model (the walkthrough points you to it), then press
  `Ctrl + Shift + Space` in any app and talk.

**macOS (experimental)** — open the `.dmg`, drag AuraScribe to Applications. Unsigned, so right-click
the app → **Open** the first time. *(Launches, but dictation is not functional yet.)*

**Linux (experimental)** — `.AppImage`: `chmod +x AuraScribe_1.3.0_*.AppImage && ./AuraScribe_1.3.0_*.AppImage`.
Or `.deb`: `sudo dpkg -i aura-scribe_1.3.0_amd64.deb`. *(Launches, but dictation is not functional yet.)*

## 🔒 Privacy (unchanged, and the whole point)

Everything runs on your device. Audio is transcribed locally; cleanup is plain local text processing.
The **only** network request the app ever makes is downloading a voice model the first time. No cloud
transcription, no telemetry, no analytics, no account.

## 🐞 Known limitations

- **macOS/Linux dictation is not implemented yet** (experimental previews — see above).
- **Noisy-room noise suppression** is planned for a future release. This version improves *quiet-voice*
  pickup; it does not yet actively suppress background noise.
- The Windows build is unsigned (SmartScreen warning on first run).

## 🙏 Feedback

Issues and ideas: https://github.com/JeswinJestin/AuraScribe/issues
