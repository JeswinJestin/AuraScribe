# Notetaker (local meeting transcription) — design

**Date:** 2026-10-13 · **Status:** Phase 1 in progress (backend foundation + data model + minimal UI)
· **Owner decision:** build a **local-capture MVP now** (recording → transcript, **no cloud AI
summary**), phased, verified on-device.

## Why this exists / the charter conflict, resolved

`CLAUDE.md` names **meeting transcription as the #1 v1 non-goal** and the biggest scope-creep risk.
The owner has decided to build it anyway — but **only the parts that stay true to the
non-negotiables**. This section is the contract that keeps it from becoming "Wispr Flow, but
breaking our own product."

| Wispr Flow does | Can we? | Decision |
|---|---|---|
| Cloud STT of the meeting | **No — never a cloud call** | Transcribe **locally** with the engines we already ship. |
| Cloud LLM summary / "ask anything" / action items | **No — no cloud, and a local summarizer is multi-GB (breaks ~4.6 MB / lightweight)** | **Out of scope.** MVP produces a faithful **transcript**, not an AI summary. A local summary is a *possible later* feature **only** if the already-optional `prompt` LLM (Qwen2.5, opt-in download) is present — never bundled, never default, never cloud. |
| Calendar connect, bot that joins calls, sharing, team | Cloud/account features | **Out of scope** (v1 non-goals: team, cloud, accounts). |
| Record **system audio** (what you hear in the call) + **your mic** | **Yes, locally** (WASAPI loopback) | **In scope** — this is the core of the MVP. |

**One-line scope:** a local button/hotkey that records **the meeting you hear (system output) +
your microphone**, transcribes it **on-device** with the installed model, and saves a timestamped
**transcript** you can read, copy, and delete. No cloud, no account, no bundled LLM.

## The hard technical facts (verify by running — not yet verified on hardware)

1. **Capturing "what you hear through headphones" = WASAPI loopback.** You cannot capture remote
   call audio from the mic; it only exists in the system *output* stream. On Windows that is
   **WASAPI loopback** (`IAudioClient::Initialize` with `AUDCLNT_STREAMFLAGS_LOOPBACK` on the
   render endpoint). `cpal` 0.15 does **not** expose loopback, so we drive WASAPI through the
   **`windows` crate already in `Cargo.toml`** (enable the `Win32_Media_Audio` feature) rather than
   adding a new dependency — this respects "weigh every dependency."
2. **Two sources must be mixed.** The transcript should contain *both* sides: the remote
   participants (loopback) and the local user (mic). MVP mixes both to one mono 16 kHz stream and
   transcribes the mix. **Speaker diarization (who said what) is NOT in the MVP** — it needs a
   separate model and is a documented later phase. We will *not* fake speaker labels.
3. **Long-form accuracy is a real limitation.** The shipped engines (Moonshine/Parakeet/Dolphin)
   are tuned for **short dictation**, not hours of meeting audio. The MVP reuses the existing
   `run_chunker` windowed pipeline to keep latency and memory bounded, but **accuracy on long,
   multi-speaker, cross-talk audio will be worse than single-speaker dictation.** This is stated
   plainly in the UI and release notes. A better long-form model (e.g. a Whisper large/distil tier
   for meetings, GPU-optional) is the honest accuracy path and is a later phase.
4. **Loopback format differs from mic.** The render endpoint's mix format is typically 48 kHz
   stereo float; the mic may be 44.1/48 kHz, i16/f32. Both are converted to mono f32 and resampled
   to 16 kHz by the existing `audio::resample_linear` before ASR, same as dictation.
5. **macOS/Linux loopback is NOT in the MVP.** macOS has no public loopback API without a virtual
   device (e.g. BlackHole) or a system-extension; Linux needs PulseAudio/PipeWire monitor sources.
   Per the "don't fake success on unimplemented platforms" rule, `start_meeting` returns an
   **explicit "system-audio capture is Windows-only for now"** error off Windows. Mic-only meeting
   capture may be offered cross-platform later.

## Architecture (reuses the proven dictation pipeline)

```
Notetaker "Start"  ─→ meeting::start_capture
                         ├─ WASAPI loopback (render endpoint)  ┐
                         └─ cpal mic input (existing path)     ┴─→ mix → mono f32 @ native rate
                                                                       │
                              (same as dictation from here down)       ▼
                                   audio::resample_linear → 16 kHz
                                   run_chunker (windowed, live) → asr::transcribe → cleanup
                                                                       │
                                   db: meeting_segments (ts, text)  ◀──┘
Notetaker "Stop"   ─→ finalize meeting row (duration, word count)
```

- **Isolation rule:** all meeting code lives in `meeting.rs`. It must **never** touch the
  dictation hot path (`start_recording`/`stop_recording`/`run_chunker` stay byte-identical). A bug
  in meeting capture cannot regress daily dictation.
- **Status:** meetings use their own state (`MeetingState`), not the dictation `Status`/overlay, so
  the tray/overlay single-status-path rule is unaffected. (A later phase may add a recording
  indicator.)

## Data model — migration `011_meetings.sql`

```sql
CREATE TABLE IF NOT EXISTS meetings (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    title       TEXT    NOT NULL DEFAULT 'Untitled meeting',
    started_at  INTEGER NOT NULL,          -- unix seconds
    ended_at    INTEGER,                   -- null while recording
    duration_ms INTEGER NOT NULL DEFAULT 0,
    word_count  INTEGER NOT NULL DEFAULT 0,
    source      TEXT    NOT NULL DEFAULT 'system+mic'  -- what was captured
);
CREATE TABLE IF NOT EXISTS meeting_segments (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id  INTEGER NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    offset_ms   INTEGER NOT NULL,          -- ms from meeting start
    text        TEXT    NOT NULL,
    created_at  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_meeting_segments_meeting ON meeting_segments(meeting_id, offset_ms);
```
Transcript text is cheap (the 2026-08-27 storage audit measured all dictation history at <1 MB);
no audio is persisted in the MVP — only text. (Keeping the raw audio is a later, opt-in choice.)

## Commands (new; registered in `main.rs`)

- `start_meeting(title?) -> meeting_id` — begins capture; errors clearly off Windows.
- `stop_meeting() -> ()` — stops capture, finalizes the row.
- `list_meetings() -> Vec<MeetingRow>` — for the Notetaker list.
- `get_meeting(id) -> { meeting, segments }` — for the detail view.
- `delete_meeting(id) -> ()` — removes a meeting + its segments.
- `meeting_status() -> { recording, meeting_id, elapsed_ms }` — UI polls this (events are the fast
  path, not the only path — the CLAUDE.md rule).

## Phases

- **Phase 1 (this session):** spec; migration + DB methods; `meeting.rs` with WASAPI loopback +
  mic mix (compiles; pure mix/segment logic unit-tested; COM path **unverified — needs a real run**);
  commands + registration; a minimal **Notetaker** sidebar view (start/stop, list, read, delete) +
  IPC. Compiles, `cargo test` green, `tsc` green.
- **Phase 2 (owner-verified):** owner runs a real call, confirms loopback+mic are captured and the
  transcript is faithful; `aurascribe.log` shows the capture format. Fix whatever the first run
  reveals (format negotiation, device selection, gain). Add a recording indicator + live transcript.
- **Phase 3 (accuracy + polish):** evaluate a dedicated long-form meeting model (Whisper distil/large,
  GPU-optional), optional raw-audio retention, search, export. Optional local summary **only** via the
  already-opt-in `prompt` LLM — never bundled, never cloud. Release.

## What this design will NOT do (so scope stays honest)

- No cloud call of any kind (unchanged non-negotiable).
- No bundled summarization LLM; no "ask anything"/action-items in the MVP.
- No fake speaker labels, no fake accuracy claims, no "works on macOS/Linux" until it does.
- No change to the dictation hot path.
