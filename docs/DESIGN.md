# AuraScribe — Design system

The visual direction, written down so it stays consistent as features are added.

## Direction: "instrument"

AuraScribe is a piece of audio equipment, not a dashboard.

That framing comes from the subject itself — this is a tool that reads a signal off a
microphone — and it does the work of keeping the interface honest: an instrument reports
state plainly, doesn't decorate, and stays quiet until something is happening.

It is deliberately **not**:

- a warm cream + serif + terracotta layout (the current default "editorial" AI look)
- a near-black page with one acid-green accent
- a clone of Wispr Flow, which owns warm cream + deep pine green

## Colour

Colour means state. It is never decoration.

| Token | Light | Dark | Means |
|---|---|---|---|
| `--background` | `#FAFBFC` | `#0E1116` | page |
| `--card` | `#FFFFFF` | `#171B22` | panel |
| `--border` | `#E2E7EC` | `#242A33` | hairline |
| `--primary` | `#0B7C8A` | `#3ECEDE` | **signal-cyan — ready / active** |
| `--record` | `#E03B45` | `#F0555C` | **recording** |
| `--standby` | `#C4780F` | `#E8A33D` | **transcribing** |

Three states, three colours, and cyan is reserved for "live". If a new element wants a
colour just to look interesting, it doesn't get one.

Dark is a true cool graphite, blue-shifted rather than neutral grey, so panels read as
anodised metal instead of flat charcoal.

## Type

- **UI:** Inter (already bundled via `next/font`).
- **Technical readouts:** the system monospace stack (`ui-monospace, SF Mono, Menlo,
  Consolas`). No extra webfont is downloaded — a deliberate nod to the "lightweight"
  principle.

Monospace is used for things a machine reports: hotkey combos, model ids, file sizes,
counts, durations, timestamps. Prose stays in Inter. This split is what makes the app read
as an instrument rather than a webpage, and it's also what the PRD asked for.

Numerals use `tabular-nums` so figures don't jitter as they update.

## Layout

```
┌──────────────┬────────────────────────────────────┐
│ AuraScribe   │                                    │
│ ON-DEVICE    │        [ signal meter ]            │
│              │                                    │
│ ▸ Dictate    │             ( ● )                  │
│   History    │      Press Ctrl Shift Space        │
│   Words      │                                    │
│   Snippets   │   ┌─────────┐  ┌─────────┐         │
│   Insights   │   │  Tap    │  │  Hold   │         │
│   Settings   │   └─────────┘  └─────────┘         │
│              │                                    │
│ ● Ready      │                                    │
│ base.en      │                                    │
└──────────────┴────────────────────────────────────┘
```

- Sidebar is a fixed 212px. Navigation is a flat list — no groups, no collapsing. Six
  destinations don't need hierarchy.
- The **status rail** pinned to the bottom of the sidebar always shows the live state and
  the loaded model. Equipment tells you what it's doing without being asked.
- Content is centred with a max width so panels don't stretch into unreadable lines.
- Radius is `10px` throughout — precise, not bubbly.
- No gradients, no glass, no drop shadows, no hover-lift. Borders and background do the
  work.

## Signature: the signal meter

The one memorable element, and the only place with real motion.

- **Idle** — a flat row of dim ticks. The instrument is powered but reading nothing.
- **Listening** — ticks rise and fall in red, staggered so it reads as a waveform rather
  than a mechanical sine.
- **Transcribing** — ticks give way to a single amber band sweeping across.

It answers the only question that matters mid-dictation — *is this thing hearing me?* —
without words, and it earns its motion by being functional. Everything else in the
interface stays still.

## Writing

- Say what the person controls, not how it's built: "Voice model", not "Whisper model
  backend"; "Words", not "Dictionary entries table".
- Buttons name their outcome and keep that name through the flow: **Install** → *Getting…*
  → **In use**.
- Empty states invite action instead of shrugging: "No words yet — add a term above and
  AuraScribe will correct it in every dictation."
- Errors state what happened and what to do. They never apologise and are never vague.
- Sentence case everywhere. No exclamation marks.

## Accessibility floor

Non-negotiable, and not announced in the UI:

- Visible focus rings via `:focus-visible` (2px, accent colour).
- `prefers-reduced-motion` disables the meter animation and all transitions.
- Status is conveyed by text and shape as well as colour — the sidebar rail names the
  state, the meter carries an `aria-label`.
- Toggles are real `role="switch"` buttons with `aria-checked`.
- Light-mode cyan is darkened to `#0B7C8A` so it holds contrast on white.
