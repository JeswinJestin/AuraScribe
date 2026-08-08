# AuraScribe v0.4.1 — Moonshine-first, and Words/Snippets that actually work

Builds on v0.4.0's Moonshine speed engine. This release makes Moonshine the default, cleans up
the model list, **fixes the personal dictionary and snippets so they actually change your
dictation**, adds a first-run walkthrough, and makes the Glass look the default. Still 100%
local, still free, still private.

---

## ✨ What's new

### 🖱️ Click the overlay to stop
The floating "Listening…" pill is now a **stop button** — click it with the mouse to end
dictation, no need to reach for the hotkey. It uses a non-activating window so the click never
steals focus from the app you're dictating into (otherwise the text would land in the overlay).

### 🌍 Fast multilingual: Parakeet + bring-your-own models
- New **Parakeet v3** engine (NVIDIA, via sherpa-onnx): **25 European languages** with automatic
  language detection, accuracy at/above Whisper large-v3, fast on CPU. The same public model Handy
  and OpenWhispr ship.
- New **bring-your-own-model** support: drop any sherpa-onnx transducer bundle into the models
  folder and it appears in the list automatically — 100% local, no cloud. This is the path to
  **Hindi / Malayalam and other Indian languages** via AI4Bharat's IndicConformer; see
  `docs/INDIC-CONFORMER.md` for the one-time export recipe.
- Honest note: no free model matches Moonshine's speed for non-English; a transducer like
  Parakeet/IndicConformer is faster-than-real-time on a good CPU, not Moonshine-instant. Cloud
  services (Soniox, etc.) are faster still but were rejected — they'd break the local-first promise.

### 🗣️ Words & Snippets now really apply to your dictation
Previously the **Words** (personal dictionary) and **Snippets** screens let you add entries, but
those entries were never applied to the text that got typed — they looked done but did nothing.
Now they run on every dictation, right after cleanup and before the text is inserted:

- **Words** — say “kubernetes”, get “Kubernetes”. Whole-word and (optionally) case-sensitive.
- **Snippets** — say “my email”, get your full address inserted. Triggers match as whole phrases,
  case-insensitively, longest trigger first. Expansions are inserted verbatim.

### ⚡ Moonshine is now the default; the model list is trimmed
- The default model is **`moonshine-base-en`** (fast **and** the most accurate English option).
- Removed the Whisper **`base.en`** and **`base` (multilingual)** models: `moonshine-tiny-en` is
  both faster and more accurate on English, so a Whisper `base` model was a strictly worse
  choice. **`tiny.en`** stays as the smallest possible fallback.
- The **Recommended** badge now points at the best model across *both* engines, not per-engine.

### 👋 First-run walkthrough
- New users get a **5-step onboarding**: what AuraScribe is, picking a voice model, the hotkey,
  Cleanup/Words/Snippets, and appearance. It shows once and can be skipped at any time. Returning
  users never see it and keep their settings.

### 🎨 Glass is the default appearance
- Fresh installs open in the **Glass** look. Existing installs keep whatever appearance they had.
- Light, Dark, and Match-system are still there in Settings → Appearance.

### 🧹 Cleanup verified
- The local cleanup pass (punctuation, capitalisation, filler-word removal, junk-annotation
  stripping) is covered by 13 unit tests, all passing. Filler removal (“um”, “uh”, “like”, “you
  know”, …) works and stays behind its own toggle.

---

## 🆚 Since v0.4.0

| | v0.4.0 | **v0.4.1** |
|---|---|---|
| Default model | `base.en` (Whisper) | **`moonshine-base-en`** |
| Model list | tiny.en, base.en, base, + Moonshine | **tiny.en + Moonshine only** |
| Words / Snippets | Stored but **never applied** | **Applied to every dictation** |
| First run | Straight into the app | **5-step walkthrough** |
| Default appearance | Light | **Glass** |

---

## 🔒 Privacy — unchanged and non-negotiable
100% local. No cloud, no account, no telemetry. Words, snippets, and cleanup are all plain
on-device text processing. The only network request in the entire app is the one-time model
download.

---

## 📥 Install (Windows)
1. Download **`AuraScribe_0.4.1_x64-setup.exe`** below.
2. Windows SmartScreen may warn because the app isn't code-signed — click **More info → Run
   anyway**.
3. On first launch, follow the short walkthrough, then open **Settings → Voice model** and
   download **`moonshine-base-en`**. Press **Ctrl+Shift+Space** anywhere to dictate.
