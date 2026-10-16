# Prompt optimization — current state and the plan to make it actually work

**Date:** 2026-10-16
**Status:** investigation + plan (not started). The feature is **not working today.**

## What's true right now (verified by reading the code)

- `src-tauri/src/optimize.rs` owns the *behaviour* only: the system prompt (`SYSTEM_PROMPT`) and the
  Qwen/ChatML `build_prompt()`. These are solid and unit-tested.
- `optimize_text()` has two paths:
  - **without the `prompt` feature** (the shipped default — `default = ["whisper", "moonshine"]`):
    returns `Err("Prompt optimization is not available in this build")`.
  - **with the `prompt` feature**: **echoes the input unchanged** (a deliberate no-op, Phase-1
    scaffolding). So even in a `prompt` build it does nothing useful.
- The hotkey → capture-selection → optimize → replace **pipeline** exists and is wired
  (`optimize_selection` command, a second global shortcut, Settings toggles). Only the model step is a
  stub.
- **Net: in the installed app, prompt optimization is off and does nothing.** It is *not* part of the
  v2.2.0 Notetaker release and ships disabled.

## What PR #3 (`feat/prompt-optimizer-and-overlay-fix`) already has

Open, **CONFLICTING/DIRTY** against master, +813/−60. It contains:
- **The real `llama-cpp-2` 0.1.150 integration** in `optimize.rs` (behind `prompt`): cached backend +
  model, `find_gguf()` locating the optimizer GGUF in the models dir (prefers 1.5B → 0.5B → any),
  `generate()` (tokenize → batch → greedy decode → `token_to_piece`, stop on `is_eog_token`),
  `clean_output()` (strips chat markers / quotes; unit-tested).
- `Cargo.toml`: `llama-cpp-2 = "=0.1.150"` + `llama-cpp-sys-2 = "=0.1.150"` + `encoding_rs`, all
  optional under `prompt` (pinned exactly — the API shifts between 0.1.x).
- An **overlay repaint-after-sleep fix** (independent, low-risk, genuinely useful).
- `test-build.yml`: a manual `workflow_dispatch` Windows build with `--features moonshine,prompt` that
  uploads the installer — the way to prove the llama.cpp/ggml link on MSVC.
- Its own note: **"not compile-verified in the sandbox"** (llama.cpp builds from source), and the
  feature link + generation are **owner-verified only**.

## Why it isn't just "merge it"

1. **Conflicts.** PR #3 was cut before the Notetaker work; it touches `commands.rs`, `hotkey.rs`,
   `injection.rs`, `main.rs`, `overlay.rs`, `storage.rs`, `ipc.ts`, `SettingsView.tsx` — all of which
   moved a lot on `feat/notetaker-mvp-and-notepad-fix`. A rebase onto the new master is required and
   non-trivial.
2. **Never compiled against a real toolchain.** `llama-cpp-2` builds llama.cpp from source (CMake +
   a C++ compiler + ggml). It has to be linked on MSVC at least once — that's what `test-build.yml` is
   for. It cannot be proven in this sandbox.
3. **Needs a model + a real run.** A Qwen2.5 GGUF (~0.4 GB for 0.5B, ~1 GB for 1.5B) has to be
   downloaded and the `generate()` loop exercised on real text, on-device.
4. **Heavier app.** Pulling in llama.cpp + ggml grows the build and the install; weigh against the
   "stay lightweight" non-negotiable. Keep it strictly behind the opt-in `prompt` feature so the
   default installer is unaffected (as PR #3 already does).

## Plan to make it actually work

1. **Rebase PR #3 onto the current master** (post-2.2.0 merge). Resolve conflicts carefully in the
   eight shared files; keep the Notetaker changes intact and layer the optimizer/overlay changes on
   top. Re-run `cargo test` + `tsc`.
2. **Prove the link** with `test-build.yml` (`--features moonshine,prompt`) on CI (MSVC). Fix any
   ggml/llama.cpp link issues there — this is the step the sandbox can't do.
3. **Wire the model**: add the Qwen2.5 GGUF to the model flow (download on demand, like the Notetaker
   add-ons), default 0.5B to stay light, 1.5B optional. `find_gguf()` already looks for it.
4. **Owner on-device verification**: install the `prompt`-feature build, download the GGUF, select real
   text, hit the optimize hotkey, and confirm a sensible rewrite replaces the selection. Capture a
   before/after in `aurascribe.log`.
5. **Ship behind a clearly-labelled, opt-in toggle** (Settings) with the honest size/why, once it's
   verified. Keep the default installer free of llama.cpp.

## Low-risk extract worth doing sooner

The **overlay repaint-after-sleep fix** in PR #3 is independent of the LLM and fixes a real "pill never
appears after the machine wakes" bug. It could be cherry-picked, rebased, and merged on its own, ahead
of the full prompt-optimization effort.
