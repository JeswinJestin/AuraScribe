# Prompt Optimization Engine — Design

**Status:** design approved in brainstorming (2026-08-25); awaiting spec review before an implementation plan.
**Owner:** Jeswin Thomas Jestin

## 1. What it is

An optional, on-device feature that turns rough text into a well-formed prompt for AI tools —
**in place**. You dictate normally; your words land at the cursor as usual. Then a floating button
(near the insert) or a global hotkey takes that text — or **any text you have selected** — and
rewrites it into a structured, context-preserving prompt, replacing the original.

It is 100% local: a small instruct LLM runs on the user's own machine. No cloud, no telemetry — the
local-first non-negotiable is intact (the only network request remains a one-time model download).

## 2. Goals / non-goals

**Goals**
- Optimize the **current selection** (robust) or the **most recent dictation** (best-effort) in place.
- Two triggers: a **floating button** shown after a dictation, and a **global hotkey** for any selection.
- **Intent-adaptive** output: infer from the text whether the user wants a structured prompt, a plain
  cleanup, or both, and produce accordingly — **never losing the original context/meaning**.
- Stay optional and lightweight: the ~1 GB model is a separate download; the default app is unaffected
  until the user opts in.

**Non-goals (v1)**
- No preview/confirm window (the user chose direct in-place replacement).
- No external context (clipboard/recent-history/active-app) — v1 is self-contained; those are
  fast-follows.
- No cloud fallback, ever.
- No streaming UI of tokens; a simple "Optimizing…" indicator is enough.

## 3. Interaction model (decided)

1. Dictation inserts the transcribed text at the cursor (unchanged; relies on the now-fixed clipboard
   injection — see §8).
2. A small **floating button** (an overlay-style always-on-top window) appears near the cursor for a
   few seconds. Clicking it optimizes the just-inserted text in place.
3. A **global hotkey** (default e.g. `Ctrl+Shift+O`, rebindable; `Cmd+Shift+O` on macOS) optimizes the
   **current selection**, anywhere, at any time.
4. Result replaces the source text directly — no preview.

## 4. Architecture / components

- **`src-tauri/src/optimize.rs`** (new) — the engine. Lazy-loads the GGUF model on first use via a
  llama.cpp Rust binding; builds the system prompt (§6); runs a single generation; returns the rewrite.
  Owns a cached model handle behind a `Mutex<Option<Model>>` so the ~1 GB load happens once.
- **Commands** (`commands.rs`):
  - `optimize_selection()` — hotkey path: capture selection → optimize → replace (§5).
  - `optimize_recent()` — button path: re-select the last dictation → optimize → replace (§5, ⚠️).
  - `optimize_model_status()` / `download_optimize_model()` — check/download the model (reuses the
    existing model-download infra).
- **`src-tauri/src/system.rs`** (or `injection.rs`) — add **capture current selection** (send Ctrl/Cmd+C,
  read clipboard) and reuse **replace selection** (set clipboard + paste). Both build on the fixed,
  race-free clipboard code from §8.
- **Floating button** — a new small `WebviewWindow` modeled on `overlay.rs` (transparent, always-on-top,
  non-activating so it never steals focus). Shows an icon + an "Optimizing…" spinner state.
- **Hotkey** — a second global shortcut registered via the `global-shortcut` plugin, alongside the
  dictation hotkey, with its own enable flag and rebinding.
- **Settings → "Prompt optimization"** — download the model (with progress), enable/disable the feature,
  set/rebind the hotkey, toggle the floating button.
- **Migration** — a settings migration for the new fields (`prompt_optimize_enabled`,
  `prompt_optimize_hotkey`, `prompt_optimize_button`), following the `009` pattern; all default off/empty.

## 5. Data flow

**Hotkey (robust path):**
1. Save current clipboard.
2. Send Ctrl/Cmd+C, wait briefly, read the clipboard → the selected text. If empty, no-op with a hint.
3. `optimize.rs` generates the rewrite.
4. Set clipboard to the rewrite → send Ctrl/Cmd+V (replaces the selection).
5. Restore the original clipboard on the background thread, only-if-still-ours (the §8 mechanism).

**Floating button (best-effort path):** ⚠️
- After a dictation we know the inserted text and its length. On click, re-select it by sending
  `Shift+Left` × char-count (or `Shift+Home` for a single line), then run the same optimize+replace as
  above. **Fragile if the cursor moved since dictating** — documented as best-effort; the hotkey-on-
  selection path is the reliable one.

## 6. The system prompt (the actual behavior)

A fixed system prompt instructs the model to:
1. **Read the user's text and infer intent** — does it ask (explicitly or implicitly) for a *structured
   prompt*, a *plain cleanup/clarification*, or *both*?
2. **Produce accordingly:** a structured prompt (Role/persona · Context · Task · Output format · any
   Constraints), a clean clarified request, or a cleaned-and-structured combination.
3. **Never lose the original context or meaning.** Expand and organize; do not invent facts the user
   didn't say, and do not drop details they did. Preserve every concrete instruction.
4. **Output only the result** — no preamble, no explanation (it goes straight to the cursor).

Generation is greedy/low-temperature for determinism and speed. The prompt template and a few worked
examples live in `optimize.rs` and are unit-testable independently of the model.

## 7. Model + runtime (decided)

- **Runtime:** llama.cpp via a Rust binding, compiled into the release build (adds ~1–3 MB to the
  binary — the model, not the engine, is the heavy part). Reuses the project's existing
  CMake/C++-from-source build pattern (whisper.cpp).
- **Model (lightweight-first — owner decision 2026-08-27):** default **Qwen2.5-0.5B-Instruct**, 4-bit
  quantized GGUF (**~0.4 GB**). Apache-2.0 licensed — clean for an open-source app. Chosen to keep the
  download and CPU load light per the project's "light + fast + accurate" rule; good enough for prompt
  cleanup/structuring. **Qwen2.5-1.5B-Instruct Q4 (~1 GB)** remains an optional "higher quality" download
  for stronger rewrites. Same ChatML template, so the engine code is identical for both.
- **Delivery:** optional on-demand download from Hugging Face into the models dir, via the existing
  download infra (`.part` → rename on completion). The feature no-ops with a "download the model" hint
  until the file is present. Default installer stays small; feature dormant until opted into.

## 8. Foundation: the clipboard/injection fix (done)

The "optimize in place" flow copies and replaces text via the clipboard, so it depends entirely on a
reliable clipboard. The pre-existing bug — a fixed 120 ms restore delay that let a slow target paste the
*old* clipboard instead of the dictation — was fixed first (commit `bb5a3c2`): restore now runs on a
background thread after 500 ms and only if the clipboard still holds our text, and is skipped entirely
when the paste keystroke fails. This is the substrate both dictation and this feature stand on.

## 9. Error handling

- **Model not downloaded** → button/hotkey shows a hint to download it; no crash.
- **Generation fails / times out** → keep the original text unchanged, surface a brief error.
- **Empty selection** (hotkey with nothing selected) → no-op with a hint.
- **Latency** → the floating button / an overlay shows an "Optimizing…" spinner; it's a few seconds.
- **Clipboard busy** → the existing retry logic (`open_clipboard_retrying`) already handles contention.

## 10. Testing strategy

**Unit-testable now (no model, deterministic):**
- System-prompt construction and the intent/template logic.
- The "re-select last dictation" length math (Shift-Left count for a given string).
- Clipboard save/restore/replace behavior (already covered by injection tests; extend for the
  capture-selection round-trip where mockable).

**Requires the owner's machine (POC):**
- The llama.cpp build, model load, and — critically — **generation quality**: does the rewrite preserve
  context and read well? This is judged by a human, on-device, and cannot be verified in CI/sandbox.

## 11. Risks / honest limits

- **llama.cpp build + generation are unverifiable from the sandbox.** The engine ships behind a POC the
  owner runs and judges before we call it done.
- **Floating-button in-place replace is genuinely fragile** (cursor may have moved). Hotkey-on-selection
  is the reliable trigger; the button is a convenience.
- **Latency**: a 1.5B model on CPU is seconds, not instant — set expectations in the UI. If too slow, a
  smaller model (e.g. 0.5B) is the fallback.
- **Binary/build weight**: another from-source C++ dependency increases build time.

## 12. Phasing

- **Phase 1 (POC, behind the feature + model gate):** `optimize.rs` + `optimize_selection` (hotkey) +
  the system prompt + model download + a Settings section. Owner runs it, judges speed/quality.
- **Phase 2:** the floating button and `optimize_recent` (best-effort in-place replace), once Phase 1
  quality is confirmed.
- **Phase 3 (fast-follows, optional):** external context (clipboard / recent history), style presets.

Building Phase 1 first keeps the risky, unverifiable-here parts (the LLM) isolated and judged before we
invest in the button UX.
