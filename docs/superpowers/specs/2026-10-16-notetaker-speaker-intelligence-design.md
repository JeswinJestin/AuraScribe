# Notetaker speaker intelligence — design

**Date:** 2026-10-16
**Status:** approved to build (owner said "write it up and start building")
**Builds on:** `docs/superpowers/specs/2026-10-13-notetaker-design.md` (the Notetaker MVP)

## Intent and why

The Notetaker produces a flat, speaker-less transcript today. The owner wants it to be
**more intelligent than Wispr Flow**: understand who is speaking, label each line by speaker,
and — once confident — replace "Speaker N" with the person's real name, **all while the meeting
is still running** (online, not post-hoc), and eventually stream text **word by word**.

The motivating evidence is a real Wispr transcript (a Lenny/Teresa podcast) the owner pasted. It
shows Wispr's actual behaviour and its failure modes, which define our bar:

- It labels `Teresa`, `Lenny`, and `Speaker 3`. Names are **not** guessed from the first words —
  they are attached to voice clusters once the conversation *names* a person, then applied
  retroactively to every line of that cluster.
- `Speaker 3` ("Thank you so.") never got a name: one ambiguous line, no naming evidence → it
  stayed generic. **No evidence → no name. Never a random guess.**
- Wispr figured the speakers out **mid-meeting** (the owner confirmed: the recording was just
  interrupted partway; it had already labelled live, it did not wait for the end).
- Where Wispr *failed*, and where we aim to beat it: it **duplicated** a whole paragraph (the
  marshmallow passage appears twice), printed **"Thanks, William"** (wrong name, surfaced anyway),
  and invented a spurious third speaker.

## Our structural advantage

We capture the **microphone** and the **system/loopback** audio in **two separate buffers**
(`MeetingState.mic_buf`, `MeetingState.system_buf`) and only mix them at transcription time. Wispr
works from one already-mixed stream; we do not. Therefore:

- **"You" is free and essentially perfect.** Any window whose energy is dominated by the mic
  channel is the local user; the loopback channel is everyone else in the call. This only holds on
  **headphones** (the owner's scene) — on speakers the mic also hears the room, so we detect that
  case and fall back to treating mic audio as just another acoustic source.
- We only have to acoustically diarize the **guest (loopback)** stream into Speaker 1/2/3…, which
  is a strictly smaller problem.

## Goals

1. **Online speaker labels.** During the meeting, each transcript line is attributed to a speaker:
   `You`, or `Speaker 1..N` for guests, appearing and growing live as new voices are heard.
2. **Online name resolution.** When the conversation names a speaker with enough confidence, their
   cluster is relabelled to that name **live, retroactively** across all their lines.
3. **Self-correction.** Periodic re-clustering fixes drift (a voice that split into two labels, or
   two near-identical voices that merged) while the meeting runs.
4. **Editable everywhere.** Every speaker label is click-to-edit (rename a speaker → all their
   lines update). Matches Wispr; covers the cases our heuristics get wrong.
5. **Speaker-grouped transcript UI** in both the floating panel and the in-app Notetaker view,
   matching the pasted Wispr layout: a coloured speaker name heading above that speaker's grouped
   message bubbles.
6. **Resume a past meeting** — reopen a finished meeting and continue recording into it (appends
   new segments, diarization continues from the existing speaker set).
7. **Word-by-word streaming transcript** — the last layer; partial hypotheses update a line as the
   speaker talks, instead of a finished line every ~6s.

## Non-goals / anti-goals

- **No cloud, ever.** Diarization, embeddings, naming, streaming — all on-device. The only network
  call in the app remains the one-time model download. (Owner approved an *extra* local model
  download for the speaker-embedding extractor, same pattern as the ASR models.)
- **No faked accuracy.** Online diarization is genuinely hard; we do not claim flawless speaker
  separation. We make it good, self-correcting, honest about confidence, and always editable. A
  label we are not sure of stays `Speaker N`, never a guess. **Surfacing a wrong name is worse
  than surfacing none** — the "Thanks, William" failure is the thing to avoid.
- Not touching the dictation hot path. Meetings keep their own state/threads/chunker.
- No diarization of the *mic* channel into multiple people (one mic = one local user = `You`).

## Constraints

- Windows-only capture for now (unchanged — loopback is WASAPI).
- Lightweight: the embedding extractor is downloaded on first use, not bundled; clustering and
  naming are pure Rust with no new heavy dependency.
- Never edit an applied migration — add `014_meeting_speakers.sql`.
- Secondary windows, IPC-through-`ipc.ts`, `emit_status`, `Err`-on-failure rules all still apply.

## Architecture — the four layers

Build order is **A → B → C → D**. A+B+C deliver speakers+names (the owner's "start here"); D
(streaming) is last.

### Layer A — channel attribution ("You" vs "Guest"). Zero download. Ships first.

Each ~6s window is already drained from `system_buf` and `mic_buf` separately before they are
mixed. Compute the RMS energy of each channel for the window:

- mic energy ≫ system energy → `You` (cluster 0).
- system energy ≫ mic energy → a guest (hand off to Layer B for which guest; until B exists,
  everyone-not-you is `Speaker 1`).
- both significant (overlap, or speakers-not-headphones) → attribute to the louder, mark the
  window "mixed" so B can down-weight it.

Pure function `attribute_channel(mic_rms, sys_rms) -> Channel { Self, Guest, Mixed }` with a
ratio threshold + a silence floor. Unit-tested. `You` from this path is treated as ground truth
and is **not** overridden by acoustic clustering.

### Layer B — online guest diarization (Speaker 1/2/3…). One model download.

For each **guest** window:

1. **Embed.** Run a sherpa-onnx speaker-embedding extractor over the window's guest audio →
   a fixed-length L2-normalized vector. (Downloaded on first meeting that needs it, like ASR
   models; cached in the models dir. Candidate model: a 3D-Speaker / NeMo-TitaNet ONNX export
   that sherpa supports — exact pick validated on-device.)
2. **Assign online.** Cosine-compare to each known cluster centroid. Best match ≥ `JOIN_THRESHOLD`
   → that cluster (update its running centroid). Else → a **new** cluster (new `Speaker K`).
3. **Self-correct every ~30s.** Agglomerative re-cluster all stored embeddings for the meeting
   (average-linkage, cosine, a merge threshold), remap cluster ids to a stable canonical order,
   and emit the corrected labels so the UI relabels live.

Clustering is **pure Rust, fully unit-tested** (cosine, centroid update, online assignment,
agglomerative merge) with synthetic vectors — it needs no model to test. Only step 1 (the actual
embedding extraction) needs on-device verification, exactly like the loopback capture path.

Per-segment we store the **cluster id**, not a frozen label, so a later merge/rename updates every
line at once.

### Layer C — online name resolution. Zero download (text only).

After each window, run a naming pass over the transcript-so-far:

- **Self-introduction** in a cluster's lines → names *that* cluster: `I'm <Name>`,
  `this is <Name>`, `my name is <Name>`, `<Name> here`.
- **Address / introduction** in a cluster's lines → names the cluster being *spoken to*:
  `<Name>, thank you`, `thanks, <Name>`, `hey <Name>`, `welcome <Name>`, `<Name> is a <role>`,
  `author of …`, `please welcome <Name>`. In a 2-party stretch the addressee is "the other
  active cluster"; with 3+ active it is lower confidence.
- **Candidate extraction:** capitalized tokens that are not sentence-initial and not in a
  stopword/common-word list (so `Netflix`, `Monday` never become a speaker). Fuzzy-normalize
  spelling variants (reuse `expand::levenshtein_bounded`: `Theresa`≈`Teresa`).
- **Confidence gate:** assign a name to a cluster only on **≥2 consistent, unconflicted** naming
  events (or 1 strong self-intro). Conflicting evidence of similar strength → stay `Speaker N`.
  When it crosses the bar, relabel live and emit the rename.

Pure Rust, unit-tested against the owner's real pasted transcript as a fixture (expect: Teresa and
Lenny resolved; the lone "Thank you so." line **not** named; "William" rejected as a one-off
conflicting with the established host name).

### Layer D — word-by-word streaming. Last.

Swap the guest/you transcription from "finish a window every 6s" to an **online** recognizer that
emits partial hypotheses, so the current line grows word by word and finalizes on endpoint. This
is a different sherpa recognizer (streaming zipformer transducer) from the current offline
Moonshine/Parakeet, so it is isolated as its own engine behind a setting and built/verified last.
Diarization (A–C) operates on finalized segments regardless of streaming vs chunked.

## Data model — `014_meeting_speakers.sql`

```sql
-- Per-meeting speaker clusters and their resolved display label.
CREATE TABLE IF NOT EXISTS meeting_speakers (
    meeting_id  INTEGER NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    cluster     INTEGER NOT NULL,          -- 0 = You (self/mic); 1..N = guests
    label       TEXT    NOT NULL,          -- "You" / "Speaker 1" / a resolved or edited name
    is_self     INTEGER NOT NULL DEFAULT 0,
    is_named    INTEGER NOT NULL DEFAULT 0, -- 1 once a real name is resolved or user-edited
    PRIMARY KEY (meeting_id, cluster)
);

-- Attribute each segment to a cluster (not a frozen label) so a rename/merge updates all lines.
ALTER TABLE meeting_segments ADD COLUMN speaker_cluster INTEGER NOT NULL DEFAULT -1; -- -1 = unknown
```

`-1` keeps every existing row valid and renders as "Speaker" until re-processed. The UI resolves a
segment's display name by joining `speaker_cluster` → `meeting_speakers.label`.

## Events / IPC

- `meeting-segment` payload gains `speaker_cluster`.
- New `meeting-speakers` event: the full `{cluster, label, is_self}` list for the active meeting,
  emitted whenever clusters or names change (new speaker, 30s re-cluster, name resolved). The panel
  and in-app view keep a cluster→label map and re-render all lines from it — that is how a line
  flips from "Speaker 1" to "Teresa" retroactively and live.
- New commands: `rename_speaker(meeting_id, cluster, label)` (sets `is_named=1`, user edit wins and
  is never overwritten by the heuristic), and `resume_meeting_recording(meeting_id)` for Layer /
  goal 6 (reopen a finished meeting and continue capture into it).

## UI

Both surfaces render the transcript **grouped by speaker turn**, matching the pasted Wispr layout:
a coloured speaker-name heading, then that turn's message bubble(s). Colours come from a small
fixed palette keyed by cluster (You = primary; guests cycle through distinct, theme-aware hues), not
random per render. Speaker headings are click-to-edit (→ `rename_speaker`). A past meeting shows a
**Resume** control that reopens and continues it.

- Panel (`src/app/panel/page.tsx`) and `NotetakerView.tsx` share the grouping + colour logic —
  factor it into a small helper so the two surfaces stay identical (Operate-mode consistency).
- Keep the existing "You / Guest" live feel; names swap in when resolved, with no layout jump.

## Copyright / independence

Nothing is copied from Wispr Flow — not code, not assets, not text. The pasted transcript is used
only to *understand observed behaviour and derive our own heuristics*; our naming rules, clustering,
channel split, and UI are written from scratch and documented here. The channel-split "You" and the
periodic self-correction are our own ideas, not Wispr's.

## Build order (each its own commit; TDD on the pure pieces)

1. `014_meeting_speakers.sql` + DB methods (`upsert_speaker`, `list_speakers`, `rename_speaker`,
   segment `speaker_cluster`). 
2. **Layer A**: `attribute_channel` + wire per-window channel RMS into the chunker; store
   cluster 0 vs 1; emit `speaker_cluster` + a `meeting-speakers` list. Tests.
3. **Layer C naming** (pure engine + tests against the pasted transcript) wired to run each window.
4. **UI**: speaker-grouped, coloured, editable transcript in panel + NotetakerView; live relabel.
5. **Layer B**: online clustering (pure + tests) → then the sherpa embedding extractor + model
   download (on-device verification).
6. **Resume past meeting** (goal 6).
7. **Layer D**: streaming recognizer (isolated, verified last).

Layers A, C, and the clustering math of B are fully sandbox-verifiable. The embedding extraction
(B step 1) and streaming (D) need the owner's hardware, and will be labelled unverified until then,
per the project's honesty rule.

## Left to the builder

- Exact RMS ratio / silence-floor thresholds for Layer A (tune on-device; sensible defaults first).
- Exact cosine `JOIN_THRESHOLD` and agglomerative merge threshold (defaults from literature; tune).
- The specific sherpa embedding model + its download URL/size (validate it loads on-device).
- Guest colour hues (theme-aware, from the existing palette).
