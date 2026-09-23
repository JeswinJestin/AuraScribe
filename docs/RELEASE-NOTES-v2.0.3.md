# AuraScribe v2.0.3

Free, open-source, 100% on-device voice dictation. Press a hotkey, speak, and clean text appears
where your cursor is — in any app. Nothing leaves your machine except the one-time model download.

This is a **reliability release**: the window now stays locked to its frame on every OS, and the
audio-capture path is more robust — it works with more microphones and no longer drops parts of a
recording.

## ✨ What's fixed

- **The window stays put.** On the frameless window you could accidentally drag the *contents*
  around inside the frame — the sidebar sliding off one edge, the side panel off the other — which
  looked broken. The app is now pinned to its window: content can't pan or slide, and everything
  stays visible within the frame at any size, on Windows, macOS and Linux.
- **Correct layout at every window size.** The same root cause made the layout sit slightly off or
  break at narrower widths (most visible on Linux). The interface now fits cleanly from the smallest
  supported window (860×560) up to full screen — no clipped edges, no horizontal drift.
- **Works with more microphones.** Some microphones (more common on Linux/PipeWire) report audio in
  an integer format that the previous build couldn't open — so it captured nothing. AuraScribe now
  records from those devices too, and the "microphone blocked?" check no longer trips on them.
- **No more dropped words.** Under load the recorder could silently discard chunks of incoming
  audio, so a recording could come back with only part of what you said. The capture path no longer
  drops audio when it's busy — it holds the samples and writes them the moment it can, so the full
  recording reaches the transcriber. (This most affected Linux; Windows recordings are unchanged.)
- **Better support logs.** The app now records the detected microphone format and capture health in
  `aurascribe.log`, so if dictation ever misbehaves we can pinpoint it from the log instead of
  guessing.

## 💻 Platform support — read this

| Platform | Status | Dictation works? |
|---|---|---|
| **Windows (x64)** | ✅ **Supported** | **Yes** — the proven, daily-use build |
| **Linux (x64, .deb)** | 🧪 **Beta** | **Yes** — verified on real hardware; this release targets the "only some words come through" report |
| **macOS (Apple Silicon)** | 🧪 **Beta** | **Preview** — installs and launches; on-device dictation still needs confirmation (see `docs/INSTALL.md`) |

**Windows is the proven, supported product.** **Linux is beta** — dictation has been verified on real
hardware, and this release specifically targets microphone compatibility and the dropped-audio bug some
Linux users hit. **macOS is a preview**: it installs and launches (bundle ships its libraries,
ad-hoc signed), but on-device dictation still needs confirmation — see the Gatekeeper + permission
steps in `docs/INSTALL.md`.

## 📦 Install

- **Windows:** download `AuraScribe_2.0.3_x64-setup.exe` and run it. Nothing else needed.
- **Linux (Debian/Ubuntu):** download `AuraScribe_2.0.3_amd64.deb`, then
  `sudo apt install ./AuraScribe_2.0.3_amd64.deb` (apt resolves the dependencies; `dpkg -i` does not).
- **macOS:** download the `.dmg`, then follow the Gatekeeper steps in `docs/INSTALL.md`
  (the app is not notarized).

## 🔒 Still 100% local

No cloud, no telemetry, no account — unchanged. The only network request the app ever makes is the
one-time voice-model download.
