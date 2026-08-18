# Spotlight Onboarding — design

**Date:** 2026-08-18 · **Status:** approved (owner said "build it") · **Replaces:** `src/components/Onboarding.tsx` (the 5-step modal)

## Goal

A short, interactive first-run walkthrough that highlights one thing at a time and dims/blurs the
rest, so a new user understands AuraScribe fast and doesn't drop off. Skippable at every step.
Replayable later. Must stay lightweight (no new dependency) and honour `prefers-reduced-motion`.

## Constraints discovered in the code

- On a **fresh install no model is loaded**, so `DictateView` shows an "Add a voice model to begin"
  empty state with a **Download** button — the record button / "Press <hotkey>" panel only appears
  *after* a model loads (`DictateView.tsx`). So a spotlight can reliably anchor to the **Download
  CTA** on first run, and to the **record button** on replay (model present). The tour tries the
  Download CTA selector first, then the record selector.
- `docs/DESIGN.md` restrains motion ("the signal meter is the only element allowed real motion").
  The step-2 animation is a **deliberate, documented exception** for the first-run surface: brief,
  colour-matched, reduced-motion-aware. Recorded here so it is intentional, not drift.

## Mechanism

New `src/components/SpotlightTour.tsx`, portaled to `document.body` at a high z-index. No dependency.

- **Dim + soft blur** the surround. For the spotlight step, four fixed `backdrop-filter: blur`
  panels tile everything *except* the target rect (no CSS-mask fragility), plus an indigo ring
  around the target. For the card/demo steps, one full-screen blurred dim layer.
- Target measured via `getBoundingClientRect()` and tracked every animation frame while the
  spotlight step is active (re-measures on layout/scroll/resize), so the highlight never drifts.
- Anchors are real elements tagged with `data-tour="download-model"` and `data-tour="record"` in
  `DictateView`. The tour resolves the first that exists.
- A Glass tooltip card (reusing `.panel`/tokens) sits beside the target (below if room, else above),
  or centered for the card/demo steps. Progress dots + Back / Next; final Next = "Start dictating".
- **Skip at every step:** a persistent "Skip tour" control (top-right) dismisses immediately and
  drops the user into the app. Skip and Finish both persist `onboarded = true`.

## The 3 stops

1. **Welcome** — centered card over dim+blur. "Your voice, on your machine — 100% on-device, free."
2. **See it work** — an inline **motion graphic** (`HotkeyDemo`), a short (~4.5s) looping,
   colour-matched sequence driven by a small JS phase machine (not CSS infinite loops, so
   reduced-motion can jump straight to the finished frame):
   `Ctrl + Shift + Space` keys press → mic lights up (indigo "listening") → signal bars →
   text types itself into a faux field ("…right where your cursor is"). Reduced-motion → final
   frame, no animation.
3. **Add a model** — the real DOM **spotlight**: anchors to the Download CTA (fresh install) or the
   record button (replay). "It all starts with one voice model — download once, then it runs
   offline." Finishing leaves the user looking at that button.

## Wiring

- `page.tsx`: show the tour when `tauri && !onboarded` **or** when a transient `replay` flag is set.
  When it opens, force `view = 'dictate'` and expand the sidebar so the anchor exists. On
  Skip/Finish, persist `onboarded = true` (first run only) and clear `replay`.
- `SettingsView`: a "Replay walkthrough" button calls an `onReplayTour` prop → sets `replay`.
- `DictateView`: add `data-tour="download-model"` and `data-tour="record"`.
- Delete `Onboarding.tsx`.
- `/preview` harness: replace the old "Onboarding" tab with the tour rendered over a faux
  Dictate panel that carries `data-tour="download-model"`, so the spotlight step is previewable.

## Out of scope (YAGNI)

No auto-navigation into buried Settings controls, no multi-page tour, no analytics, no persisted
"seen the tour" beyond the existing `onboarded` flag.

## Verification

Built and clicked through live in the `/preview` harness (next dev) before commit: all three steps,
the animation, the spotlight positioning on a real anchor, Skip on every step, reduced-motion
fallback. Plus `npm run typecheck` and `npm test`.
