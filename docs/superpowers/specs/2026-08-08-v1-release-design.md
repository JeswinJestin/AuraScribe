# AuraScribe v1.0.0 — release design

**Date:** 2026-08-08 · **Status:** draft for owner review

## Goal

Ship **v1.0.0**, the first stable milestone. Bundle the accumulated multi-engine work with a
focused set of UX upgrades, then cut a proper, well-documented GitHub release. Nothing is pushed
or released until every sub-project below is built **and verified** (per `CLAUDE.md`: verify by
running, not reading).

## Non-negotiables (unchanged)

- **No cloud calls.** Only permitted network request stays the one-time model download.
- **Verify by running.** Each sub-project ships with evidence (test pass, real behaviour), not
  "the code looks right".
- **`DESIGN.md` is law for UI**: 10px radius, borders + background do the work (no gradients /
  glass / shadows), cyan = "live" only, monospace for machine values, sentence case,
  `prefers-reduced-motion` respected.

## Out of scope (explicit)

- **Configurable hotkey / preset picker** — dropped. Windows `RegisterHotKey` needs a normal key
  with the modifiers, so a bare `Ctrl+Win` can't be a global shortcut. Owner chose to keep the
  single fixed default `Ctrl+Shift+Space`.
- **Kannada HuggingFace upload** — parked; batched with the Malayalam docs PR when the sherpa-onnx
  maintainer replies.

---

## Sub-project 1 — Recommended model = AuraScribe English (Moonshine base)

**Intent.** The "Recommended" badge must land on **AuraScribe English** (`moonshine-base-en`,
286 MB, accuracy 4), the most accurate real-time English model — not **English Mini**
(`moonshine-tiny-en`, 110 MB, accuracy 3).

**Approach.** Recommendation is *computed* by the engine facade (`engine.rs`), not a static flag
(`moonshine.rs` hardcodes `recommended: false` and defers). Confirm the facade's election rule
("most accurate model that still keeps up") actually elects `moonshine-base-en`; if a tie-break or
ordering picks tiny, fix the election so base wins. Lock it with a unit test asserting
`moonshine-base-en` is recommended and `moonshine-tiny-en` is not, on a CPU-only build.

**Files.** `src-tauri/src/engine.rs` (election), `src-tauri/src/asr.rs`/`moonshine.rs` (ranks if
needed) + test.

**Verification.** `cargo test` green with the new assertion; Settings shows the badge on AuraScribe
English.

## Sub-project 2 — Design-system dropdowns

**Intent.** Every dropdown should match the system: rounded (10px), bordered, themed popup — not the
plain OS-drawn list it shows today.

**Approach.** Native `<select>` popups are OS-rendered and can't be styled. Build one small reusable
accessible **`Select`** component (a button + popover `listbox`) that matches `DESIGN.md`
(`--card`/`--border`, 10px radius, `:focus-visible` ring, keyboard nav: Up/Down/Enter/Esc/type-ahead,
`prefers-reduced-motion`). Replace the four native selects: **Mode**, **Microphone**, **Language**,
**Appearance** (`SettingsView.tsx`). Keep the closed control visually identical to the current
`.input` so the rest of the layout is unaffected.

**Files.** New `src/components/ui/Select.tsx` (or add to `ui.tsx`); `SettingsView.tsx`;
`globals.css` if a token is needed.

**Verification.** `tsc --noEmit` clean; manual: open each dropdown, confirm rounded/bordered/themed
in light + dark + glass, keyboard nav works, focus ring visible.

## Sub-project 4 — History suite

**Intent.** Make History browsable over time: grouped by day, paginated, with a usage heatmap and a
date-range delete.

**Approach (four parts).**
1. **Day grouping.** Group the transcript list under date headings ("Today", "Yesterday", then
   "8 August 2026"). Frontend grouping over the existing `getTranscripts(limit, offset)`.
2. **Pagination.** Load ~1 month (or first N) initially, then a **"Show more"** button that pages
   with `offset`. No infinite scroll.
3. **Usage heatmap.** A GitHub-contributions-style grid (one cell per day, intensity = dictation
   count), sized to fit the panel, themed to `--primary` steps. Needs a **new backend query**
   returning `(day, count)` rows over a range.
4. **Date-range delete.** A "Delete a range" control (from–to) alongside the existing **Clear all**.
   Needs a **new backend command** `delete_transcripts_between(start, end)`.

Calendar/heatmap layout will get a visual mockup for owner sign-off before building this part.

**Files.** `src/components/views/HistoryView.tsx`; `src/lib/ipc.ts` (2 new calls);
`src-tauri/src/commands.rs` + `db.rs` (per-day counts query, range delete); possibly a small
`Heatmap`/`Calendar` component.

**Verification.** `cargo test` (new query/command), `tsc` clean; manual: headings correct across day
boundaries, "Show more" pages older entries, heatmap intensity matches real counts, range delete
removes exactly the selected span and nothing else.

## Sub-project 5 — Cleanup runs for every model

**Intent.** Confirm "Tidy up my dictation" + "Remove filler words" apply to **every** engine
(Moonshine, Parakeet, Dolphin, NeMo-CTC, Whisper), not just some.

**Approach.** Trace where `expand.rs`/cleanup is applied in the pipeline. It should be a
post-transcribe step in the facade/commands, engine-agnostic. Confirm it isn't gated per engine; add
a test that the cleanup pass runs on output regardless of `EngineKind`. Fix if any engine bypasses
it.

**Files.** `src-tauri/src/engine.rs`/`commands.rs`/`expand.rs` (read; fix only if a gap) + test.

**Verification.** `cargo test`; if practical, a real dictation on two different engines showing
fillers removed.

## Sub-project 6 — v1.0.0 release (gated on 1–5)

**Approach.**
1. Bump version to **1.0.0** in `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`.
2. Write **`docs/RELEASE-NOTES-v1.0.0.md`** — plain, user-facing: what's new, what each model does,
   the new History, privacy stance. Named and structured for readability.
3. Build the installer with the **Tauri CLI** (`npm run build`) — never plain `cargo build --release`.
4. Update `README`/`HANDOFF`.
5. Commit part-by-part, push the branch, open/merge, then create the **GitHub release** with the
   notes and the installer artifact.

**Verification.** Full `cargo test` + `tsc` green; the built binary launches and dictates (owner
smoke test) before the release is published.

## Build order

1 → 2 → 4 → 5 → 6. Each verified before the next. #6 only after 1–5 are done.
