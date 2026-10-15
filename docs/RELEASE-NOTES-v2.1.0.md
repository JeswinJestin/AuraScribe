# AuraScribe v2.1.0

Free, open-source, **local-first** voice dictation. Hotkey → speak → clean text at your cursor, in
any app. 100% on-device — no cloud, no account, no telemetry.

## ✨ New: Notetaker (local meeting transcription)

Record a meeting and get a transcript — entirely on your device. Notetaker captures **what you hear in
the call (system audio)** plus **your microphone**, and transcribes it locally with the model you
already use for dictation.

- **Real-time transcript** that builds up live while you record.
- **Local summary** — key points + keywords, extracted on-device (no model download, no cloud, and it
  can't make things up).
- **Editable meetings** — rename a meeting, add your own **notes**, and click any transcript line to
  edit it. Meetings auto-name themselves from the first thing said.
- **No cloud, no account, no AI-summary upload** — a transcript that stays on your machine.

> System-audio capture is **Windows-only** in this release. Long, multi-speaker meetings are harder
> than short dictation — expect more mistakes than when you dictate solo.

## 🐛 Fixed: garbled dictation in Windows 11 Notepad

Short dictations could come out as runs of `......` / `?????` **only in the modern Windows 11
Notepad** (other apps were fine). Cause: Notepad's rewritten text control mishandles the
character-by-character input AuraScribe used for short phrases. **AuraScribe now pastes every
dictation on Windows**, which lands cleanly everywhere, including Notepad. Verified end-to-end against
the real Windows 11 Notepad.

## 🎯 Accuracy: fuzzy correction of your Words (opt-in)

A new **"Fuzzy-correct my Words"** setting (off by default) also fixes *near-miss* mishears of your
personal dictionary terms — e.g. "kuberneties" → "Kubernetes" — not just exact matches.

## Install

- **Windows:** download `AuraScribe_2.1.0_x64-setup.exe` and run it.
- macOS / Linux: see `docs/INSTALL.md`. (Dictation on those platforms remains a preview — the Notetaker
  system-audio capture is Windows-only for now.)

Everything runs locally. The only network request the app ever makes is the one-time voice-model
download.
