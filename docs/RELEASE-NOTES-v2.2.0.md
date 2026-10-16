# AuraScribe v2.2.0 — Notetaker: local meeting transcription with speaker intelligence

**The headline:** AuraScribe now records meetings — **what you hear in the call *plus* your mic** —
and turns them into a timestamped, **speaker-labelled** transcript, entirely on your device. It works
out *who is speaking* and, once the conversation makes it clear, puts **real names** on them — live,
mid-meeting. No cloud, no account, ever.

> **Windows-only** for now: capturing the other side of a call needs system-audio loopback (WASAPI).
> Dictation remains cross-platform. Uninstall any older AuraScribe before installing.

## 🧑‍🤝‍🧑 Notetaker

- **Records the call + your mic**, mixes them, and transcribes on-device with the same local voice
  model as dictation. A **floating, theme-matched panel** pops up while you record (expand, dock back
  to the side, minimize, or close); the main app returns when you stop.
- **Knows who's speaking.** Because AuraScribe captures your mic and the call audio as *separate*
  streams, **"You" is near-perfect and free**. The call side is split into **Speaker 1 / 2 / 3** with
  an optional voice-fingerprint model, and the whole thing self-corrects as the meeting goes on.
- **Finds real names.** It reads how people address and introduce each other ("thanks, Teresa",
  "I'm Lenny") and — only when it's confident — relabels a speaker to their name, retroactively across
  all their lines. Never a random guess; every label is click-to-edit.
- **Word-by-word streaming** *(optional)* — turn on a small streaming model to watch the transcript
  fill in live as people speak, instead of every few seconds.
- **Editable and yours.** Rename the meeting, edit any line, rename a speaker, jot private notes, and
  generate a **local extractive summary** — all on-device. **Resume** a past meeting to keep recording
  into it. A clean, date-grouped library; open a meeting as a full page.

### Optional on-device add-ons (download on demand, never automatic)

| Add-on | Size | What it adds |
|---|---|---|
| **Separate speakers** | ~40 MB | Splits the call audio into Speaker 1 / 2 / 3 (voice fingerprints). |
| **Word-by-word transcript** | ~45 MB | Live streaming transcription (a lighter model). |

Without them, Notetaker still labels **You** vs **Guest** and detects names — just with one guest
bucket and a few-second cadence.

## 🐛 Fixes & polish

- **The Windows 11 Notepad dictation bug is fixed.** Short dictations that came out as `......` / `?????`
  **only in the new Notepad** are gone — AuraScribe now pastes every dictation (the path that can't
  corrupt injected text), with per-character typing as a fallback.
- Calmer accent colour; theme-matched selection highlight; the floating panel respects your
  light / dark / glass theme and stays legible.

## 🔒 Still 100% local

The only network request the app ever makes is a one-time model download. No cloud transcription, no
LLM calls, no telemetry — the whole reason AuraScribe exists.

## ⚠️ Honest status

- **Dictation** (all platforms) and the **Notepad fix** (Windows) are verified on real hardware.
- The **speaker-separation** and **streaming** engines are proven by tests against real models/audio;
  their *accuracy in a long, messy, multi-speaker meeting* improves with tuning — treat them as strong
  but new. Labels are always editable.
- macOS/Linux remain **dictation previews** (model-loading on those two still being confirmed on real
  devices). Notetaker is Windows-only.

## Install

Grab the installer for your OS from the release assets. **Windows:** run
`AuraScribe_2.2.0_x64-setup.exe` (SmartScreen → *More info → Run anyway*), then
Settings → Voice model → download a model. Full per-OS steps: [docs/INSTALL.md](INSTALL.md).
