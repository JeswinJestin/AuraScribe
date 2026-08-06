# AuraScribe v0.4.0 — the Moonshine speed engine

**The headline: AuraScribe now has a second, much faster speech engine — Moonshine — for
near-instant English dictation, plus a polished dark-glass interface.** Still 100% local,
still free, still private.

---

## ✨ What's new

### ⚡ New: the Moonshine engine (fast English dictation)
- Adds **Moonshine**, an on-device speech engine that is **~5× faster than Whisper** for
  English. On a normal laptop CPU it runs at roughly **0.1× real time — about ten times faster
  than you speak**, so text lands almost the moment you stop talking.
- Unlike Whisper's fixed 30-second processing window, Moonshine's compute **scales with how long
  you actually spoke**, which is why it feels instant for real dictation.
- Runs fully on-device via **sherpa-onnx / ONNX Runtime** using small int8 models. Two sizes:
  `moonshine-tiny-en` (110 MB) and `moonshine-base-en` (286 MB).

### 🔀 Two-engine architecture
- AuraScribe now runs **two engines** and switches between them automatically based on the model
  you pick: **Whisper** (robust, multilingual) or **Moonshine** (fast, English). The rest of the
  app doesn't change — pick a model and go.

### 🎨 Interface: dark-glass, done right
- New **Glass appearance** reworked to a macOS-style *dark vibrancy* look that reads cleanly over
  a dark backdrop image.
- **Fixed the controls to match the theme:** toggles are now clearly visible (they were an
  invisible white-on-white / harsh black before), native dropdown menus are themed to the glass
  instead of OS-black, and selection highlights (active nav, the Tap/Hold picker) use an indigo
  glass frost instead of a muddy brown block.

### 🧹 Smarter model list
- Removed heavy Whisper models that were **far too slow on a CPU** (minutes per sentence). The
  guiding rule now: every model must be **light + fast + accurate** at once. The path to better
  accuracy is a faster engine (Moonshine), not a heavier model.

---

## 🆚 How this is better than v0.3.0 (“warm-glass”)

| | v0.3.0 | **v0.4.0** |
|---|---|---|
| Speech engines | Whisper only | **Whisper + Moonshine** |
| English speed | Whisper (~0.5× real time) | **Moonshine (~0.1× — ~10× faster than you speak)** |
| Appearance | Warm light glass | **Dark-vibrancy glass** |
| Controls (toggles/menus/selection) | Off-theme, some invisible | **Themed and visible everywhere** |
| Model list | Included heavy, slow models | **Light/fast models only** |

In short: **v0.3.0 could transcribe; v0.4.0 transcribes fast, looks consistent, and is the
foundation for fast multilingual next.**

---

## 🔧 Under the hood

- Solved a tricky **Windows C-runtime conflict** that made the new engine crash — the Moonshine
  library ships with a static C-runtime while the rest of the app uses the dynamic one; the
  release build reconciles this and runs cleanly. (Diagnosed by inspecting the compiled DLLs and
  verified by running.)
- The installer now **bundles the ONNX Runtime and sherpa-onnx DLLs** next to the app so
  Moonshine works out of the box after install.

## 🔒 Privacy — unchanged and non-negotiable
- 100% local. No cloud, no account, no telemetry. The only network request in the entire app is
  the one-time model download.

---

## 📥 Install (Windows)
1. Download **`AuraScribe_0.4.0_x64-setup.exe`** below.
2. Windows SmartScreen may warn because the app isn't code-signed — click **More info →
   Run anyway**.
3. Launch AuraScribe, open **Settings → Voice model**, and **download a Moonshine model**
   (`moonshine-base-en` recommended) to try the fast engine. Press **Ctrl+Shift+Space** anywhere
   to dictate.
