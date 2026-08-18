# Release checklist — v1.3.0

The end-to-end steps to cut v1.3.0 across Windows / macOS / Linux. Boxes already ticked were done
this session; the rest are **owner actions** (this environment can't push, and macOS/Linux only build
on the cloud runners).

## 1. Pre-flight (done)
- [x] Version bumped to **1.3.0** in `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `package.json`.
- [x] `npm run typecheck` clean · `cargo test --features moonshine` **71/71**.
- [x] Release notes written → `docs/RELEASE-NOTES-v1.3.0.md` (use as the GitHub release body).
- [x] Cross-platform CI workflow in place → `.github/workflows/release.yml`.

## 2. Build the Windows installer (proven path)
- [ ] Run **`moonshine-build.bat`** → `src-tauri/target/release/bundle/nsis/AuraScribe_1.3.0_x64-setup.exe`.
  - Confirm it bundles `onnxruntime.dll`, `sherpa-onnx-*.dll`, and the 3 MSVC runtime DLLs (extract with
    7-Zip if unsure). Never ship a plain `cargo build --release` — it omits the frontend + DLLs.

## 3. Verify on YOUR Windows machine BEFORE publishing (PROCESS RULE: exactly one install)
Install elevated (replacing `C:\Program Files\AuraScribe`), then check:
- [ ] **Onboarding** appears on a fresh profile: the animation plays, the **key-click / mic-on / voice /
      mic-off** sounds fire, the voice line plays once, the text types in, and **Skip** works on steps 1–2.
- [ ] The onboarding shows the **correct hotkey for this OS** (Windows: `Ctrl+Shift+Space`).
- [ ] Pressing the hotkey starts/stops dictation; text lands at the cursor in another app.
- [ ] **Settings → Hotkey → "Enable the dictation hotkey"** off ⇒ the hotkey stops working; on ⇒ works again.
- [ ] **Replay walkthrough** button (Settings → Hotkey) re-opens the tour.
- [ ] **Quiet speech**: dictate softly — it should transcribe noticeably better than before.
- [ ] **Window size** looks right (and ask your friend to confirm on the laptop that was wrong before).
- [ ] Streak / Insights / Recap still render.

## 4. Commit + push + tag
- [ ] Commit the app changes (this repo) and the landing favicon change (`../aurascribe-landing`) **separately**.
- [ ] `git push origin master`
- [ ] `git tag v1.3.0 && git push origin v1.3.0`  ← this triggers the release workflow.

## 5. Let CI build macOS + Linux (the whole reason for the workflow)
- [ ] Watch the **Actions → Release** run. It builds Windows/macOS/Linux in parallel and attaches them
      to a **draft** release for `v1.3.0`.
- [ ] **Expect the macOS/Linux rows to possibly fail the first time** (their sherpa `.dylib`/`.so`
      bundling has never run). `fail-fast: false` keeps Windows green regardless. Send me any red
      macOS/Linux log and I'll iterate on the dylib bundling.

## 6. Prepare the draft release
- [ ] Paste `docs/RELEASE-NOTES-v1.3.0.md` as the release body.
- [ ] Confirm the **Windows** `.exe` is attached and installs on a clean PC.
- [ ] For **macOS/Linux** artifacts: keep the "⚠️ experimental preview — installs & launches but does
      not dictate yet" wording. Do **not** present them as supported.

## 7. Publish
- [ ] Publish the release (mark it **Latest**). Optionally publish **Windows-only** and hold the
      macOS/Linux assets until their platform code lands — your call.

## What to send your friends right now
- **Windows friends:** send `AuraScribe_1.3.0_x64-setup.exe` — it's the real thing and works.
  Tell them: SmartScreen → **More info → Run anyway** (it's unsigned), then download a model on first run.
- **macOS / Linux friends:** **hold off**, or be explicit that it's a **launch-only preview that can't
  dictate yet** (injection + hotkey aren't implemented off Windows). Otherwise they'll install it, speak,
  and nothing will happen — which reads as "broken" rather than "preview." Real Mac/Linux dictation is a
  future release.

## Deferred to a future release (tracked)
- macOS/Linux **text injection + global hotkey** (the deps are already in `Cargo.toml` for macOS).
- **Noisy-room noise suppression** (this release ships low-voice gain only).
