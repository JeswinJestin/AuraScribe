# Prompt Optimization Engine — Implementation Plan (Phase 1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an on-device "optimize this text into a prompt" action — triggered by a global hotkey on the current selection — that rewrites text via a local LLM, in place.

**Architecture:** A new `optimize.rs` owns an intent-adaptive system prompt and (Phase 1b) a llama.cpp model handle. A command captures the selection via the clipboard, runs the optimizer, and replaces the selection. Gated behind a `prompt` cargo feature and an opt-in setting; dormant until the model is downloaded.

**Tech Stack:** Rust/Tauri, `llama-cpp-2` (GGUF, Phase 1b), the existing clipboard injection code, sqlx migrations, Next.js settings UI.

**Spec:** `docs/superpowers/specs/2026-08-25-prompt-optimization-engine-design.md`

## Global Constraints

- **Local-first, no cloud, ever.** The optimizer runs on-device; the only network call is the one-time model download. (CLAUDE.md non-negotiable.)
- **Never claim more than the code does.** The LLM path (Phase 1b) is owner-built + owner-verified; do not mark it "working" from the sandbox.
- **Feature/model gated.** Default builds and existing users are unaffected until they enable the setting AND download the model. App stays ~8 MB by default.
- **Model (lightweight-first, owner decision 2026-08-27):** default **Qwen2.5-0.5B-Instruct, Q4 GGUF (~0.4 GB)**, Apache-2.0 — the lightest instruct model that still cleans up / structures prompts acceptably, chosen to keep the footprint and CPU load low. **Qwen2.5-1.5B-Instruct Q4 (~1 GB)** stays available as an optional "higher quality" download for users who want stronger rewrites. Runtime: llama.cpp via `llama-cpp-2`. Both share the same ChatML template, so `build_prompt` is unchanged.
- **Trigger (Phase 1):** global hotkey on the current selection. Floating button = Phase 2.
- **Build with the Tauri CLI + `--features prompt`; never plain `cargo build`.**

---

### Task 1: `optimize.rs` — the intent-adaptive system prompt (verifiable now)

**Files:**
- Create: `src-tauri/src/optimize.rs`
- Modify: `src-tauri/src/main.rs` (add `mod optimize;`)
- Modify: `src-tauri/Cargo.toml` (add `prompt = []` feature; `llama-cpp-2` added in Task 5)

**Interfaces:**
- Produces: `optimize::build_prompt(user_text: &str) -> String` — the full chat prompt (system + user) fed to the model. `optimize::SYSTEM_PROMPT: &str`.
- Produces: `optimize::optimize_text(user_text: &str) -> Result<String, String>` — returns the rewrite; without the `prompt` feature/model it returns `Err("Prompt optimization model is not installed")`.

- [x] **Step 1: Write failing tests** for `build_prompt` (contains the system rules, preserves the user's exact text, wraps in the model's chat template) and that `optimize_text` errors cleanly without the feature.
- [x] **Step 2: Run tests, verify they fail** (`cargo test --manifest-path src-tauri/Cargo.toml optimize`).
- [x] **Step 3: Implement** `SYSTEM_PROMPT` (infer intent: structured prompt / cleanup / both; never lose original context; output only the result), `build_prompt` (Qwen chat template: `<|im_start|>system…<|im_end|><|im_start|>user…`), and `optimize_text` gated `#[cfg(not(feature = "prompt"))]` → the error; `#[cfg(feature = "prompt")]` → calls the model (stub returns `Ok(user_text.into())` until Task 5).
- [x] **Step 4: Run tests, verify pass.** (5 optimize tests green.)
- [x] **Step 5: Commit** `feat(prompt): optimize.rs system prompt + intent rules (scaffolding)` — shipped as `5446809`.

### Task 2: Capture the current selection via the clipboard (verifiable now)

**Files:**
- Modify: `src-tauri/src/injection.rs` (add `capture_selection`)
- Test: `src-tauri/src/injection.rs` tests

**Interfaces:**
- Produces: `injection::capture_selection() -> Option<String>` — saves the clipboard, sends Ctrl/Cmd+C, waits briefly, reads the copied text, restores the clipboard on the background thread (reusing the §8 race-free restore), returns the selected text (None if empty).

- [x] **Step 1:** Add `capture_selection` (Windows: `send_ctrl_c()` mirroring `send_ctrl_v`, then `read_clipboard_text`; restore via the existing background pattern). macOS/Linux: enigo Ctrl/Cmd+C + arboard read.
- [x] **Step 2:** Unit-test the clipboard round-trip for capture (extend the existing `clipboard_round_trips_awkward_text` pattern; the copy-keystroke itself isn't unit-testable, so test the read/restore logic).
- [x] **Step 3:** `cargo test … injection` → pass.
- [x] **Step 4: Commit** `feat(prompt): capture current selection via clipboard` — shipped as `cd3aee8`.

### Task 3: Settings — migration, fields, IPC, UI (verifiable now)

**Files:**
- Create: `src-tauri/migrations/010_prompt_optimize.sql`
- Modify: `src-tauri/src/db.rs` (SettingsRow + save), `src-tauri/src/commands.rs` (Settings struct/default/load/save), `src/lib/ipc.ts` (Settings type), `src/app/page.tsx` (DEFAULT_SETTINGS), `src/components/views/SettingsView.tsx` (a "Prompt optimization" section)

**Interfaces:**
- Produces: settings fields `prompt_optimize_enabled: bool` (default false), `prompt_optimize_hotkey: String` (default `Ctrl+Shift+O` / `Super+Shift+O` on macOS).

- [x] **Step 1:** Migration `ALTER TABLE settings ADD COLUMN prompt_optimize_enabled INTEGER NOT NULL DEFAULT 0;` and `… prompt_optimize_hotkey TEXT NOT NULL DEFAULT '';` (empty → resolve default in code, like the hotkey pattern).
- [x] **Step 2:** Thread the fields through `SettingsRow` (+ `save_settings` `$14/$15`), the `Settings` struct (default/load/save), `ipc.ts`, `page.tsx` `DEFAULT_SETTINGS`.
- [x] **Step 3:** Add a Settings "Prompt optimization" section: a toggle bound to `prompt_optimize_enabled`, a hotkey capture (reuse `HotkeyCapture`), and a placeholder for the model download (Task 6).
- [x] **Step 4:** `cargo test` (settings round-trip) + `npm run typecheck` → pass.
- [x] **Step 5: Commit** `feat(prompt): settings (enable + hotkey), migration 010` — shipped in `b19e345` (with Task 4).

### Task 4: The command + second global hotkey (verifiable now)

**Files:**
- Modify: `src-tauri/src/commands.rs` (`optimize_selection` command), `src-tauri/src/main.rs` (register command), `src-tauri/src/hotkey.rs` (register the second shortcut → invoke optimize)

**Interfaces:**
- Produces: command `optimize_selection()` — `capture_selection()` → `optimize::optimize_text()` → `inject_text()` to replace. Surfaces errors (no selection / model missing) via `emit_status` last_error or a toast.

- [x] **Step 1:** Implement `optimize_selection`: if no selection → return a clear error; else optimize and inject; show an "Optimizing…" status while running.
- [x] **Step 2:** In `hotkey.rs`, register `prompt_optimize_hotkey` (when set + `prompt_optimize_enabled`) alongside the dictation hotkey; on trigger, spawn `optimize_selection`.
- [x] **Step 3:** `cargo check --features moonshine` + `--features "moonshine prompt"` → compile clean.
- [x] **Step 4: Commit** `feat(prompt): optimize_selection command + second hotkey` — shipped as `b19e345`.

### Task 5 (OWNER BUILDS + TESTS): wire llama-cpp-2 into optimize.rs

**Files:**
- Modify: `src-tauri/Cargo.toml` (`llama-cpp-2` as an optional dep under `prompt`), `src-tauri/src/optimize.rs` (model load + generation)

**Interfaces:**
- Fills in the `#[cfg(feature = "prompt")]` branch of `optimize_text`: lazy-load the GGUF from the models dir into a cached `Mutex<Option<LlamaModel>>`, build a context, decode `build_prompt(text)`, greedy-sample to EOS/`<|im_end|>`, return the decoded string.

- [x] **Step 1:** Added `llama-cpp-2 = { version = "=0.1.150", optional = true }` + `encoding_rs` (streaming UTF-8 decode of tokens); `prompt = ["dep:llama-cpp-2", "dep:encoding_rs"]`.
- [x] **Step 2:** Implemented the `llm` submodule in `optimize.rs` (behind `#[cfg(feature = "prompt")]`): cached backend (`OnceLock<LlamaBackend>`) + cached model (`Mutex<Option<Arc<LlamaModel>>>`), `find_gguf()` locates the optimizer `.gguf` in the models dir (prefers 1.5B, then 0.5B, then any), and `generate()` does tokenize → batch → decode → **greedy** sample loop → `token_to_piece` until `is_eog_token`. `clean_output()` strips stray chat markers / wrapping quotes (compiled + tested on every build). Written against the **verified 0.1.150 API** (docs.rs signatures), NOT from memory — but it is **not compile-verified** (llama.cpp builds from source + needs the crate fetched; unavailable in the sandbox). Default build + 6 optimize tests stay green.
- [ ] **Step 3 (OWNER — first build + judge):** `.\moonshine-build.bat` with `--features "moonshine prompt"`, place a Qwen2.5-**0.5B** Q4 GGUF (the lightweight default; try 1.5B too if you want to compare) in `%LOCALAPPDATA%\AuraScribe\models`, run the app, select text, press the hotkey, and **judge the rewrite quality** (context preserved? reads well? latency acceptable? is 0.5B good enough, or is 1.5B worth the extra ~0.6 GB?).
  - **✅ BUILD VERIFIED (2026-08-29, CI run 33231075757, `--features moonshine,prompt`, Windows).** The
    feared ggml symbol collision **did not occur** — whisper.cpp + llama.cpp + sherpa all link into one
    installer (the sidecar-process fallback is NOT needed). Two *unrelated* issues surfaced and are fixed:
    (1) **`onnxruntime.dll doesn't exist`** at Tauri's resource check — the sherpa DLLs sit under
    `target/release/deps` until link time, and the longer llama build exposed the ordering; `test-build.yml`
    now compiles deps first and copies the DLLs to `target/release` before bundling. (2) **`no field
    use_mmap on llama_model_params`** — `llama-cpp-2 0.1.150` declares `llama-cpp-sys-2 ^0.1.150` and Cargo
    pulled the newer `0.1.154` whose bindings dropped that field; **pinned `llama-cpp-sys-2 = "=0.1.150"`**
    (Cargo.toml + lock) so the wrapper and bindings match. Installer built + downloadable
    (`aurascribe-windows-prompt-exe`, ~10 MB). **`moonshine-build.bat` for a local build should pass the
    same `--features moonshine,prompt`; the DLL-ordering copy may be needed locally too if it recurs.**
  - **If the `llm` module ever needs API tweaks on a version bump:** it's ~60 lines written to `llama-cpp-2`
    0.1.150 (+ `-sys` 0.1.150); keep the two in lockstep and check docs.rs (likely spots:
    `LlamaSampler::greedy()`/`.sample()`/`.accept()`, `token_to_piece` args, `with_n_gpu_layers`). The
    behavior around it (system prompt, ChatML, cleanup) is stable + tested.
- [ ] **Step 4:** Iterate on the system prompt / sampling based on real output; commit once quality is acceptable.

### Task 6 (OWNER): model download UI + end-to-end

**Files:**
- Modify: `src-tauri/src/commands.rs` (`optimize_model_status` / `download_optimize_model` reusing the model-download infra), `SettingsView.tsx` (download button + progress)

- [x] Wire the download to the existing model-download code; show progress; the feature no-ops with a "download the model" hint until present. **DONE (2026-08-29):** `optimize_model_status` / `download_optimize_model` commands (reqwest streaming → `.part` → rename, `optimize-model-download-progress` events, mirrors the ASR `download_model`), and a `OptimizeModelRow` in Settings → Prompt optimization (status + Download button + progress bar) replacing the old "coming soon" note. Fetches Qwen2.5-0.5B Q4 (~491 MB). **Also fixed a critical adjacent bug:** Storage → Reclaim classified the optimizer `.gguf` as an orphan and DELETED it (owner lost the model; log `Reclaimed 491400032 bytes`) — `storage.rs` now treats any `.gguf` as a Model (regression test added). Owner still verifies the interactive download + optimize flow.

---

## Notes on execution order

Tasks 1–4 are **buildable and verifiable in the sandbox now** (compile + unit tests + typecheck) and deliver the entire feature *except the actual LLM call* (which stubs to identity). Task 5–6 are the **owner-built + owner-judged** LLM parts. This isolates the one unverifiable-here piece and lets the scaffolding land tested.

## Self-review

- **Spec coverage:** interaction (hotkey) ✓ T4; intent-adaptive prompt ✓ T1; self-contained ✓ (selection only); clipboard foundation ✓ (done + T2); model/runtime ✓ T5; gating/download ✓ T3/T6; settings/UX ✓ T3. Floating button = Phase 2 (out of scope here, per spec).
- **Placeholders:** the llama-cpp-2 version and exact API are intentionally left to Task 5's first build (owner territory); everything in T1–T4 is concrete.
- **Type consistency:** `optimize_text`/`build_prompt`/`capture_selection`/settings field names are used consistently across tasks.
