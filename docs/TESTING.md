# Testing AuraScribe

A practical guide to checking the app actually works — written for the owner, not for CI.

## 0. How to actually run it

There are exactly two correct ways, and they are for different jobs. Mixing them up is the
cause of most "it looks broken" reports.

### Mode A — Installed app (use this to *test the product*)

This is what you want for daily-use trials and for giving feedback. No terminal stays open,
no dev server, no class of bug that only exists in development.

```bash
build.bat
```

Same wrapper idea as `dev.bat`: it sets up MSVC, libclang, and CMake, then runs
`npm run build`. Use it rather than `npm run build` directly, for exactly the reason below.

Then run the installer it produces:

```
src-tauri\target\release\bundle\nsis\AuraScribe_1.0.0_x64-setup.exe
```

After installing once, AuraScribe is a normal Windows application:

- Launch it from the **Start Menu** like any other app.
- It lives in the **system tray**. Left-click the tray icon to open the window.
- It keeps running when you close the window. Tray → **Quit** to actually exit.
- To test again later, just launch it from Start. **You do not rebuild to use it** — only
  when you want new code.

Rebuild (`build.bat`) and re-run the installer over the top whenever you want to pick up
changes. It upgrades in place; settings, history, and models are kept.

### Mode B — Dev mode (use this while *changing code*)

```bash
dev.bat
```

Hot-reloads the UI as you edit. Keep the terminal open — closing it kills the app. Rust
changes trigger a recompile and relaunch automatically; frontend changes appear instantly.

This mode is for development only. Report bugs from **Mode A** unless the bug is in code you
are actively editing.

### Which to use

| You want to… | Use |
|---|---|
| Try it for a day and report what's wrong | **Mode A** — installed app |
| See a code change you just made | **Mode B** — `dev.bat` |
| Check the thing users will actually get | **Mode A** |
| Check performance / real speed | **Mode A** (release build; debug is much slower) |

### Why not `npm run dev` or a bare `npm run build`

Both are the right commands in the wrong environment. whisper-rs
compiles whisper.cpp from source and needs MSVC, libclang, and CMake on `PATH`. `dev.bat`
and `build.bat` are small wrappers that set those up and then run the same command — that is
the only difference, and it is the difference between a build and a wall of linker errors.

The specific failure is `Unable to find libclang`, from `whisper-rs-sys`'s bindgen step.
Always use `dev.bat` / `build.bat` on Windows.

### Why not the debug binary on its own

Do not run `src-tauri/target/debug/aurascribe.exe` directly. Debug builds load their UI from
the dev server at `localhost:1420`; with no dev server running, every window shows
*"localhost refused to connect"* — most visibly the always-on-top overlay, which then looks
like a stuck error box. The same happens to `target/release/aurascribe.exe` after
`cargo test --release`, which rebuilds it without embedding the frontend.

As of Round 5 the overlay refuses to appear unless its page confirms it loaded, so this
should no longer put a box on your screen. If one ever does:

```bash
taskkill /IM aurascribe.exe /F
```

### If you see "404 — This page could not be found"

That was a real bug, fixed in Round 5: the overlay window requested `overlay/index.html`,
which the dev server 404s on. If it reappears, you're running a binary built before the fix
— rebuild. It is never a sign that something is wrong with your machine.

## 1. First-run setup

**Check the window size first.** It should open at **1480x936 logical, centred**, without
you touching it. On a display smaller than that, it shrinks to fit the screen and re-centres
(`fit_to_screen` in `main.rs`) rather than opening with its own controls off the edge.

If it opens small and you find yourself dragging it bigger, that's the Round 5b bug — you're
on a build from before the fix, or `tauri-plugin-window-state` has been re-added. Resizing
the window by hand is never something you should have to do.


1. Launch the app. The settings window **opens by itself** when no model is installed.
2. Go to **Settings → Whisper Model**.
3. Click **Download & Use** on `large-v3-turbo-q5_0` (~574 MB, recommended). Pick
   `base.en` (~142 MB) instead if you'd rather not wait on the download.
   - It downloads once, then loads automatically. A progress bar shows during download.
   - After this, it works offline forever — free, no account.
4. Go back to **Home**. It should say **Ready**.

If Home still says setup is needed, the load failed and the error is now shown in red in
the Settings model section. Check `%LOCALAPPDATA%\AuraScribe\models\` — you should see
`ggml-base.en.bin` at roughly the size listed.

### Confirming a model is actually active

Three places now show it, and all three read the same authoritative value from the backend:

1. The sidebar status rail says **Ready** with the model name beneath it.
2. The Dictate screen shows the model id under the hotkey hint.
3. Settings marks that model **In use**.

If those disagree with each other, that's a bug worth reporting — they are all driven by
`Status.loaded_model`, which reports what is genuinely in memory rather than what was last
saved.

### What is a "Whisper model"?

It's the speech-recognition file that turns your voice into text. Whisper is OpenAI's
open-source speech model; `.bin` files are the offline weights. Downloading one is like
installing a dictionary — a one-time cost, after which nothing leaves your machine.

| Model | Size | Notes |
|---|---|---|
| `tiny.en` | ~75 MB | Fastest, noticeably less accurate |
| `base.en` | ~142 MB | Good balance if you want a small download |
| `large-v3-turbo-q5_0` | ~574 MB | **Recommended** — near-`large-v3` accuracy at a fraction of the runtime |
| `large-v3-turbo` | ~1.6 GB | Same, unquantised |
| `large-v3` | ~3.1 GB | Most accurate, slowest |

`small.en` and `medium` were deliberately removed in Round 3: `large-v3-turbo-q5_0` is
smaller, faster *and* more accurate than `small.en`, so keeping them would only offer a
strictly worse choice.

## 2. Finding the app

AuraScribe is a **tray app**, not a normal window — that's intended (the PRD calls for "no
dock icon clutter, no persistent window").

- The icon lives in the system tray, bottom-right. Windows often hides new tray icons
  behind the **`^` chevron** — click it, then drag AuraScribe's icon onto the visible tray.
- **Left-click** the tray icon to open Settings. **Right-click** for a menu with Quit.
- Closing the settings window hides it; the app keeps running. Use tray → **Quit** to exit.

## 3. Basic dictation test

1. Open **Notepad** and click in it so the cursor is blinking.
2. Press **Ctrl+Shift+Space**.
   - The tray icon turns **red**, and a small **"Listening…"** pill appears near the bottom
     of the screen.
3. Say: *"this is a test of local speech recognition"*
4. Press **Ctrl+Shift+Space** again to stop.
   - The indicator switches to amber **"Processing…"** for a moment.
5. Cleaned, punctuated text should be typed at your cursor.

Default mode is **toggle** (press to start, press again to stop). Switch to **Hold** on the
Home screen if you prefer push-to-talk.

### If the hotkey does nothing

`Ctrl+Space` — the old default — is commonly stolen by Windows IME for input-language
switching, so it can silently fail. The default is now `Ctrl+Shift+Space`. If yours is
still contended, rebind it: **Settings → Hotkey → Combination**, click, and press your new
combo.

You can also test without the hotkey entirely: use the big **mic button** on the Home
screen. If that works but the hotkey doesn't, the hotkey is being intercepted by another
app.

## 4. Testing across applications

Text is injected as real keystrokes (`SendInput`), so it works anywhere that accepts
typing. Worth trying:

| App | What it proves |
|---|---|
| Notepad | Baseline |
| Chrome address bar / a textarea | Browser input works |
| VS Code | Code editors work |
| Word / Outlook | Office apps work |
| WhatsApp / Slack / Discord | Chat apps work |

**Known limitation:** apps running **as Administrator** reject synthetic input from a
non-elevated process. AuraScribe detects this, copies the text to your clipboard instead,
and tells you so — paste with `Ctrl+V`. To dictate into elevated apps, run AuraScribe as
Administrator too.

## 5. Verifying the privacy claim

The strongest test of "local-first" is simple:

1. Make sure a model is downloaded and loaded.
2. **Turn off Wi-Fi / unplug the network.**
3. Dictate.

It should work exactly as before. If dictation ever fails without a network, something has
regressed against the core promise — treat it as a serious bug.

## 6. Automated tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Covers the cleanup rules and audio resampling. The real end-to-end transcription test needs
a model and a sample WAV, so it's `#[ignore]` by default:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test transcription -- --ignored --nocapture
```

Set `AURASCRIBE_TEST_WAV` to a 16 kHz mono WAV first. You can generate one with Windows TTS
via PowerShell (`System.Speech.Synthesis.SpeechSynthesizer` → `SetOutputToWaveFile`).

## 7. Reading the logs

The app logs to stdout. To see them, run it from a terminal:

```bash
cd src-tauri
```

```bash
$env:RUST_LOG="aurascribe=debug"; .\target\release\aurascribe.exe
```

Useful lines: `Auto-loaded model at startup`, `Loading Whisper model`,
`Audio capture stopped (48000Hz, 2ch)`, and any `Failed to …` error.

## 8. The feedback loop

The point of installing it (Mode A) is to use it for real and notice what's wrong. To make a
report actionable, capture these four things:

1. **Which mode** — installed app, or `dev.bat`? They fail differently.
2. **What you expected vs what happened.** "The window opened small and I had to drag it" is
   a great report; "the UI is off" is not actionable.
3. **A screenshot**, if it's visual. Window size, position, and layout bugs are almost
   impossible to describe in words and obvious in one image.
4. **The log**, if something errored — see §7.

### Measuring instead of eyeballing

Window geometry claims should be measured, not guessed. This prints the real rect of every
AuraScribe window, which settles "is it the right size?" in one command:

```bash
powershell -Command "Get-Process aurascribe | ForEach-Object { $_.MainWindowTitle }"
```

The fuller version used in Round 5c enumerates every top-level window with its size and
position via `GetWindowRect`. Reach for it whenever a sizing claim needs evidence — reading
`tauri.conf.json` proved twice to be worthless for this, because a plugin was overriding it.

### A good iteration cycle

1. `build.bat`, install, use it for a real task.
2. Note anything that made you stop and think, or that you had to correct by hand.
3. Report with the four items above.
4. Fix, rebuild, reinstall over the top. Settings and models are preserved.

## 9. Resetting

To start completely fresh, quit the app and delete:

```
%LOCALAPPDATA%\AuraScribe\
```

That removes settings, dictation history, and downloaded models. Models will need
re-downloading.
