# Installing AuraScribe

AuraScribe ships a native installer per platform. Pick your OS below.

> **Signing status (be honest with yourself before installing):** the Windows and macOS/Linux
> builds are **not** signed with a paid code-signing certificate, and the macOS build is **not
> notarized**. That is normal for a free, open-source app — it just means each OS shows a "this is
> from an unidentified developer" warning the first time, which you clear once with the steps below.
> Everything runs 100% locally; the only network request the app ever makes is the one-time voice-model
> download.

---

## Windows (x64) — fully supported

1. Download **`AuraScribe_<version>_x64-setup.exe`** from the release.
2. Run it. If **SmartScreen** shows *"Windows protected your PC"*, click **More info → Run anyway**
   (it appears because the installer is not signed with an EV certificate, not because anything is
   wrong).
3. Follow the installer. Launch AuraScribe, press the hotkey (**Ctrl+Shift+Space** by default),
   speak, and the cleaned text lands at your cursor.

If the app fails to start on a *clean* Windows PC with a `VCRUNTIME140_1.dll` error, you have an old
build — the current installer bundles the Microsoft VC++ runtime beside the app, so re-download the
latest.

---

## macOS (Apple Silicon) — preview

The `.dmg` is built for **Apple Silicon (M1/M2/M3/M4)**. It is ad-hoc signed so it can launch, but it
is **not notarized**, so Gatekeeper will stop it the first time until you explicitly allow it.

### Install

1. Download **`AuraScribe_<version>_aarch64.dmg`** and open it.
2. Drag **AuraScribe** onto the **Applications** folder shown in the disk image.

### First launch — clear Gatekeeper (do this once)

macOS blocks unsigned/un-notarized apps by default. Use **one** of these:

**Option A — Open Anyway (menu, simplest):**
1. Try to open AuraScribe once (double-click). macOS refuses and shows a warning — that's expected.
2. Open **System Settings → Privacy & Security**, scroll to the **Security** section. You'll see
   *"AuraScribe was blocked to protect your Mac."* Click **Open Anyway**.
3. Confirm with your password / Touch ID, then open AuraScribe again → **Open**.

**Option B — Terminal (most reliable, especially on macOS Sequoia 15+ where right-click→Open no
longer bypasses Gatekeeper):**
```bash
xattr -dr com.apple.quarantine /Applications/AuraScribe.app
```
This removes the "downloaded from the internet" quarantine flag so the app opens normally. Run it once
after copying the app to Applications.

### Grant permissions (required for dictation to work)

AuraScribe types into other apps and listens to your mic, so macOS's privacy system (TCC) will ask:

1. **Accessibility** — **System Settings → Privacy & Security → Accessibility** → enable **AuraScribe**.
   This is what lets the app send the keystrokes/paste that place your dictated text, and register the
   global hotkey. Without it, the hotkey and text injection do nothing.
2. **Microphone** — approve the mic prompt on first dictation (or enable it under
   **Privacy & Security → Microphone**).
3. **Input Monitoring** — if the hotkey still doesn't fire, enable **AuraScribe** under
   **Privacy & Security → Input Monitoring** too.

After changing Accessibility/Input Monitoring, **quit and reopen AuraScribe** so it re-registers.

> **Known limit (preview):** the macOS build embeds the sherpa-onnx / ONNX Runtime libraries, but
> **model loading on macOS has not yet been verified on real hardware.** If a downloaded voice model
> fails to load, that's the outstanding item — please report it with the contents of
> `~/Library/Application Support/dev.aurascribe.app/aurascribe.log` (or `~/Library/Logs`), which names
> the exact library that failed so it can be fixed quickly.

### Uninstall
Drag `AuraScribe.app` to the Trash and remove `~/Library/Application Support/dev.aurascribe.app`.

---

## Linux (x64, Debian/Ubuntu) — preview

A **`.deb`** is provided (the AppImage is temporarily dropped from CI — see the release notes).

### Install
```bash
sudo apt install ./AuraScribe_<version>_amd64.deb
```
`apt` pulls the runtime dependencies (`libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libasound2`, `libxdo3`).
On a very fresh system you may need:
```bash
sudo apt-get -f install    # resolve any missing dependencies
```
Then launch **AuraScribe** from your app menu, or run `aurascribe` from a terminal.

### Notes / known limits (preview)
- Dictation uses **X11** keystroke/clipboard injection (via `enigo`/`xdo`). On a **Wayland** session,
  global-hotkey and injection support is limited — log into an **Xorg** session for now if the hotkey
  or text insertion doesn't work.
- **Auto-start on login** is not implemented on Linux yet (Windows only) — start it manually.
- As with macOS, **model loading on Linux is not yet verified on-device.** If a model fails to load,
  the log at `~/.local/share/dev.aurascribe.app/aurascribe.log` names the missing `.so`.

---

## What "preview" means for macOS/Linux

The Windows build is the daily-driver, verified path. The macOS/Linux builds now contain the **full
dictation loop** (global hotkey → record → transcribe → inject) — this is new in **v2.0.0** and is a
big step past the earlier "installs but can't dictate" state. The remaining, honestly-flagged unknown
is whether the bundled speech libraries load on those OSes; that needs a test on real hardware. Until
someone confirms it, treat the non-Windows assets as previews and report the log if a model won't load.
