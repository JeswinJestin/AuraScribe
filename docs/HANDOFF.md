# AuraScribe — Project Handoff

> **This is the single source of truth for project state.** If you are an AI assistant
> starting a fresh session, read this file first, then `docs/ARCHITECTURE.md`. Update this
> file at the end of every task — see `docs/MAINTAINING-DOCS.md` for the rules.

**Last updated:** 2026-08-08 &nbsp;·&nbsp; **Owner:** Jeswin Thomas Jestin

**Status — v1.0.0 (first stable release), multi-engine, Windows. Malayalam AND Kannada VERIFIED
working by the owner's real mic tests.** AuraScribe presses a hotkey → speaks → clean text at the
cursor, 100% local. Five model tiers cover English, ~40 Asian languages, 25 European languages, and
Malayalam/Kannada — all fast, offline, free. Words/Snippets apply to dictation; cleanup, glass
default, 5-step onboarding, overlay click-to-stop all shipped.

**v1.0.0 changes (this milestone, see `docs/superpowers/specs/2026-08-08-v1-release-design.md`):**
- **Recommended model** now elects **AuraScribe English (Moonshine base)** — the election in
  `engine.rs` was speed-first (electing the fractionally-faster Mini); switched to accuracy-first,
  tie-broken by fastest (`elect_recommended`, with tests).
- **Dropdowns** — new accessible `Select` (`ui.tsx`) replaces the four native `<select>`s in
  Settings; themed popover, keyboard nav, no OS-drawn list.
- **History** — day-grouped headings, "Show more" pagination, a 6-month usage heatmap, and
  date-range delete (`db.daily_counts` / `delete_transcripts_between` + commands + `HistoryView`).
- **Cleanup** verified engine-agnostic (applied to the transcript string post-transcribe, no
  per-engine branch) — runs for all five engines.
- Injection "paste into terminal" report investigated: not reproducible (both typing and paste
  paths verified landing in Claude Code); transient, no code defect found.
- **Cross-platform:** macOS/Linux remain Windows-only for now — the honest plan (Linux via WSL2/VM;
  macOS via CI + a cloud/borrowed Mac) is not yet implemented and must be verified on-target.

**Open-source contribution (in progress):** Malayalam model published to HuggingFace
(`jeswinjestin/sherpa-onnx-nemo-ctc-indicconformer-malayalam`, verified: model.onnx + tokens.txt +
`test_wavs/0.wav`). Part A ✅ and Part B ✅ (sherpa-onnx discussion #3199, "Update — solved" comment)
done; Part C (docs PR) drafted, awaiting a maintainer reply before opening. Kannada is next (same
process, its own HF repo — batch with the Malayalam docs PR). See `docs/CONTRIBUTING-MALAYALAM.md`
(git-ignored: owner's private process notes).

**`.gitignore` hardened this session:** excludes local agent tooling (`.agents/`, `.codex/`,
`.github/hooks/`, `skills-lock.json`), the owner's private contribution/research notes, and the
Colab export notebook — so only working code + public docs get pushed. Verified no secrets or
machine-path leaks in the pushable tree.

**Current state at a glance**

- **Speech engines** (all via the `engine.rs` `Asr` facade, routed by each model's `EngineKind`):
  | UI name | `EngineKind` / crate | Model id(s) | Coverage | Verified? |
  |---|---|---|---|---|
  | AuraScribe English (+ Mini) | Moonshine (sherpa-rs) | `moonshine-base-en`, `moonshine-tiny-en` | English, ~0.1× | ✅ owner |
  | AuraScribe Asian | Dolphin (sherpa-rs) | `dolphin-base-multilang` | ~40 Asian incl. Hindi/Tamil (auto-detect) | ✅ owner (log) |
  | AuraScribe European | Parakeet (sherpa-rs `transducer`) | `parakeet-v3-multilingual` | 25 European (auto-detect) | ⚠️ owner should retest |
  | AuraScribe Malayalam / Kannada | NeMo-CTC (raw `sherpa-rs-sys` FFI) | `indicconformer-ml`, `indicconformer-kn` | Malayalam / Kannada | ✅ ml owner-verified; ⚠️ kn untested |
  | (fallback engine only) | Whisper (whisper.cpp) | — none shipped — | — | engine kept as code fallback |
- **The whole ASR stack is `moonshine`-feature-gated** and **only runs in a RELEASE build**
  (`moonshine-build.bat`) — a debug build trips a harmless CRT debug-heap assertion (Round 17).
  `sherpa-rs` + `sherpa-rs-sys` provide the sherpa-onnx runtime; DLLs are bundled by the installer.
- **Models download on demand**, packaged for sherpa-onnx at download/load time. **IndicConformer
  needs `ensure_packaged`** (`nemo_ctc.rs`) to append ONNX metadata, or sherpa-onnx `exit()`s and
  crashes the app — see Round 27/26b. Never ship a sherpa model load unverified.
- **Recommendation is speed-first** (`engine.rs`) → `moonshine-base-en`. **Glass** is the default
  appearance for fresh installs. **Language UI** shows AuraScribe names + "Powered by <engine>" +
  full language lists (`src/lib/models.ts`).
- **Build + publish** (owner runs; sandbox blocks the release network fetch + git):
  1. `moonshine-build.bat` → `AuraScribe_0.4.1_x64-setup.exe` (~8.8 MB).
  2. Install elevated, replacing `C:\Program Files\AuraScribe` (PROCESS RULE below).
  3. `git push origin master` · `git tag v0.4.1 && git push origin v0.4.1` ·
     `gh release create v0.4.1 --notes-file docs/RELEASE-NOTES-v0.4.1.md src-tauri/target/release/bundle/nsis/AuraScribe_0.4.1_x64-setup.exe`
- **Open-source contribution (owner, in progress):** publish the packaged Malayalam model to
  HuggingFace + answer sherpa-onnx discussion #3199 — full guide in `docs/CONTRIBUTING-MALAYALAM.md`.
- **Next features (discussed, not built):** a **Linux** port (feasible — Tauri + Linux
  text-injection à la Handy); an **"AuraScribe Universal"** auto-routing mode (heavy: several
  engines + a language-ID router). **iOS is a redesign, not a port** — iOS bans system-wide text
  injection / global hotkeys, so the core UX can't work as-is. Kannada needs an owner test.
- **PROCESS RULE:** there must be exactly ONE AuraScribe installed. To show the owner a change,
  rebuild the installer and reinstall (elevated, replacing Program Files) — never launch a loose
  `target\*` build. Ignoring this caused the recurring "old UI" confusion (Round 19).

### Round 30 (2026-08-08) — owner feedback round 2 on the v1.0.0 build

Owner inspected the round-29 build and flagged four things; all fixed and rebuilt:

- **Recommended badge sat on "AuraScribe European", not English.** The old rule was
  *accuracy-first over all models*, and the real catalogue gives `parakeet-v3-multilingual`
  accuracy 5 — so the badge landed on the multilingual Parakeet. `elect_recommended`
  (`engine.rs`) is now **English-first**: among models that keep up with speech it prefers
  non-multilingual (English) models, then most accurate then fastest — electing
  **`moonshine-base-en` (AuraScribe English)** and keeping the badge off the multilingual
  "European"/"Asian" models. Tests updated to the real catalogue values (parakeet accuracy 5,
  multilingual) plus a fallback test for when no English model keeps up.
- **Single-key hotkeys were accepted (a bare `C` would hijack dictation while typing).**
  `HotkeyCapture` (SettingsView) now **rejects any capture without a modifier** — a lone
  letter/number/space/F-key shows a hint and is ignored; you must press a modifier + a key
  (Ctrl/Alt/Shift + key). The default remains `Ctrl+Shift+Space`.
- **Heatmap top labels were unreadable.** Month labels were one-truncated-letter-per-cell
  ("J", "F", "M", …) that read as gibberish. Rewrote the label pass: each month now gets one
  **full short month name** ("Jan"…"Dec") spanning its own columns, exactly like GitHub's year
  graph, and the confusing day-letter column on the left was removed.
- **"Delete a date range" must live in the right-hand rail**, under the "Your data"/"Tip"
  cards, in red. Moved it out of the History tab entirely into the `WidgetRail` history card —
  a new `danger` card ("Delete a date range") with a red outline, red header, red border button
  and a two-step confirm. The rail bumps `HistoryView` via a `reloadToken` prop so the list and
  heatmap refresh after a range delete. History's own right-hand column is gone (full-width
  layout).
- **Dropdown option highlight now matches the sidebar.** Added `.select-option-active` (used by
  the custom `Select`) plus `.glass-bg .select-popover li:hover` rules so the hover/active row
  uses the same frosted-white hover and indigo active tone as the sidebar nav items, instead of
  the muddy warm-charcoal `bg-accent`.
- Verified: **48 tests pass** (was 47 — one new fallback test), `tsc --noEmit` clean, and the
  installer rebuilt with `--features moonshine --config src-tauri/tauri.moonshine.conf.json`
  (`AuraScribe_1.0.0_x64-setup.exe`, ~8.8 MB). **Owner must reinstall** (PROCESS RULE: one
  install) and re-check: Recommended badge on AuraScribe English, hotkey capture rejecting a
  bare key, the heatmap month labels, the red delete-range card in the right rail, and the
  dropdown hover tone in Glass.

### Round 29 (2026-08-08) — owner-verified UI fixes on the v1.0.0 build

Owner ran the v1.0.0 installer and flagged real defects; all fixed and rebuilt:

- **Model list order was wrong.** The user wants **AuraScribe English Mini first**, then
  **AuraScribe English (286 MB)** with the **Recommended** badge on it for English. The
  catalogue printed base-first. Fixed in `moonshine.rs`: `MOONSHINE_MODELS` now lists
  `moonshine-tiny-en` ("English Mini") first, `moonshine-base-en` ("English") second;
  the facade's `elect_recommended` (accuracy-first among real-time models) still pins the
  badge on **`moonshine-base-en`** (accuracy 4 vs Mini's 3). Tests lock it.
- **Custom `Select` popover (ui.tsx) was clipped and un-themeable.** It rendered as an
  absolutely-positioned child of the control, so the window's `overflow-y-auto` content
  column clipped it — it appeared *below* the card, half-hidden. Rewrote it to render through
  a **portal into `document.body`** with `position: fixed` coords computed from the button's
  `getBoundingClientRect()`, flipping **up** when there isn't enough room below. It now floats
  as a **glass-morphism pane above the card** in Glass (frosted blur + translucent navy via
  the new `.select-popover` / `.glass-bg .select-popover` rules in `globals.css`), and tracks
  `--popover` tokens in light/dark.
- **History layout.** The usage **heatmap now fills the full panel width** like GitHub's
  graph (`HEATMAP_WEEKS` = 53, a full year) with month labels up top, weekday hints on the
  left, per-day hover tooltips ("3 dictations on Thursday, 8 August 2026"), and the grid
  measured to exactly fill the container. **Delete-a-date-range moved out of the main column
  into a right-hand rail** of the History tab, per the owner's spec. The transcript day groups
  ("Today" / "Yesterday" / dated) render to its left.
- Version label in the Settings rail bumped v0.4.1 → **v1.0.0**.
- Verified: 47 tests pass, `tsc --noEmit` + `next build` clean, installer rebuilt with
  `--features moonshine` + the Moonshine bundle overlay (`AuraScribe_1.0.0_x64-setup.exe`,
  ~8.8 MB). **Owner must reinstall** (PROCESS RULE: one install) and re-check: model list
  order + badge, the open dropdown in every appearance, and the History heatmap/right rail.

### Round 28 (2026-08-08) — AuraScribe model branding; Whisper removed; contribution guide

Post-Malayalam polish (owner requests):

- **Removed Whisper `small`** (`asr.rs::MODELS` empty again) — "get rid of it, it's slow". The fast
  engines now cover what matters; Whisper engine stays as code fallback only.
- **AuraScribe-branded model names + full language lists** (`src/lib/models.ts` +
  `SettingsView`): each model shows an AuraScribe name (e.g. **AuraScribe European**,
  **AuraScribe Malayalam**, **AuraScribe Asian**, **AuraScribe English**), "Powered by <engine>"
  (credits Moonshine/Parakeet/Dolphin/AI4Bharat — also good license hygiene), and the **complete
  language list** so the user understands a model's coverage end-to-end. Removed the old raw-id +
  "multilingual" badge. `EUROPEAN_LANGS`/`ASIAN_LANGS` live in `models.ts`. (Dictate screen's small
  model readout still shows the raw id — minor, deferred.)
- **`docs/CONTRIBUTING-MALAYALAM.md`** — a complete, beginner-grade walkthrough for the owner to
  publish the packaged Malayalam model (HuggingFace upload via web UI + ready model card, answer
  #3199, optional PR). The upload files are the app-packaged `model.onnx` + `tokens.txt` in the
  owner's models folder (CC-BY-4.0, attribution included).
- Verified: 43 tests pass, `cargo check --features moonshine` clean, `next build` typechecks.

**Discussed, NOT built (owner said don't implement the "one model for everything" idea):** no single
free model does all of English+European+Asian+Malayalam fast; a true "auto across all regions" would
mean downloading several engines + a language-ID router (a heavy v2 feature). Each engine already
auto-detects within its own region. **iOS/Linux (owner's next ask):** Linux is a feasible Tauri port
(needs Linux text-injection via xdotool/wtype like Handy); **iOS is a redesign, not a port** — iOS
forbids system-wide text injection/global hotkeys, so the "dictate into any app" model can't work
as-is (would need a custom keyboard extension). sherpa-onnx itself runs on both.

### Round 27 (2026-08-07) — Malayalam WORKS: auto-package IndicConformer ONNX (verified load)

Solved the Round-26b crash properly. **Verified in the sandbox** with the sherpa-onnx Python API on
the exact `trysem` Malayalam model: appending the six `metadata_props` sherpa's own exporter sets
(`vocab_size`, `normalize_type=per_feature`, `subsampling_factor=8`,
`model_type=EncDecHybridRNNTCTCBPEModel`, `version`, `model_author`) makes `from_nemo_ctc` **load
cleanly (no `exit()`) and decode Malayalam** (silence → `അ`; UTF-8-confirmed).

- **`nemo_ctc.rs::append_sherpa_metadata`** writes those entries as raw protobuf bytes appended to
  `model.onnx` (metadata_props = field 14, repeated `StringStringEntryProto`; repeated fields merge
  regardless of position, so no 493 MB parse — verified onnxruntime reads them back). `ensure_packaged`
  runs it once (`.packaged` marker), from both download and load, so a **raw pre-existing download
  from the crashing build is auto-repaired on next load**.
- **Re-enabled `indicconformer-ml` / `-kn`** in the catalogue. `EngineKind::NemoCtc` path unchanged.
- Model I/O confirmed: `audio_signal[_,80,_]` + `length` → `logprobs[_,_,5633]` (5632 vocab + blank).
- Verified: `cargo check --features moonshine` clean; Python load+decode PASS.
- **✅ VERIFIED BY THE OWNER (real mic test):** dictated a full Malayalam paragraph, transcribed to
  clean, coherent Malayalam — including code-switched "Manglish" rendered sensibly. Owner reports
  it "far more better, accurate", satisfied with the speed. **Malayalam is DONE** — fast, local,
  accurate, no cloud. This closes the last language gap. Tooling: `scratchpad/package_and_test.py`.
  Only `ml` + `kn` are enabled; the `trysem` repo has all 12 Indian languages if more are wanted
  (though Dolphin already covers hi/ta/te/bn/gu/mr/pa/or/ur). Natural next step: repackage the
  metadata-added model, upload to HF, and answer #3199 — the owner's open-source contribution.

### Round 26b (2026-08-07) — CRASH FIX: raw IndicConformer ONNX aborts sherpa; catalogue cleared

Round 26 shipped a load that **crashed the app**. Log proof: each `Loading NeMo-CTC model
indicconformer-ml` line is immediately followed by a fresh `==== AuraScribe started ====` — the
process died inside `SherpaOnnxCreateOfflineRecognizer`. Root cause: `trysem`'s ONNX is **raw
AI4Bharat export with no sherpa-onnx metadata**, and sherpa-onnx calls **`exit()`** on such a model
rather than returning null — so the Rust null-check can't catch it. (Download + `tokens.txt`
generation worked; only the load aborts.)

Fix: **emptied `NEMO_MODELS`** so no nemo_ctc model can be selected → crash can't be triggered. The
engine code stays, ready for a *properly packaged* model. Lesson (re-)learned, the CLAUDE.md one:
**do not ship a sherpa model load without verifying it actually loads** — a bad one aborts the
process, it doesn't fail gracefully.

**The real Malayalam path from here:** the IndicConformer weights DO exist (trysem, CC-BY) — the
missing step is packaging them for sherpa-onnx (add metadata via `add-model-metadata.py`, generate
proper tokens) and **verifying the load with the sherpa-onnx Python API before shipping**. That
verified model can then be re-enabled in `NEMO_MODELS` (or dropped in via a future bring-your-own
nemo_ctc discovery). Dolphin remains the working fast option for Hindi/Tamil/Telugu/Bengali/etc.
(confirmed transcribing in the same log). Malayalam is still the one gap.

### Round 26 (2026-08-07) — NeMo-CTC engine: IndicConformer Malayalam/Kannada (the real path)

The owner's search turned up the breakthrough: **`trysem/indicconformer-120m-onnx`** (a copy of
`sulabhkatiyar/...`, **CC-BY-4.0**) publishes AI4Bharat IndicConformer for **all 12 Indian
languages as plain CTC ONNX** — one `model.onnx` (~493 MB) + `vocab.json` per language, including
**Malayalam (`ml`) and Kannada (`kn`)**. This **bypasses the blocked NeMo→sherpa export entirely**:
the weights already exist in ONNX, and the interface (`audio_signal [B,80,T]` + `length` → CTC
logits) is the standard NeMo Conformer-CTC that sherpa-onnx supports.

- **New `nemo_ctc.rs` engine** (`EngineKind::NemoCtc`), wired through the facade. sherpa-rs has no
  safe wrapper for the NeMo-CTC config, so it's built with **raw `sherpa-rs-sys` FFI** (added
  `sherpa-rs-sys` as an optional dep under the `moonshine` feature; recognizer lifecycle mirrors
  sherpa-rs's own `dolphin.rs`). Catalogue: `indicconformer-ml`, `indicconformer-kn` (downloadable).
- **`vocab.json` → `tokens.txt`** is generated on download (`write_tokens_from_vocab`): vocab
  tokens in order, blank appended at id = `len(vocab)` (IndicConformer's CTC blank convention).
- Frontend: `ipc.ts` engine union adds `nemoctc`; `SettingsView` shows the specific language per
  model ("Malayalam only …") and treats it as no-language-selection.
- **Verified: compiles (`cargo check --features moonshine` clean), 43 tests pass, `next build`
  typechecks.** NOT verified end-to-end — needs the owner's 493 MB download + a Malayalam mic test.
  **Two known risks, both visible in `aurascribe.log`:** (1) sherpa may reject the ONNX at load if
  it lacks sherpa metadata → the load logs "could not create recognizer"; (2) if it loads but the
  blank/token mapping is off, output will be garbled → we adjust `tokens.txt` generation. Either is
  a tight iterate loop now that the weights exist. If it works, the owner can repackage it as a
  proper sherpa-onnx model and contribute it to #3199 — the ideal ending.

### Round 25b (2026-08-07) — Malayalam via Whisper `small`; per-model language coverage in the UI

Owner: "I want Malayalam, completely, from our side" + "the UI doesn't say which languages each
model covers." Both addressed:

- **Malayalam works now (via Whisper).** Re-added Whisper **`small`** (multilingual, 466 MB) to
  `asr.rs::MODELS` — it covers all 99 Whisper languages, including **Malayalam and Kannada** which
  Dolphin/Parakeet/Moonshine all miss. It is slower (~1.5× CPU, so the model list warns), and it's
  the honest bridge until a fast IndicConformer export exists. For a multilingual Whisper model the
  Language selector stays active, so the user picks `ml`. Download URL verified live (487 MB).
  Recommendation is unaffected (still speed-first → `moonshine-base-en`); Whisper `small` carries a
  "slower than real time" warning by the existing rule.
- **Per-model language coverage in Settings.** `SettingsView` now shows a plain-language line under
  each model (`modelLanguages()`): Moonshine = English; Dolphin = Hindi/Tamil/Telugu/... *not*
  Malayalam/Kannada; Parakeet = 25 European; Whisper `small` = all 99 incl. Malayalam. Directly
  fixes "the user can't tell which model does which language."
- Verified: 43 tests pass, `cargo check --features moonshine` clean, `next build` typechecks. The
  owner confirmed the #3199 post is live. Dolphin download URLs verified reachable (a real Dolphin
  transcript is still an owner mic test). Fast Malayalam (IndicConformer) remains the tracked goal.

### Round 25 (2026-08-07) — Dolphin engine: fast local dictation in ~40 Asian languages

Reading sherpa-onnx discussion #3199 (where the owner was about to post) surfaced a ready-made
answer for Indian languages that skips the whole IndicConformer export grind: **Dolphin**
(DataoceanAI/Tsinghua), a multilingual **CTC** model already published in sherpa-onnx format and
exposed by `sherpa_rs::dolphin`.

- **New `dolphin.rs` engine** (`EngineKind::Dolphin`), wired through the facade exactly like
  Moonshine/Parakeet. Downloadable model `dolphin-base-multilang` = a single `model.int8.onnx`
  (~104 MB) + `tokens.txt` from `csukuangfj/sherpa-onnx-dolphin-base-ctc-multi-lang-int8-2025-04-02`.
  Auto-detects language (returns `.lang`). Covers **Hindi, Tamil, Telugu, Bengali, Urdu, Marathi,
  Gujarati, Punjabi, Odia** + more — ~40 Asian languages, on CPU, light (~105 MB).
- **Honest gap it does NOT close:** **Malayalam and Kannada** are absent from Dolphin
  (`DataoceanAI/Dolphin/languages.md`). Those two still need IndicConformer — the owner's upstream
  post (`docs/CONTRIB-indicconformer-sherpa-onnx.md`) targets exactly that gap.
- Frontend: `ipc.ts` engine union adds `dolphin`; `SettingsView` treats Dolphin as auto-detect and
  shows its real language list (Hindi/Tamil/... not the European one).
- Verified: `cargo check --features moonshine` clean, 43 default tests pass, `next build` typechecks.
  A real Dolphin voice transcript is an owner step (needs the download + a mic).

### Round 24 (2026-08-07) — click-to-stop focus bug; overlay jitter; tiny re-added; Colab fixes

Owner testing found real issues:

- **Click-to-stop pasted nothing (hotkey-stop worked).** Root cause: clicking the overlay moved
  foreground focus, and injection targets the focused window, so the paste went to the overlay
  instead of the app being dictated into. Fixed properly: `start_recording` now captures the
  foreground window (`system::capture_foreground_window`, stored in `AppState::target_window`),
  and the injection task calls `system::focus_window` to restore it right before pasting. No-op on
  the hotkey path (focus never moved), so no regression. `WS_EX_NOACTIVATE` stays as a first line
  of defence; this is the reliable backstop.
- **Overlay hover jitter.** The pill resized when the label changed ("Listening…" → "Stop"),
  shifting it under the cursor. Fixed with a fixed-width pill + fixed-width label box in
  `overlay/page.tsx` — content changes, geometry doesn't.
- **Re-added `moonshine-tiny-en`** as the genuinely-light option (110 MB) per owner request; the
  Whisper `tiny.en` stays removed. Recommendation is still speed-first → `moonshine-base-en`.
- **Colab notebook fixed** (`scripts/export_indicconformer_colab.ipynb`): the export cell used a
  non-existent `export-onnx.py`; corrected to the real **`export-onnx-transducer-non-streaming.py`**,
  dropped the redundant quantise cell, and added a step-by-step "download the zip / unzip into
  %LOCALAPPDATA%\AuraScribe\models" section.

  **Owner ran it (Round 24b) → two more fixes:** (1) a mangled `\n` escape in cell 1 broke that
  cell's `print`, so `LANG`/`OUT_DIR` were never defined and *every* later cell `NameError`'d —
  regenerated ASCII-only, no escape sequences. (2) The real blocker: the sherpa-onnx script loads
  via `nemo_asr.models.ASRModel.from_pretrained(model_name=...)`, which treats the arg as a HF repo
  id and rejects a local path (`HFValidationError: ... './model.nemo'`). The notebook now **patches
  that line** to `restore_from(restore_path=...)` when the arg ends in `.nemo`, so a local
  checkpoint loads.

  **Owner ran again (Round 24c):** the loader patch worked, but `restore_from` then hit
  `KeyError: 'dir'` in `_setup_monolingual_tokenizer` — a **known** AI4Bharat issue (confirmed via
  HF "Can't load model" discussions): their checkpoints load only with **their NeMo fork**
  (`git clone AI4Bharat/NeMo && git checkout nemo-v2 && bash reinstall.sh`), not vanilla NeMo.
  Notebook cell 2 updated to install the fork; loader patch switched to the concrete
  `EncDecHybridRNNTCTCBPEModel`. **This is the genuine frontier** — the fork install is heavy and
  the sherpa export may still need fixes; nobody has published a sherpa-onnx IndicConformer yet.
  **Decision (owner): pause the Colab grind and file the blocker upstream as the contribution.**
  Wrote `docs/CONTRIB-indicconformer-sherpa-onnx.md` — a ready-to-post write-up (partial recipe +
  the `KeyError: 'dir'` blocker + focused questions) for sherpa-onnx discussion #3199 and the
  AI4Bharat repos. The app side is complete and proven; IndicConformer/Malayalam is now tracked,
  waiting on either a community answer or AI4Bharat publishing sherpa-onnx-ready ONNX. The Colab
  notebook (with the fork route) stays in `scripts/` for whenever the export path is unblocked.

### Round 23 (2026-08-07) — removed tiny models; speed-first recommendation; IndicConformer Colab

Diagnosed the "tiny models not working" report using the new log: **all four models actually
produced text** (`tiny.en` 37 chars, `moonshine-tiny-en` 53, `moonshine-base-en` 89, `parakeet`
50–98/chunk at ~300 ms). So the tiny models weren't broken — just the least accurate, which felt
like "not working" next to base/Parakeet. Owner chose to **remove them**.

- **Removed `tiny.en` and `moonshine-tiny-en`.** The Whisper catalogue (`asr.rs::MODELS`) is now
  empty (the engine stays as a fallback); the Moonshine catalogue is just `moonshine-base-en`.
  Shipped catalogue = moonshine-base-en (English) + parakeet-v3 (European) + bring-your-own
  transducer bundles. Default model + `page.tsx` unchanged (`moonshine-base-en`). Empty-catalogue
  edge cases handled: `asr.rs` recommendation tests made empty-safe; `Settings::default` no longer
  references `tiny.en`.
- **Recommendation is now speed-first** (`engine.rs`): among models that keep up, pick the
  *fastest* (tie-break accuracy) → `moonshine-base-en` (~0.15×) rather than the heavier
  `parakeet-v3` (~0.5×). Matches the product's speed-first priority and the owner's.
- **Proven: the transducer engine works end-to-end** (Parakeet transcribed on the owner's machine
  at ~300 ms/chunk — in the log). This de-risks Indic: IndicConformer is the same engine.
- **IndicConformer Colab notebook** (`scripts/export_indicconformer_colab.ipynb`, valid JSON,
  generated + validated): downloads the **public** per-language `.nemo`
  (`objectstore.e2enetworks.net/indicconformer/models/indicconformer_stt_<lang>_hybrid_rnnt_large.nemo`
  — no HF gate), runs sherpa-onnx's hybrid NeMo→onnx exporter, packages encoder/decoder/joiner +
  tokens for drop-in. **Best-effort, untested** (can't run NeMo in the sandbox; AI4Bharat needs a
  NeMo fork; the hybrid export isn't a solved community path) — honest caveats are in the notebook
  and `docs/INDIC-CONFORMER.md`. Once a bundle is produced it drops into the models folder and is
  auto-discovered by the transducer engine (Round 21).

### Round 22 (2026-08-07) — observability: file logging + transcription diagnostics

The owner reinstalled v0.4.1, downloaded **parakeet-v3-multilingual** (all 4 files present, verified
on disk), and reported **no transcription** and that **"processing" wasn't visible**. Diagnosis was
impossible because **logs went only to stdout** — and a release build has no console
(`windows_subsystem = "windows"`), so `aurascribe.log` never existed. Fixes this round:

- **Persistent file logging** (`main.rs`, no new dependency): a custom `MakeWriter` tees `tracing`
  to `%LOCALAPPDATA%\AuraScribe\aurascribe.log` (truncated when >5 MB, session header on start),
  in addition to stdout. This is what makes "why isn't the model transcribing?" answerable — the
  file is what the owner can read/share. `get_log_file_path` already pointed here.
- **Transcription diagnostics**: `parakeet.rs`/`moonshine.rs` now log each chunk's sample count,
  output length, and time; the transducer logs model-load start/finish and a loud WARN when it
  returns empty text. So the log now says whether the model loaded, how long inference took, and
  whether it produced anything.
- **Processing indicator is NOT broken.** Verified by previewing `/overlay` with a forced
  `is_processing` state: it renders "Processing…" with the spinner. `DictateView` also shows
  Processing. The likely perception issue: with live chunking on the fast engines, most work
  happens *during* recording, so the post-stop "Processing" phase is brief — and if Parakeet was
  fed **Malayalam (which it does not support — it's 25 European languages)** it would return
  empty, flashing straight to "No speech detected." The new log will confirm which case it is.

**Owner step:** reinstall this build, dictate once (in English to test Parakeet, since it can't do
Malayalam), then read `%LOCALAPPDATA%\AuraScribe\aurascribe.log` — it will show the model load and
the transcribe result. Malayalam still needs the IndicConformer bundle (Round 21 / INDIC-CONFORMER.md).

**Follow-up (same round): engine-aware Language control.** The Settings "Audio and language"
selector was misleading — Parakeet auto-detects and the backend *ignores* the manual language
choice for it, yet the UI still offered a dropdown (incl. Malayalam, which Parakeet can't do).
`SettingsView` now derives `langMode` from the loaded model's engine: **auto** (Parakeet / custom
transducer — shows "Detected automatically" with the 25 languages as a tooltip), **english**
(Moonshine / tiny.en — shows "English"), or **manual** (a multilingual Whisper model — keeps the
dropdown). Backend behaviour was already correct (Parakeet's `transcribe` ignores the language
hint); this just makes the UI honest about it.

### Round 21 (2026-08-07) — overlay click-to-stop; fast multilingual (Parakeet + bring-your-own)

Two features, all verified by `cargo test` (45 pass) + `cargo check` (default and `--features
moonshine`) + `next build`/typecheck. Runtime click behaviour and any *voice* transcript remain
owner steps (need the release app + a mic).

1. **Overlay click-to-stop.** Clicking the "Listening…" pill stops dictation (previously only the
   hotkey could). The subtlety that makes this safe: on Windows a click would normally make the
   overlay the foreground window, and since injection pastes into whatever is focused, the
   transcript would land *in the overlay*. Fixed by marking the overlay `WS_EX_NOACTIVATE`
   (`overlay.rs::make_non_activating`) so it receives the click without ever stealing focus. The
   overlay page (`src/app/overlay/page.tsx`) shows a stop affordance on hover and calls
   `stop_recording`.

2. **Fast multilingual — the transducer engine (`parakeet.rs`).** Added on the sherpa-onnx engine
   we already link. Two things it serves:
   - **Parakeet-TDT-0.6b-v3** (built-in, downloadable from `csukuangfj/sherpa-onnx-nemo-parakeet-
     tdt-0.6b-v3-int8`): 25 European languages, auto language detection, ≥ Whisper large-v3
     accuracy, ~0.5× CPU. Loaded via sherpa-rs's `transducer::TransducerRecognizer`.
   - **Bring-your-own transducer bundles:** `parakeet.rs::custom_models` auto-discovers any
     sherpa-onnx transducer bundle (encoder/decoder/joiner + tokens.txt, int8 or full) dropped into
     the models dir, and lists it as a selectable multilingual model. `engine_of` became an
     instance method so the facade can route these disk-discovered ids to the transducer engine.
     Detection is unit-tested (and proven not to mis-detect a Moonshine bundle, which has no joiner).

   **This is the answer to Hindi/Malayalam** without a cloud call or a heavyweight Python server.
   The user researched Soniox/Speechyou (cloud — rejected, they violate the local-first
   non-negotiable) and VEXYL-STT (local but a ~3 GB Python/PyTorch server — too heavy to embed).
   Decision (owner, this round): run AI4Bharat **IndicConformer** natively in our engine instead.
   The model side is a one-time NeMo→sherpa-onnx export documented in **`docs/INDIC-CONFORMER.md`**;
   once exported and dropped in the models folder, it's auto-discovered and works. **Not yet
   verified end-to-end** — the export needs a Python/NeMo env + the gated model + a real-audio
   benchmark, which the sandbox can't do. Speed ceiling is honest: faster-than-real-time on a good
   CPU, not Moonshine-instant (no free model is, for these languages).

   SenseVoice (CJK) is reachable the same way via `sherpa_rs::sense_voice` if wanted later; it does
   not cover Malayalam so it was not added.

### Round 20 (2026-08-07) — v0.4.1: Moonshine-first, Words/Snippets wired, onboarding, Glass default

Six changes this round, all verified by `cargo test` / `cargo check` (default **and**
`--features moonshine`), a clean `next build` + typecheck, and a browser preview of the new UI.
A real Moonshine *voice* transcript and the release install remain owner steps (Moonshine only
runs in a release build; installing needs the UAC prompt).

1. **Model catalogue trimmed; Moonshine is the default.** Removed Whisper `base.en` and `base`
   (multilingual) from `asr.rs::MODELS` — `moonshine-tiny-en` is faster *and* more accurate on
   English, so a Whisper `base` model was strictly worse. Kept `tiny.en` as the smallest
   fallback. The **Recommended** badge is now computed in `engine.rs::list_available_models`
   **across both engines** (was per-engine, which would show two badges); it picks
   `moonshine-base-en`. Default `whisper_model` is now `moonshine-base-en` (feature-gated: a
   Whisper-only build falls back to `tiny.en`). Multilingual is intentionally deferred to the
   next SenseVoice/Parakeet engine rather than kept on slow Whisper `base`.

2. **Words + Snippets actually apply now (was a stored-but-dead feature).** `stop_recording`
   previously did transcribe → `cleanup::clean` → inject, with **no** dictionary/snippet step —
   the Words and Snippets screens saved entries that never touched dictation. New module
   `expand.rs` applies the personal dictionary (whole-word, optional case-sensitive) then snippet
   expansions (whole-phrase, case-insensitive, longest-trigger-first, inserted verbatim), loaded
   from the DB once per dictation and run after cleanup, before injection. Runs even when cleanup
   is off. 8 unit tests. This is the classic "looks done, does nothing" trap the docs warn about.

3. **First-run onboarding.** New `Onboarding.tsx` — a 5-step walkthrough (welcome · pick a model ·
   the hotkey · Cleanup/Words/Snippets · appearance) shown over the app on a fresh install, with
   Skip / Back / Next and a final "Start dictating". Gated on a new `onboarded` setting.

4. **Glass is the default appearance.** Fresh installs open in Glass; returning users keep their
   theme. **How first-run is detected:** `Database::new` captures `is_fresh = !db_path.exists()`
   *before* creating the file; on a fresh DB it runs `UPDATE settings SET theme='glass',
   onboarded=0`. Migration `006_onboarding.sql` adds `onboarded` with **DEFAULT 1** so existing
   installs are backfilled to "already onboarded" and never nagged or re-themed. (SQL alone can't
   tell a fresh DB from an old one — every migration runs at first launch — hence the Rust check.)

5. **Cleanup verified.** All 13 `cleanup` tests pass; filler removal works and stays behind its
   toggle. No change needed beyond confirming it.

6. **Version + docs.** Bumped 0.4.0 → **0.4.1** across `package.json` / `Cargo.toml` /
   `tauri.conf.json`, fixed the hardcoded "v1.0" in `WidgetRail.tsx`, added
   `docs/RELEASE-NOTES-v0.4.1.md`, and refreshed the README model table + first-run steps and the
   EXPLAINER pipeline/model sections. (Rationale for the bump is in the at-a-glance list above.)

**New wire fields:** `Settings.onboarded` (Rust `commands.rs` + `db.rs` `SettingsRow` +
`src/lib/ipc.ts`). The `settings_round_trip` wire-format test now asserts it.

### Round 19 (2026-08-06) — ROOT CAUSE of the recurring "old UI" (process fix)

The owner repeatedly saw the OLD UI after changes. **It was never a UI code bug.** Root cause:
an old copy installed at `C:\Program Files\AuraScribe` is what Windows launches when the owner
opens AuraScribe normally (shortcut/taskbar/Start). Fresh builds were being launched *directly*
from `target\debug` / `target\release` — a different exe — so the owner's shortcut kept opening
the stale installed copy. The single-instance guard (`dev.aurascribe.app`) made whichever copy
launched first win, compounding the confusion.

**Permanent fix applied:** installed v0.4.0 elevated (`Start-Process setup.exe -ArgumentList /S
-Verb RunAs`, owner accepts one UAC prompt), which OVERWRITES `C:\Program Files\AuraScribe` with
the new build + bundled DLLs. Fixed the stale HKCU `Run\AuraScribe` autostart value (it pointed
at `target\release`) to the installed exe. Verified: the INSTALLED Program Files app runs the new
UI with `moonshine-base-en` Active, no crash — end-to-end proof the bundled installer works.

**PROCESS RULE (do not repeat the mistake): to show the owner a change, rebuild the installer
and reinstall (elevated, replacing Program Files) — never launch a loose `target\*` build. There
must be exactly ONE AuraScribe installed.**

### Round 18 (2026-08-06) — v0.4.0 release: Moonshine bundled into the installer

Cut release **v0.4.0** (follows v0.3.0; version files aligned from an inconsistent 1.0.0 to
0.4.0 across Cargo.toml, package.json, tauri.conf.json). The Moonshine engine now ships in the
installer: a Moonshine-only config overlay (`src-tauri/tauri.moonshine.conf.json`) declares the
sherpa-onnx + ONNX Runtime DLLs as bundle resources, wired into `moonshine-build.bat` via
`--config` so the default Whisper installer is unaffected. Build with `moonshine-build.bat`.

Verified: the build produces `AuraScribe_0.4.0_x64-setup.exe` (~8.7 MB, up from the 4.6 MB
Whisper-only build because it bundles the ONNX Runtime), and the generated NSIS script
(`target/release/nsis/x64/installer.nsi`) installs `onnxruntime.dll`,
`onnxruntime_providers_shared.dll`, `sherpa-onnx-c-api.dll`, `sherpa-onnx-cxx-api.dll` to
`$INSTDIR` beside the exe — where the loader finds them. Release notes:
`docs/RELEASE-NOTES-v0.4.0.md`. Tag `v0.4.0` created locally. **Publishing (git push + GitHub
release) is done by the owner — the sandbox blocks network git here.** (The installed-app run
was verified in Round 19; a real voice transcript is still an owner step.)

### Round 17 (2026-08-06) — Moonshine RUNS on Windows (release); UI highlight fixes

**Moonshine works on Windows in a release build — verified by running.** The Round-14
`0x80000003` abort is a `_CrtIsValidHeapPointer` **debug-only** heap assertion caused by
sherpa-onnx-c-api.dll's STATIC CRT (/MT) vs the app's DYNAMIC CRT (/MD). Root cause confirmed
by dumpbin (sherpa dll has no VCRUNTIME140/MSVCP140 dep; onnxruntime.dll does). A `/NODEFAULTLIB`
link hack fixed debug but broke the release link (mainCRTStartup) — reverted. The key insight:
**release builds don't run that debug-heap check.** Verified end-to-end: `tauri build
--features moonshine` (release) launches, survives 60s idle, downloads + LOADS `moonshine-tiny-en`
(onnxruntime inits, model goes Active) with no crash. So: **build/run Moonshine in RELEASE**
(`moonshine-build.bat`), not the debug dev server (which still trips the harmless assertion).

**NOT yet verified:** a real Moonshine transcript (needs a human mic test) and the on-CPU
speed number. **Remaining to ship:** bundle `onnxruntime.dll`, `onnxruntime_providers_shared.dll`,
`sherpa-onnx-c-api.dll` (in `target/release`) into the NSIS installer via `tauri.conf.json`
(resources/externalBin) so a distributed installer finds them; the built `.exe` beside those
DLLs already works locally.

**UI:** toggles switched to an indigo accent (were invisible/black); native `<select>` popups
themed (were OS-black); active nav + Tap/Hold selection use an indigo glass frost (were muddy
brown `bg-accent`). Heavy Whisper models (`small`, `large-v3-turbo`) were added then removed —
too slow on CPU; rule saved: models must be light + fast + accurate (Moonshine is the path).

**Process:** run only ONE app instance (single-instance guard). The old installed copy at
`C:\Program Files\AuraScribe` is an out-of-date "first version" UI that surfaces if a build
isn't running — owner is uninstalling it via Windows Settings → Apps.

### Round 16 (2026-08-06) — Reverted heavy models; toggle + select theming fixed

- **Reverted the Round-15 heavy models.** `small-q5_1` / `large-v3-turbo-q5_0` were far too
  slow on CPU (the app even warned ~1 min for 30 s of speech). Removed. **Standing rule now
  recorded in memory (`models-must-be-light-fast-accurate`): every model must be light + low
  latency + accurate at once. Do not offer bigger Whisper models as an "upgrade." The path to
  better accuracy is a fast engine (Moonshine), not a heavier model.**
- **UI fixes (verified by screenshotting the running app).** The `Toggle` used `bg-foreground`
  for the checked track — light in dark/glass (invisible white-on-white) and solid black in
  light mode. Now indigo when on, subtle neutral when off: visible in every appearance. Native
  `<select>` option popups defaulted to OS black; now themed to the popover tokens, navy in
  glass. Verified: toggle renders indigo, model list shows only the light models.
- Verification method worth reusing: `tauri build --debug --no-bundle` for a fast standalone
  exe, then a PrintWindow (PW_RENDERFULLCONTENT=2) capture of the app window to see the real
  render. Only ONE app instance may run (single-instance guard) — stop others first.

### Round 15 (2026-08-06) — Multilingual Whisper models added; version-confusion cleared

Added `small-q5_1` (190 MB) and `large-v3-turbo-q5_0` (574 MB) to the Whisper catalogue —
both multilingual, both running through the existing whisper.cpp engine (no new native code,
no crash). They are the pragmatic "next version" accuracy/multilingual upgrade that works on
Windows now; the Moonshine speed engine remains blocked on the Round-14 CRT fix. On a CPU
these bigger models are more accurate but slower; the model list's per-clip wait/warning makes
the trade explicit. `cpu_cost` values are estimates pending a real benchmark.

**Process note / a scare worth recording.** While iterating, the running app was swapped
between builds (I stopped the running `target/release` exe to test the Moonshine dev build,
which crashed). With those gone, a **stale installed copy at `C:\Program Files\AuraScribe`**
surfaced — an older "first version" UI — and the owner reasonably thought the interface had
been reverted. Nothing was lost (git clean, all UI source intact); it was the old install
plus the app's single-instance guard colliding across multiple builds. **Fix going forward:
keep ONE app.** The owner should uninstall the Program Files copy (Settings → Apps); dev/test
should not run while another build is running. Also: the dark-glass only appears in
Settings → Appearance → **Glass** (it is an appearance mode, not the default), and a built exe
only reflects source as of its build time — the confusion started because the running exe
pre-dated the glass commit by an hour.

The app currently in front of the owner is a **debug bundle** (`tauri build --debug`,
`target/debug/aurascribe.exe`) — functional but slower than a release build; use `build.bat`
for a real release/benchmark.

### Round 14 (2026-08-06) — Moonshine engine built (feature-gated); dark-glass; history cleaned

Three things this round: the Moonshine engine from Round 13's plan is now actually built,
the Glass appearance was reworked for the owner's dark backdrop image, and the git history
was cleaned of the Claude co-author trailer.

**Moonshine engine — built and integrated, behind the `moonshine` Cargo feature.** Whisper
stays the v1 default engine, untouched; Moonshine is the new, faster, lower-latency option
layered on top via `sherpa-rs 0.6.8` (ONNX Runtime via sherpa-onnx). Landed as small commits,
each `cargo check`-verified:

1. `EngineKind {Whisper, Moonshine, Parakeet}` tagged onto every model + `ModelInfo`, mirrored
   in `src/lib/ipc.ts`. Additive; behaviour unchanged.
2. `sherpa-rs` optional dep + `moonshine` feature + feature-gated `moonshine.rs` (`MoonshineASR`).
   `download-binaries` fetches a prebuilt, cached sherpa-onnx + onnxruntime — no CMake fight.
3. Moonshine catalogue (`moonshine-tiny-en`, `moonshine-base-en`, int8 ONNX) + a downloader
   that pulls the five model files flat from HuggingFace (`csukuangfj/sherpa-onnx-moonshine-*`),
   no archive/unpack, reusing the existing reqwest streaming path.
4. `engine::Asr` facade — one type over both engines, routing list/download/load/transcribe/
   delete by `EngineKind`. `AppState.asr` is now `Arc<Asr>`. Compiles both with and without
   the feature, warning-clean.

Plus `moonshine-dev.bat` / `moonshine-build.bat` (mirror dev/build with `--features moonshine`).

**What is verified:** the code compiles in both configurations (`cargo check` default and
`--features moonshine`, both clean). The sherpa-onnx prebuilt libs download and link.

**RUNTIME FINDING (2026-08-06, found by running `moonshine-dev.bat`): the Moonshine build
crashes on startup — NOT usable yet.** The debug build compiled and linked
(`Finished ... 3m50s`), the app launched and served the UI (`GET / 200`), then aborted with
`exit code: 0x80000003` (STATUS_BREAKPOINT) and **no Rust panic** — i.e. a native abort. The
link step carried `LINK : warning LNK4098: defaultlib 'msvcrt' conflicts with use of other
libs`, which appears only with the `moonshine` feature. Diagnosis: a **CRT mismatch** between
the prebuilt sherpa-onnx / ONNX Runtime and whisper.cpp + Rust; the abort fires when the
startup auto-load of the Whisper model allocates across the mismatched CRT boundary (the
debug heap trips `__debugbreak`). This is a Windows native-linking issue, not an engine-code
bug — the Rust integration is correct and compiles.

**Two Windows-integration tasks remain before Moonshine runs/ships (next focused work):**
1. **Unify the CRT.** Determine which CRT the sherpa-onnx prebuilt uses (dumpbin on the libs
   in `~/.cargo`/the sherpa-rs-sys OUT_DIR, or sherpa-rs Windows docs) and match everything to
   it — either force whisper.cpp's CMake to that runtime (`CMAKE_MSVC_RUNTIME_LIBRARY` via a
   toolchain file) and/or set Rust `crt-static` accordingly. Verify the LNK4098 warning is
   gone AND the app runs without aborting. Do not guess the direction — inspect the prebuilt.
2. **Bundle `onnxruntime.dll`** (and any sherpa dll) into the Tauri bundle so the installed
   release can find it at runtime (`tauri.conf.json` resources/externalBin). In dev the build
   script drops the dll in `target/debug`, so dev "finds" it; the NSIS installer will not
   include it without config.

**Still NOT verified:** no real Moonshine transcript or benchmark — blocked by the crash
above. Once it runs, benchmark on an idle CPU and replace the `cpu_cost`/accuracy placeholders
in `moonshine.rs`.

**The open decision — installer size vs. shipping Moonshine.** `sherpa-onnx` links the ONNX
Runtime into the binary at build time, adding well over 10 MB. Enabling `moonshine` in the
default feature therefore breaks the ~4.6 MB "stay lightweight" non-negotiable. So it was
**deliberately left opt-in. Owner's decision (2026-08-06): keep it opt-in for now** — try it
via `moonshine-dev.bat` and confirm a real transcript + benchmark first, then revisit shipping.
Do not flip `moonshine` into `default` until that revisit. Later options remain: (a) ship
Moonshine by default with a larger installer; (b) ship two installers (lite vs. full).

**Still not done (deferred, not forgotten):**
- Silero VAD recovery (sherpa ships it) for auto-trim / auto-stop — Round 13 step 4.
- Parakeet + DirectML multilingual/GPU tier — Round 13's follow-on.
- A "New · Moonshine" badge in the Settings model list (the `engine` field is already on
  `ModelInfo`; the UI just doesn't surface it yet).
- `should_chunk` returns true for Moonshine via the facade, but Moonshine `cpu_cost` is an
  estimate until benchmarked.

**Dark-glass appearance.** The supplied `public/glass-bg.jpg` is a dark navy image; the old
Glass used dark text on a frosted-white veil, which assumed a bright backdrop and was
unreadable over it. Reworked the `.glass-bg` scope in `globals.css` to macOS-style dark
vibrancy: glass now rides on the dark token set (light text), the shell is a smoked-glass
veil, cards are darker frosted panes, with a gentle top-down darkening for contrast.
`page.tsx` adds the `dark` class in Glass mode. Previewed against the real image.

**Git history cleaned.** The owner did not want Claude listed as a GitHub contributor. All
25 commits + the `v0.3.0` tag were rewritten to drop the `Co-Authored-By: Claude` trailer;
the code tree is byte-identical to the pre-rewrite backup (`backup-pre-coauthor-rewrite`,
local). **The force-push was blocked by the sandbox — the owner must run
`git push --force origin master v0.3.0` themselves.** Going forward, commits in this repo
omit the trailer (recorded in the memory file `no-claude-coauthor-trailer`).

### Round 13 (2026-08-06) — Glass appearance corrected; the speed-engine plan (READ BEFORE NEXT BUILD)

**This round is mostly a plan.** The owner is continuing in a new chat to build the speed
engine. The next section is the brief for that work.

**Glass appearance, corrected.** It was wrong before — a shrunken box on a glow. Now it is
**full size like every other mode**; what changes is the *material*: a bluish backdrop fills
the whole window, the shell is a frosted veil, and every card is frosted glass (dark text
stays readable because the frost lightens the blue). Implemented entirely in the `.glass-bg`
scope of `globals.css`; the shell is always full-bleed in `page.tsx`. Selectable at
Settings → Appearance → **Glass** (4th option).

**`public/` folder created.** It did not exist (that is why the owner couldn't find it). Drop
a wide, calm, bluish image at `public/glass-bg.jpg` and it becomes the Glass backdrop
automatically (referenced as `/glass-bg.jpg`; CSS falls back to a blue gradient until then).
CSP already allows `img-src 'self'`, so a local image is fine. See `public/README.md`.

#### The speed engine — research answers + implementation brief

The owner's three questions, answered:

**1. Is Moonshine public or someone's private thing?**
Fully open source, **MIT license**, built by Useful Sensors (now "Moonshine AI"), on GitHub +
HuggingFace. Anyone can use the models and code freely. Handy uses this exact public model —
nothing stops us doing the same.

**2. Is anything faster than Moonshine for us?**
For **our exact case — Windows CPU, short English dictation — Moonshine is the fastest
practical option.** It is purpose-built for CPU/edge, its compute scales with clip length
(perfect for dictation, and it *helps* our chunking), ~5× faster than Whisper, WER better than
`tiny.en`/`base.en`. **Parakeet** posts huge throughput numbers (100–2000× RTFx) but those are
on **GPU / Apple Neural Engine**, not CPU; it is 600M params and heavier. Parakeet's real win
is *multilingual* + accelerator throughput. **SenseVoice Small** is a fast multilingual
alternative. Conclusion: **Moonshine for English speed; Parakeet/SenseVoice for multilingual.**

**3. If we use the same public model as competitors, how do we stand out? Should we build our own?**
Be clear-eyed here: **the models are commodities.** Whisper, Moonshine, Parakeet are all open,
and *every* competitor (Handy, VoiceInk, Spokenly, open-wispr) uses these same public models —
**none of them trained their own.** Training a competitive ASR model costs millions in
compute + data and would violate the free/lightweight promise; it is the one move that would
sink this project. **Differentiation is the product layer, not the model:** latency-hiding
(chunking + silence trim), the local cleanup pipeline (punctuation, question marks, fillers,
dictionary/snippets), injection reliability, the design, and being genuinely free + local +
open. That is where we already build and where we win. Use the best public model like
everyone else, and beat them on everything around it.

##### Implementation brief for the next chat

**Vehicle: the `sherpa-onnx` Rust crate** (k2-fsa). It runs Moonshine, Parakeet, SenseVoice,
Whisper **and Silero VAD** on **Windows x86_64 CPU** via ONNX Runtime, offline. Its build
script auto-downloads a prebuilt lib (no CMake/whisper-style native pain). crates.io:
`sherpa-onnx`; source: github.com/k2-fsa/sherpa-onnx.

Suggested steps, in order:

1. **Add `sherpa-onnx` as an optional engine behind a Cargo feature** (like `whisper`), so the
   existing Whisper path stays intact and shippable while the new one is built.
2. **Introduce an `Engine` abstraction** in `asr.rs`: a trait with `transcribe(&[f32], lang)`,
   implemented by the existing `WhisperASR` and a new `MoonshineASR` (sherpa-onnx). The
   command layer (`commands.rs::transcribe_chunk`) already calls one `asr.transcribe(...)` —
   keep that surface.
3. **Extend the model list** (`MODELS` in `asr.rs`) with an engine tag. Add Moonshine tiny/base
   (English) and one multilingual (Parakeet or SenseVoice). Download URLs are HuggingFace ONNX
   archives, not the ggml `.bin` path — the download logic needs a per-engine URL/format.
4. **Recover Silero VAD** (deleted in the rebuild; in git history as `vad.rs`). sherpa-onnx
   ships VAD, so prefer its built-in. Use it to auto-trim silence and (optionally) auto-stop on
   end of speech — less audio in, snappier feel, on every engine.
5. **Re-run the benchmark** (`tests/transcription.rs`, on an **idle** CPU — see Round 8) to get
   real Moonshine-vs-Whisper numbers on this machine, and update the model `cpu_cost` table +
   recommendation logic from measurement, not estimates.
6. Keep Whisper as the accuracy option; make **Moonshine `base` the default recommendation** on
   CPU once measured.

**Do not** attempt to train or build a bespoke ASR model. Integrate the best open one and win
on the product.

##### GPU acceleration for Parakeet — the correct mental model

The owner asked about integrating Parakeet "which enables their GPU", downloaded in the
background at install. One misconception to clear up first: **a model file does not enable a
GPU. The runtime does.** Parakeet is just weights; whether it runs on GPU depends on which
ONNX Runtime *execution provider* the app is built with.

- **Parakeet runs on CPU too** — it is not GPU-only, just faster with acceleration. So it can
  ship as a universal multilingual option even for GPU-less machines.
- **On Windows, use the DirectML execution provider**, not CUDA. DirectML works on *any*
  GPU (NVIDIA / AMD / Intel integrated) with no CUDA/toolkit install — the same "one build
  serves everyone" property we wanted from Vulkan, but for ONNX. `sherpa-onnx` exposes the
  provider choice.
- **Right architecture:** Moonshine (CPU, fast, default, works everywhere) + Parakeet as an
  opt-in multilingual/accurate engine that (a) downloads its ONNX model on demand and (b)
  uses DirectML when a GPU is present, silently falling back to CPU when not. Detect the GPU
  at runtime; never *require* one. This keeps the "works on every device" promise while
  giving GPU owners a speed/accuracy bump.

This is a follow-up to the Moonshine integration, not a prerequisite. Sequence: Moonshine
first (the universal CPU win), then Parakeet + DirectML as the GPU-accelerated tier.

### Round 12 (2026-08-06) — full-bleed shell, disk cleanup, competitor speed analysis

- **Redesign is full-bleed now.** The owner found the 28px "box on a glow" inset too boxed-in.
  The app fills the frameless window edge to edge; the floating-on-glow look became an opt-in
  **"Glass" appearance** (Settings → Appearance, a 4th option beside Light/Dark/System). Glass
  applies a `.glass-bg` glow to the backdrop and re-insets the panel; a background image
  dropped into `public/` later can replace the CSS gradient (the owner will supply images).
- **Reclaimed 20 GB.** `src-tauri/target/debug` (20 GB, only used by `dev.bat`) plus `.next`
  and `out` were deleted — all rebuildable caches. `target/release` (4.8 GB) is kept so
  `build.bat` stays fast. First `dev.bat` after this recompiles whisper.cpp once.

#### Competitor analysis — how they beat us on CPU speed, and the fix

The owner asked why the app is CPU-heavy/slow versus Handy, VoiceInk, Spokenly, open-wispr,
etc. The answer is concrete and actionable:

**We only ship Whisper, which is the *slowest* CPU option.** Handy (17–20k stars, and built
on the exact same Tauri + Rust + React stack we use) offers three engines:

| Engine | CPU speed | Notes |
|---|---|---|
| **Moonshine V2** | **~5× faster than Whisper** | English, 31–192 MB, WER *better* than `tiny.en`/`base.en`. Compute scales with audio length, not Whisper's fixed 30 s window. |
| **Parakeet V3** (NVIDIA) | ~5× real time, CPU-only | multilingual, auto language detect |
| Whisper | slowest on CPU | what we have today |

**Moonshine is the headline fix for the owner's exact complaint.** It runs via ONNX Runtime
(Rust: the `ort` crate) on CPU, is smaller than `base.en`, *more* accurate on English, and
—crucially— its compute scales with clip length, so it synergises with our chunking + silence
trimming instead of fighting them (Whisper's 30 s window is what made chunking wasteful for
slow models). A Moonshine engine + our existing chunking/trim would make CPU dictation feel
near-instant without a GPU.

**Recommended roadmap to compete (in order):**
1. **Add Moonshine (ONNX via `ort`) as a second engine**, selectable per the model list. This
   is the single biggest speed win and directly answers "too CPU-heavy". Large but well-trodden
   — Handy did exactly this in our stack.
2. **Recover Silero VAD** (was deleted as dead code in the rebuild; recover from git). Auto-trim
   silence and auto-stop on end-of-speech — less audio to process, snappier feel.
3. **Parakeet V3** as the multilingual fast engine, replacing the removed large models for
   non-English.
4. GPU (Vulkan) stays parked as the accuracy-at-speed option for machines that have one.

Sources: Handy (github.com/cjpais/Handy), Moonshine (github.com/moonshine-ai/moonshine).

**Not yet started** — all of the above is analysis + roadmap; no ASR-engine code was written
this round. The owner should decide whether to invest in the Moonshine backend next.

### Round 11 (2026-08-06) — "warm glass" visual redesign

Implemented the owner's redesign from a Claude Design project (`AuraScribe App Redesign.dc.html`,
pulled via the DesignSync tool). A full change of visual direction, from the austere
"instrument" to a warm, premium consumer surface — while keeping every screen wired to the
real IPC and honouring the product principles.

- **Look:** a cream glassmorphic panel floating on a deep indigo glow (pure CSS gradient, no
  image asset). Newsreader serif for display, IBM Plex Sans for UI, IBM Plex Mono for machine
  values. Indigo `#4C6FFF` accent replacing cyan; warm red for recording.
- **Frameless window** (`decorations:false`) with a custom titlebar: sidebar-collapse left,
  min/maximize/close right (wired via `@tauri-apps/api/window`), drag via
  `data-tauri-drag-region`. Added the four `core:window:*` permissions to capabilities.
- **New per-tab right widget rail** (`WidgetRail.tsx`) — contextual cards. Deliberately no
  fabricated usage numbers; anything numeric is real state or omitted.
- Restyled tokens (`globals.css`), shell (`page.tsx`), sidebar, shared components (`ui.tsx`),
  every view, and the signal meter.

**Two principled calls, both verified:**

- **Fonts are self-hosted via `next/font`, not the design's Google-CDN `<link>`.** That link
  would be a runtime cloud request — breaking local-first *and* the CSP. next/font downloads
  at build time and serves from origin. Verified: 22 woff2 embedded, zero runtime CDN
  references in the output HTML.
- **The glow is a CSS gradient, not `bg-glow.jpg`** — lighter, scalable, no CSP/asset issues.

Verified rendering in the browser preview (computed glass panel 1304×785 fills the window)
and in the real frameless build (window opens 1573×994, controls present). Default theme
changed to `light` so fresh installs get the cream look; existing installs keep their saved
theme (switch Appearance → Light to see the cream design).

**Open / unverified by a human:** borderless-window edge-resize and snap behaviour on Windows
(should work with `resizable:true`); the cream light theme in the *real* app (confirmed in
browser preview + the dark variant in the real app).

### Round 10 (2026-08-06) — model list trimmed to CPU-viable; product-overview PDF

- **Removed the `large-v3` family** (`large-v3`, `large-v3-turbo`, `large-v3-turbo-q5_0`). On a
  CPU they ran at 2.5×–15× the length of the speech and overheated the machine; a CPU-first
  free product should not list a model that needs a GPU to be usable. The list is now
  `tiny.en`, `base.en`, and `base` (multilingual) — all faster than real time on CPU, so
  dictating in other languages survives without the heavy models. They can be reinstated
  behind a `gpu_enabled()` check once `build-vulkan.bat` succeeds. README/TESTING tables
  updated to match.
- **`docs/AuraScribe-Product-Overview.pdf`** — an 8-page as-built reference for the redesign
  team: every tab, the navigation, layout, interaction states, the current visual system
  (documented honestly, including its deliberate austerity), the accessibility floor that must
  survive any redesign, the technical shape, and a free-vs-fixed table. Regenerate with
  `scratchpad/gen_overview.py` (ReportLab). The owner wants a warmer, more premium redesign
  (gradients, curves, better type) — legitimate, and not in tension with any product principle.

### Round 9 (2026-08-05) — sounds, sidebar, question marks; GPU parked

Three requested UX fixes, plus an honest stop on GPU.

- **Audible start/stop cue (`sound.rs`).** Soft two-note sine chirps — rising on start, falling
  on stop — synthesised with `cpal` (already a dependency, no new weight) rather than the
  harsh Win32 beep the owner said not to use. Plays from the *backend*, because the hotkey
  fires when no window is open, so a frontend sound would not play. The start cue is emitted
  *before* the mic stream opens so it is never captured into the recording. Toggle in
  Settings → Application (migration `005_sound_cues.sql`, default on).
- **Sidebar collapse jitter fixed.** Every row now centres its icon in a fixed 44px slot in
  both states, so collapsing only changes panel width and hides labels — the icons no longer
  slide sideways. The previous version swapped each row between left-padded and
  `justify-center`, which moved the icon's x-position during the width transition.
- **Question marks (`cleanup::fix_question_marks`).** Whisper's small models often emit no
  terminal punctuation, so cleanup was forcing `.` even on questions. It now upgrades a
  sentence's `.`→`?` when the sentence reads as a question: WH-word initial (who/what/why/…,
  excluding "how to"), or an auxiliary + a nearby subject pronoun ("is **this** working") so
  imperatives ("do the dishes") keep their period. High-precision by design — a wrong `?`
  reads worse than a missed one. It cannot split a run-on Whisper never punctuated, and makes
  no attempt at `!` (that needs reading intent, which is an LLM's job, which is banned).

**GPU parked, deliberately.** Five distinct Windows/CMake/Ninja build failures against
whisper-rs 0.16 / whisper-rs-sys 0.15's Vulkan backend; four were fixed (VULKAN_SDK not
inherited → passed explicitly; MSBuild can't build vulkan-shaders-gen → Ninja; CMake 4.4 too
new → VS-bundled CMake 3.31; compiler ABI probe → same). The fifth is an upstream
shader-generator/Ninja path bug in the pinned whisper. `build-vulkan.bat` captures all four
fixes for a one-command retry after a whisper dependency bump. The Vulkan SDK the owner
installed is not wasted. Decision: ship the fast CPU path (`base.en` + silence trimming);
GPU is a want, not the product. **`build-vulkan.bat` and the `vulkan`/`cuda`/`metal` cargo
features stay in the tree as the on-ramp.**

### Round 8 (2026-08-05) — chunking that respects physics, plus silence trimming

Round 7's chunking helped `base.en` but made `large-v3-turbo` and `large-v3` **worse** — the
owner watched turbo process for 13+ minutes. Two causes, both now fixed:

1. **Chunking cannot help a model slower than real time.** While you speak for 60s, turbo
   (~2.5–4×) needs 150–240s; it falls behind every second and the backlog is paid back after
   you stop. Splitting never drains it.
2. **Chunking made slow models *slower*.** whisper.cpp processes fixed 30-second windows, so
   twelve 15s chunks cost twelve windows where one pass costs ~six — roughly double the
   compute on `large-v3`.

**Fix:** live chunking is now gated on `asr::should_chunk(model)` — only models that keep up
(`realtime_factor ≤ 0.8`, i.e. `base.en`/`tiny.en` on CPU, everything with a GPU) are split
during recording. Slower models transcribe once at stop: still slow, but no longer *made*
slow by the extra windows. `MAX_CHUNK_SECS` dropped 24→15 and `MIN` 8→6 to shorten the final
chunk's wait for the models that do chunk.

**Silence trimming (`chunking::trim_silence`).** Whisper is charged for silence exactly like
speech, so leading/trailing gaps and long internal pauses (>600ms) are stripped before
transcription — collapsed to a 150ms pause so sentence rhythm survives. A universal win: less
audio in, less compute, less heat, on every model and every machine. Natural sub-600ms pauses
are preserved so accuracy is untouched. 4 unit tests.

**Warnings hardened.** Red threshold moved 5×→2×: `turbo-q5_0` at 2.5× (a 75-second wait for
30 seconds of speech) now shows red, not amber, and the text states that live processing is
off for models that slow and gives the concrete minute cost *before* download.

**The honest limit, stated plainly:** on a CPU, `large-v3` is a 15× operation. No algorithm
removes that — only a smaller model or a GPU does. "Works on every device" means the fast
models work everywhere; the large ones need a GPU. This is now what the UI communicates.

### Round 7 (2026-08-05) — chunked transcription, and speed measured honestly

**The speed complaint was two separate problems, and only one was about optimisation.**

The owner reported 5-8 minute waits and a laptop hot enough to smell. Their own transcript
history settled it:

| model | processing / speech length | source |
|---|---|---|
| `base.en` | 0.36x - 0.62x | 7 real dictations |
| `large-v3` | 8.8x, 16.6x, **27.2x** | 3 real dictations |

17 seconds of speech took **471 seconds** on `large-v3`. That is not a tuning problem; the
UI offered a 3.1 GB model with a 5/5 accuracy badge and no warning that it was unusable on a
CPU. The user picked the most accurate option, which is the reasonable thing to do.

**Fixes:**

- **Recommendations are computed for the machine**, not static — the most accurate model
  that still runs faster than speech. On CPU that is `base.en`. (`large-v3-turbo-q5_0` was
  wrongly marked "recommended" in Round 5c; at ~2.5x it is slower than speaking.)
- **Warnings shown before download**, carrying the real multiplier.
- **Threads cut from logical to physical cores** (16 -> 8 here). Inference is SIMD-bound, so
  SMT siblings contend for the same vector units: more heat, no more throughput.
- **Builds capped at 6 of 16 jobs** in `dev.bat`/`build.bat`. A thermally limited laptop
  throttles anyway, so the wall-clock cost is small and the machine stays far cooler.

**Chunked transcription (`chunking.rs`).** Audio is transcribed while the user is still
speaking, so the remaining wait is one chunk rather than the whole recording. Cuts are placed
in silence — splitting a word gives Whisper half of it on each side and it mis-transcribes
both. Minimum 8s, ceiling 24s, silence gap >= 250ms, latest qualifying gap preferred for
maximum context. Expected effect on a 30s dictation: ~15s wait becomes ~1-2s.

This is the *universal* answer, which is why it outranked GPU work. Chunking does not make
transcription faster; it removes the waiting, and it does so on any hardware. GPU offload
only helps machines that have a GPU.

**The audio-buffer lock is the hazard to respect.** The cpal callback takes it with
`try_lock` and silently discards samples on failure, so holding it across a transcription
would punch holes in the recording. The chunker drains into a local buffer and releases
before doing any work.

**Benchmarking lesson: this project must be benchmarked on an idle CPU.** Measured right
after a full rebuild, disabling temperature fallback looked like a 12% win. On an idle
machine it is consistently *slower* (~31s vs ~26s, three runs each). One contaminated run
came in at 190s against a true ~26s. Both that change and an AVX2 `CFLAGS` experiment (76s
vs 42s — `/O2` was filtered out while `/arch:AVX2` landed) were reverted.

**Injection now falls back both ways.** Paste is primary for long text, but the owner's
clipboard wedged system-wide mid-session — PowerShell's own `Set-Clipboard` failed too — and
a wedged clipboard would have made dictation silently fail. Paste falls back to typing and
typing to paste, plus retry-with-backoff on `OpenClipboard` for ordinary contention from
clipboard managers.

**Wire-format bug fixed (Round 6 carryover).** Rust serialised `Status`/`Settings` as
camelCase while `src/lib/ipc.ts` and every component read snake_case, so `is_model_loaded`
was permanently `undefined`. The Dictate screen showed "Add a voice model to begin" no matter
what was loaded — **no reinstall could have fixed it** — the sidebar never showed recording
state, and `save_settings` could not deserialise what the UI sent. One mismatch, four broken
features, no error anywhere. `invoke()` returns an unchecked shape, so TypeScript asserted
the type rather than verifying it; two tests now assert the wire field names.

**Still open:** runtime per-machine calibration (the cost table is anchored on one Ryzen and
would mis-recommend on a weak laptop), silence trimming, quantised low-end tier, Vulkan.

### Round 5 (2026-08-05) — the 404 overlay and the window size

The owner reported a **"404 — This page could not be found"** box appearing on top of
everything during dev testing. Root cause found and fixed:

- **`overlay.rs` asked for `overlay/index.html`.** That is correct for the exported bundle
  but wrong for `next dev`, which serves the route as `/overlay/` and returns **404** for
  `/overlay/index.html`. Verified directly against a running dev server:

  | Request | Dev server |
  |---|---|
  | `/overlay/index.html` | **404** |
  | `/overlay/` | 200 |
  | `/` | 200 |

  Next's 404 page then rendered inside a 220×56 transparent, undecorated, always-on-top
  window — which is exactly the undismissable box that was reported. The path is now chosen
  at runtime with `tauri::is_dev()`. Both branches verified: `/overlay/` returns 200 in dev,
  and `next build` emits `out/overlay/index.html` for release.

- **The "overlay refuses to display if its page failed to load" claim was not true.** The
  guard only checked for a localhost URL in a release build, so it caught bug #9 and nothing
  else — including this. Replaced with positive confirmation: the overlay page calls a new
  `overlay_ready` command on mount, and `overlay::show` returns early until that arrives.
  Only the real page can set that flag, so *any* failed load now fails silently instead of
  parking an error box on screen. This is the second time a broken overlay load reached the
  owner; the guard now matches what the docs claim.

- **Testing guidance corrected.** `npm run dev` is **not** the way to run this — it calls
  `tauri dev` without the MSVC / libclang / CMake environment that whisper-rs needs to
  compile. Use **`dev.bat`**. See `docs/TESTING.md` §0.

### Round 5b — the window opened at the wrong size

The owner reported having to drag the window bigger by hand on every launch. **Round 4's
"window is now 1080x720" change had never actually taken effect**, and reading the config
would never have revealed why:

`tauri-plugin-window-state` restored a persisted size on every launch, overriding both the
configured default *and* `minWidth`/`minHeight`. The saved file on the owner's machine held
`505x758` — below the declared 860 minimum — left over from when the default was 480x720.
Every subsequent config change appeared to do nothing, because the saved state always won.

**The plugin is now removed entirely** (dependency, plugin registration, and the
`window-state:default` capability). For a tray app whose settings window is opened and
hidden constantly it bought nothing, and its restored *position* also fought `center: true`
and could park the window off-screen after a monitor change. The window now opens at its
configured size, centred, every launch.

`%APPDATA%\dev.aurascribe.app\.window-state.json` is now unused. It is harmless, and
deleting it is optional.

**Lesson, same shape as the 404:** a declared config value is not an observed one. Round 4
recorded the window size as fixed on the strength of the config diff alone.

### Round 5c — window sized against a measured reference

With the override gone, the size itself was still wrong: 1080x720 was a guess. The owner
pointed at Wispr Flow as the layout reference, so its window was **measured** rather than
estimated from a screenshot — `EnumWindows` + `GetWindowRect` over the running process:

| Window | Physical rect |
|---|---|
| Flow "Hub" (the reference) | **1565x987** at (152, 27) |
| Flow "Status" (its small pill) | 556x660 |
| AuraScribe before | 1152x798 |

At the owner's 101 DPI (1.052x) that makes the reference **≈1488x938 logical** — 81% of a
1920px width, 91% of its height. The default is now **1480x936**.

A fixed default that suits 1080p is too large for a 1366x768 laptop, so `fit_to_screen` in
`main.rs` shrinks the window to the monitor's work area and re-centres when it doesn't fit.
It only ever shrinks; `minWidth`/`minHeight` still apply. This is what makes the size
*scalable* rather than merely large.

**`fit_to_screen` was wrong on its first attempt, and only running it showed that.** It
capped at 90% of the full screen height — a guess. The log said:

```
Window 1573x1025 doesn't fit 1920x1080 monitor; fitting to 1573x972
```

It was shrinking the window below its design size on an ordinary 1080p desktop for no
reason. The real constraint is `Monitor::work_area()` (tauri 2.11.5), the screen minus the
taskbar: **1920x1031** on that machine, which fits 1025 exactly. Now uses `work_area()`, so
the clamp only fires when a window genuinely would not fit.

Also added **`build.bat`**, the release counterpart to `dev.bat`. A bare `npm run build`
fails at `whisper-rs-sys`'s bindgen step with `Unable to find libclang` unless MSVC, libclang
and CMake are on `PATH`. (Watch the quoting: `set VAR=value && ...` in cmd captures the
trailing space into the value — `build.bat` uses the quoted `set "VAR=value"` form.)

Installer rebuilt and verified: **4.82 MB**, 2026-08-05 16:18.

**Note on the reference:** Flow is the commercial competitor this product is positioned
against. Matching window geometry and layout structure is fair; copying its visual identity
is not, and is also against `docs/DESIGN.md`'s deliberate "instrument" direction. See the
open question at the end of §6.

### Round 6 (2026-08-05) — the window wouldn't open, and injection was corrupting text

Two product-breaking bugs from the owner's first real dictation session.

**1. Clicking the app icon did nothing.** There was no single-instance guard, so launching
from the Start Menu while the app was already in the tray started a *second* process. That
process auto-loaded the model, concluded it was already set up, and — under the old
"only show the window if no model is loaded" rule — never showed a window. Two fixes:

- `tauri-plugin-single-instance` now surfaces the running window on a second launch.
- **The app always shows its window on launch.** Withholding it because a model happened to
  be loaded meant deliberately opening the app produced no window and no feedback. The tray
  is what keeps it alive after you close it; that is not a reason to refuse to open.
- `show_main_window` now logs its failures instead of discarding every `Result` with `let _ =`.
  "The window didn't open" was an unexplainable mystery precisely because of that.

Verified: with a model loaded — the exact case that used to fail — `EnumWindows` reports
`VISIBLE=True 1573x1025 at (173,7)`.

**2. Injected text was mangled.** Real captured output, against a two-minute dictation:

```text
7.cccchose my uuuuuu uuurself,MMMMMM…Mumbai.………………………………
```

The fragments appear *in the right order*, so Whisper was fine — the delivery was destroying
it. `inject_text` built one `SendInput` call carrying ~3,000 key events. Windows delivers
those asynchronously into the target's input queue; it overflows, KEYUPs get dropped, and the
key auto-repeats. Hence `cccc`, `MMMM`, and a tail of thousands of dots.

`injection.rs` was rewritten with two strategies:

- **Paste** (clipboard + Ctrl+V) for anything over 120 characters — instant regardless of
  length, and impossible to corrupt. The previous clipboard contents are restored afterwards.
- **Typing** (`SendInput`) for short text only, now in 40-event batches with a 1 ms gap.

Clipboard access also moved off `powershell -Command Set-Clipboard` — which cost hundreds of
milliseconds per dictation and broke on quotes and newlines — onto the Win32 API directly.
That is most of the "it takes forever" complaint: the old path typed 1,500 characters one
keystroke at a time *and* spawned PowerShell.

Two tests cover it: a clipboard round-trip over quotes, newlines and unicode, and a guard on
the paste/type threshold so a future edit can't route transcripts back onto the typing path.

**Repo hygiene, before the first push to GitHub:**

- **Deleted `SETUP_GUIDE.md`.** It was a survivor of the fake first version: it instructed
  users to get an **OpenRouter API key** for a cleanup feature that no longer exists, claimed
  an "encrypted database" (it is plain SQLite), and suggested backing up to "cloud storage".
  Publishing that would have been the exact "never claim more than the code does" failure
  this project keeps warning about.
- **Replaced `.github/workflows/ci-cd.yml`.** It could not have run: `path: *.msi` is a YAML
  alias and fails to parse, it triggered on `main`/`develop` (the branch is `master`),
  referenced the `msi` bundle target removed in bug #8, and used the retired
  `upload-artifact@v3`. The new `ci.yml` is frontend-only and its steps are verified to pass
  locally. No fake green ticks.
- README corrected: hotkey was still documented as `Ctrl+Space`, install still said
  `npm run dev`, and the model table listed `small.en`/`medium` at roughly half their real
  sizes — the same wrong-sizes bug Round 2 fixed in the UI but never here.
- `.gitignore` now covers `.claude/`, `*.local.json`, `*.log`, and `.window-state.json`.

**Known, not fixed:** `cargo check --no-default-features` fails — `asr.rs` imports
`whisper_rs` unconditionally, so the `whisper` feature flag gates nothing. It is decorative,
which is the same shape as the original fake build. Worth fixing; it is also what stops CI
from building the Rust side without a 10-minute native toolchain setup.

### Round 3 (2026-08-05) — models, UI, and the status bug

- **Upgraded whisper-rs 0.7 → 0.16.** The old crate bundled a 2023 whisper.cpp with no
  `large-v3-turbo`. Turbo is a distilled 4-layer decoder: multilingual, accuracy near
  `large-v3`, but a fraction of the runtime — it genuinely breaks the
  accuracy-versus-speed tradeoff rather than sitting on it.
- **Curated the model list** to `tiny.en`, `base.en`, `large-v3-turbo-q5_0` (recommended),
  `large-v3-turbo`, `large-v3`. **`small.en` and `medium` were removed on purpose**:
  turbo-q5_0 is smaller, faster *and* more accurate than `small.en`, so keeping the old
  tiers would only invite users to pick a strictly worse option. The owner independently
  reported `small.en` as "not working" — it worked, it was just slow for its quality.
- **Thread count now matches the machine.** whisper.cpp defaults to 4 threads regardless of
  hardware; using the real core count is a large, free CPU speedup.
- `use_gpu(true)` is requested. It is a no-op unless a GPU backend feature (`vulkan`,
  `cuda`) is compiled in — enabling one is a follow-up, and the single biggest remaining
  latency lever.
- **Full UI redesign** — sidebar app shell (Dictate / History / Words / Snippets /
  Insights / Settings) replacing the two-tab layout. Design direction is "instrument":
  cool graphite panels, hairline rules, one signal-cyan accent used *only* to mean live,
  monospace for technical readouts. Signature element is the **signal meter**, which is
  flat when idle, ticks while listening, and sweeps while transcribing.
- **Dictionary, Snippets, History and Insights now have real UIs.** The backend CRUD and
  the `transcripts` table already existed and were simply never surfaced.
- **Fixed the "still shows Setup Required" bug.** Status arrived only via `status-changed`
  events; a single missed event stranded the UI claiming no model was loaded while the
  backend had one. Status is now re-read on every view change, so that state is
  unrecoverable-by-design no more.
- Added `audio_ms` to transcripts (migration `004`) so words-per-minute reflects real
  speaking time rather than processing time.

### Round 4 (2026-08-05) — the desync, properly fixed

The "still shows Setup Required" bug survived two attempted fixes. Root cause and remedy:

- **`Status` now carries `loaded_model`.** The UI was deriving "is this model active?" from
  the *saved setting* plus a boolean. Those diverge the moment a load fails or is in
  flight, so every model could render as inactive while one was loaded. The backend now
  reports which model is genuinely in memory, and the UI trusts only that.
- **Status is polled every 1.5s**, not just pushed via events. A dropped `status-changed`
  previously left the UI permanently wrong with no user-recoverable path. `get_status` is
  an in-memory read, so this is cheap and makes the desync self-healing. Events remain the
  fast path.
- **Download progress no longer jitters.** Progress was emitted once per network chunk —
  thousands of IPC messages a second, which made the bar shake rather than advance. Now
  throttled to visible movement (0.5%).
- The active model is shown on the Dictate screen and in the sidebar rail, so "which model
  am I using?" is always answerable.

**Measured after upgrading to whisper-rs 0.16** (same 6.59s clip, same transcript):
1.98s (0.7 debug) → 1.68s (0.7 release) → **1.19s (0.16 debug)**. Roughly 5.5x realtime
before release optimisation, from the newer whisper.cpp plus using the machine's real core
count.

### Round 4 UI work

- Window default was **480x720** — far too small for a six-section app. Now **1080x720**,
  minimum 860x560.
- Custom scrollbars: slim, rounded, inset. The default Windows bar made an otherwise quiet
  interface look unfinished.
- **Collapsible sidebar** (212px ↔ 60px) with icon-only mode and tooltips.

---

## 1. What this is

A free, open-source, **local-first** voice dictation app. Press a hotkey, speak, and clean
punctuated text appears at your cursor in any application.

It exists because the market has a gap:

| | Cloud? | Free? | Cleanup built in? |
|---|---|---|---|
| **Wispr Flow** (commercial leader) | Yes | No — subscription | Yes |
| **Handy** (best OSS alternative) | No | Yes | **No — raw transcript** |
| **AuraScribe** | **No** | **Yes, forever** | **Yes, on by default** |

Nobody had shipped free + open source + local + a cleanup layer that's on by default and
fast. That combination is the entire product.

### Non-negotiable principles

1. **Local-first.** Audio and text never leave the machine. The *only* network call in the
   entire app is a one-time Whisper model download.
2. **Free forever.** No tiers, no word caps, no account. Donations never gate a feature.
3. **Lightweight.** 4.6 MB installer, ~40 MB idle RAM (~180 MB with a model loaded).
4. **Honest security posture.** No telemetry, no analytics. Claims must be checkable in
   source — never claim more than the code does.
5. **Cross-platform intent.** Windows works today; macOS/Linux are stubbed honestly.

### Explicit non-goals (v1)

No meeting transcription, no mobile app, no cloud option even opt-in, no team features, no
LLM "agent command" mode, no wake words, no paid tiers. These are v2+ conversations *after*
daily use is proven.

---

## 2. Current state — what actually works

Verified by running it, not by reading code:

| Capability | Status | Evidence |
|---|---|---|
| Local Whisper transcription | ✅ Working | `"The quick brown fox jumps over the lazy dog…"` transcribed exactly |
| Transcription speed | ✅ 1.68s for 6.59s audio (~3.9× realtime, release build) | integration test |
| Local cleanup pass | ✅ Working | 12 unit tests, incl. real captured output |
| Global hotkey (toggle + push-to-talk) | ✅ Registered | 4 real recording sessions in logs |
| Text injection at cursor (Windows) | ✅ `SendInput` w/ clipboard fallback | manual |
| Tray icon w/ 3 states | ✅ Built | manual |
| Recording overlay | ✅ Built | dev path fixed + guarded, Round 5 |
| Model download + auto-load on startup | ✅ Working | logs |
| Settings persistence | ✅ SQLite | verified across restarts |
| Dictionary / snippets / history | ✅ Real CRUD **and UI** | schema + commands + views |
| Usage insights (words, wpm, streak) | ✅ Computed from local history | `db::stats` |
| Production installer | ✅ 4.58 MB NSIS | `AuraScribe_1.0.0_x64-setup.exe` |
| macOS / Linux | ❌ Not implemented | returns explicit errors, never fake success |

### Measured numbers

- Installer: **4.58 MB** · release binary: **13.08 MB**
- RAM: **~40 MB** idle, **~180 MB** with `base.en` loaded (the model is ~140 MB of that)
- Transcription: **~3.9× realtime** (a 3-second phrase ≈ under a second)
- Cleanup: pure string ops, negligible next to transcription
- Full release build: **~4–6 min** (whisper.cpp compiles from source)

---

## 3. History — why the code looks like it does

The project had a **first attempt that never worked**, then a full rebuild.

### What the first attempt actually was

A polished UI shell over a backend that was mostly stubs:

- Whisper code **had never once compiled** — it called `WhisperContextParameters` and
  `new_with_params`, which don't exist in whisper-rs 0.7. The feature flag was off.
- The only working pipeline **uploaded audio to OpenRouter's cloud** and required an API
  key — the exact opposite of the stated product.
- Text injection only ran `Set-Clipboard`; it never pasted, so text never reached the cursor.
- Dictionary, snippets, history, permissions, model management were **hardcoded fakes**
  (`get_dictionary` → `[]`, `add_dictionary_entry` → always `1`).
- **No `capabilities/` directory existed at all**, so Tauri v2's ACL denied every core and
  plugin IPC call, including the `listen()` the UI depends on.
- The UI showed an "AES-256 encrypted" privacy card while storing the API key **in plaintext**.
- The README credited "Silero VAD" that was never implemented.

### The rebuild (2026-08-04 / 08-05)

Removed the entire cloud path (`llm.rs`, `ollama.ts`, `crypto.rs`, OpenRouter settings),
deleted dead modules (`vad.rs`, `models.rs`, `events.rs`, `db.ts`), and built the real
runtime: hotkey registration, local Whisper, local cleanup, real `SendInput` injection,
tray-first window model, overlay, and genuine DB-backed CRUD.

**Net −1,126 lines while adding functionality.**

### Bugs found only by actually running it

These are worth remembering — several were invisible to code review:

1. **CMake 4.x rejects whisper.cpp's `cmake_minimum_required < 3.5`.** Pinned via committed
   `src-tauri/.cargo/config.toml` so clones build without per-machine env setup.
2. **Migrations silently no-opped.** `CREATE TABLE IF NOT EXISTS settings` did nothing
   against a legacy table from an older architecture → app crashed on launch. Fixed with
   `002_settings_rebuild.sql`.
3. **Stale schema across a migration.** Pool connections opened *before* a schema-changing
   migration held an old schema → "no column found for name: hotkey". Migrations now run on
   a dedicated connection that is closed first.
4. **Models were written to Roaming AppData** — GB-sized files would sync on roaming
   profiles. Moved to Local.
5. **`load_model` returned `Ok(())` even when loading failed**, so clicking "Load" did
   nothing with no error shown. This is why models appeared un-loadable.
6. **App was invisible on first run** (`visible: false` + `skipTaskbar: true`) — no window,
   no taskbar entry, tray icon buried in Windows' overflow. Looked like a failed launch.
7. `normalize_punctuation` had an unreachable branch, so `"the store ."` kept its space.
8. Tauri's `msi` bundle target downloads the WiX toolset mid-build and **hung**. Removed;
   NSIS is the standard Tauri Windows installer.
9. **`cargo test --release` silently breaks the app.** Plain cargo rebuilds
   `target/release/aurascribe.exe` *without* embedding frontend assets, so the binary falls
   back to the dev-server URL. Running it then shows "localhost refused to connect" in every
   window — most visibly in the always-on-top overlay, which looks like a stuck error box
   the user can't dismiss. Always rebuild with `npm run build` afterwards.
10. **Dev server and static export disagree on page filenames.** `next dev` serves
    `/overlay/` and 404s on `/overlay/index.html`; the export only contains
    `overlay/index.html`. A hardcoded path is right in exactly one of the two. Any *new*
    secondary window must pick its path with `tauri::is_dev()` — see `overlay.rs`.
11. **`Ctrl+Space` is a bad default hotkey.** Windows IME claims it for input-language
    switching, so registration can silently do nothing. Default is now `Ctrl+Shift+Space`
    (what the PRD originally suggested), with migration `003` moving anyone still on the
    old default.

### Round 2 fixes (2026-08-05, after first real user test)

The owner's first hands-on test surfaced UX failures that testing-by-developer had missed:

- **"I can't see it as an open tab"** — the app was `visible: false` + `skipTaskbar: true`,
  so on first run there was no window, no taskbar entry, and the tray icon was buried in
  Windows' overflow chevron. It looked like the app hadn't launched. Now the settings window
  opens automatically when no model is loaded, and a taskbar entry appears when visible.
- **"I downloaded the model but it still says setup required"** — caused by bug #5
  (`load_model` returning `Ok` on failure). Download and load are now a single
  **"Download & Use"** action, and failures appear in red in the Settings panel.
- **"I don't understand what a Whisper model is"** — the UI now explains it in plain
  language and states that it downloads once and then runs offline free forever.
- Model sizes shown in the UI were roughly half the real download size (e.g. `base.en`
  listed as 74 MB, actually ~142 MB). Corrected.
- The overlay now refuses to display if its page failed to load, so a broken build can
  never again park an undismissable error box on screen.
- **Whisper's non-speech annotations were being typed into the user's document.** Real
  captured output included `[Music] [Music] [Music]`, `[typing sounds]`,
  `[indistinct chatter]` and `[BLANK_AUDIO]`. Whisper narrates silence and background
  noise this way; cleanup passed it straight through. Now stripped, and a recording that
  contains nothing but annotations injects nothing at all rather than typing junk at the
  cursor. Parenthesised spans are only removed when they look like audio descriptions, so
  a genuinely dictated aside like "(roughly ten)" survives.

**Quality note from that same session:** actual speech transcribed excellently —
*"Hi, hi, hello. This is a test run that I'm testing. So I hope this works perfectly."*
The engine was never the problem; presentation and edge cases were.

---

## 4. How to run it

### Prerequisites (Windows)

```bash
winget install LLVM.LLVM
```

```bash
winget install Kitware.CMake
```

Plus **Visual Studio Build Tools** with the C++ workload, Node 18+, and Rust stable.
Set `LIBCLANG_PATH` to `C:\Program Files\LLVM\bin`.

### Development

```bash
dev.bat
```

`dev.bat` sets up MSVC, libclang, and CMake, then runs `npx tauri dev`. First build takes
several minutes (whisper.cpp compiles from source); later builds are fast.

**Do not use `npm run dev`.** It runs `tauri dev` without that environment, so whisper-rs
fails to compile. `dev.bat` is the wrapper that makes the same command work.

### Production build

```bash
build.bat
```

Wrapper around `npm run build` that sets up the same toolchain `dev.bat` does. A bare
`npm run build` fails with `Unable to find libclang`.

Produces `src-tauri/target/release/bundle/nsis/AuraScribe_1.0.0_x64-setup.exe`.

### Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

The real transcription test is `#[ignore]` by default (needs a model + sample audio):

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test transcription -- --ignored --nocapture
```

Set `AURASCRIBE_TEST_WAV` to a 16 kHz mono WAV first.

---

## 5. How to use it (and what to tell users)

**"Whisper model" means:** the speech-recognition file that turns your voice into text.
It's downloaded **once** (~574 MB for the recommended `large-v3-turbo-q5_0`, or ~142 MB
for the lighter `base.en`), then everything runs
offline on your own machine, free, forever. Bigger models are more accurate but slower.

### First run

1. The settings window **opens automatically** when no model is installed.
2. Settings → Whisper Model → **Download & Use** on `large-v3-turbo-q5_0` (recommended),
   or `base.en` if you want a smaller download.
3. It downloads once and loads immediately — Home should then say **Ready**.
4. Close the window; the app keeps running in the system tray.

### Daily use

1. Click into any text field — Notepad, Chrome, VS Code, Slack, Word, anything.
2. Press **Ctrl+Space** (default), speak, press again to stop (toggle mode).
3. Cleaned text is typed at your cursor.

### Testing across applications

It works in any app that accepts keyboard input, because it synthesizes real keystrokes
via `SendInput`. Good things to try: Notepad, a browser address bar or textarea, VS Code,
Word, a chat app.

**Known limitation:** apps running *elevated* (as Administrator) reject synthetic input
from a non-elevated process. AuraScribe detects this, copies the text to your clipboard
instead, and tells you — paste with Ctrl+V. Run AuraScribe as admin if you need this.

### Where things live

- Database + settings: `%LOCALAPPDATA%\AuraScribe\aurascribe.db`
- Models: `%LOCALAPPDATA%\AuraScribe\models\`
- To fully reset: quit the app and delete that folder.

---

## 6. Roadmap

### Immediate next steps

- [ ] **Interactive verification by the owner** — hotkey in both modes, injection across
      several apps, tray state colors, overlay visibility. See `docs/TESTING.md`.
      **Overlay specifically:** run `dev.bat`, dictate, and confirm a "Listening…" pill
      appears bottom-centre — not a 404, and not nothing. Round 5 fixed the path and added
      a guard, but neither has been confirmed on screen by a human.
- [ ] Commit the rebuild (currently uncommitted — the whole rebuild plus docs).
- [ ] Daily-use trial: one week without switching back to another tool (the PRD's real
      definition of "v1 done").
- [ ] Consider installing via the NSIS installer rather than running the dev binary — it
      gives a Start Menu entry and avoids the dev-server class of problem entirely.

### Phase 2 (only after daily use is proven)

- [ ] **Personal dictionary** — DB table and CRUD commands already exist; needs to be
      applied to transcripts in `cleanup.rs` and exposed in the UI.
- [ ] **Per-app formatting rules** — `app_profiles` table exists; needs foreground-window
      detection (Windows `GetForegroundWindow`) and profile matching.
- [ ] **Dictation history UI** — `transcripts` table is already populated; needs a view and
      a "copy last" shortcut.
- [ ] Multilingual support (Whisper's multilingual models already download).

### Upgrades worth doing

- [ ] **GPU acceleration (Vulkan/CUDA).** whisper-rs is now on 0.16 (done in Round 3), and
      `use_gpu(true)` is already requested — but it is a no-op until a backend feature is
      compiled in. This is the single biggest remaining latency lever.
- [ ] **macOS support** — needs `CGEvent`-based injection and Accessibility permission
      handling. Interfaces already exist and return explicit errors.
      **Do this first:** the bundle identifier is `dev.aurascribe.app`, and Tauri warns that
      ending in `.app` conflicts with the macOS application bundle extension. Change it
      before any macOS build (and before wide distribution — changing it later makes
      existing installs look like a different app). Harmless on Windows, so it was left
      alone rather than forcing another release rebuild.
- [ ] Streaming/partial transcription for perceived latency.
- [ ] Auto-stop on silence (a VAD existed but was unused and removed; recover from git if
      wanted).

### Ideas for a richer UI (asked about; deliberately deferred)

A dashboard with dictation stats (words dictated, time saved, accuracy trends) is
appealing, but check it against the PRD's own warning: **scope creep is the single biggest
risk to finishing.** The app is a background utility, not something users stare at. Suggested
order if pursued: history view first (data already exists) → dictionary management → then
stats. Keep the settings window a single scrollable pane; multi-tab settings at this scope
would be over-engineering.

---

## 7. Things to be careful about

- **Never add a cloud fallback.** It breaks the core promise and the entire reason the
  product exists. The restrictive CSP in `tauri.conf.json` is deliberate so regressions are
  obvious.
- **Never claim more than the code does.** The previous build's fake "AES-256 encrypted"
  card is exactly the failure mode to avoid.
- **Don't fake success on unimplemented platforms.** Return an explicit error.
- **Route all status changes through `commands::emit_status`** — it is the single place
  that updates the tray icon, the overlay, and the frontend. Bypassing it desyncs them.
- **Don't edit an already-applied migration.** sqlx records a checksum; changing 001 breaks
  every existing install. Add a new migration instead.
- **All IPC goes through `src/lib/ipc.ts`** so the command surface stays auditable.
- Commands must **return `Err` on failure**, not log-and-return-`Ok` — bug #5 above.
- **New secondary windows must pick their page path with `tauri::is_dev()`.** `next dev`
  serves `/route/`; the export contains `route/index.html`. A hardcoded path is correct in
  exactly one of the two — bug #10 above.
- **Don't re-add `tauri-plugin-window-state`.** It was removed in Round 5b because it
  overrode the configured window size *and* the declared minimum, making config changes
  look like no-ops. If per-window persistence is ever genuinely wanted, restore position
  only — never size.
- **Never show a window before its page confirms it loaded.** Always-on-top, undecorated
  windows turn any load failure into an error box the user cannot dismiss. The overlay's
  `overlay_ready` handshake is the pattern to copy.
