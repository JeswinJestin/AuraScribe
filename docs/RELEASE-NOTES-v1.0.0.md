# AuraScribe v1.0.0 — the first stable release

**Your voice, everywhere. 100% local, free forever.**

Press a hotkey, speak, and clean text appears at your cursor in any app — a browser, an editor, a
terminal, a chat box. Nothing leaves your machine: no cloud, no account, no telemetry. This is the
first release we're calling **stable** — the engines, the languages, and the everyday flow are all
proven in daily use, and it's the first build verified to install and run on a **second, clean
Windows PC**.

---

## 🛠️ The fix that makes this the one to install

**Earlier builds (v0.4.x and before) could fail to start on a fresh Windows PC** with:

> *"The code execution cannot proceed because VCRUNTIME140_1.dll was not found."*

That wasn't a corrupt download — those builds silently depended on the **Microsoft Visual C++
Redistributable** being already installed (it is, on a developer's machine; it often isn't on a
stock Windows install). **v1.0.0 fixes this:** the required runtime is now bundled with the app, so
it launches on any Windows 10/11 machine with **nothing extra to install and no internet needed to
start**.

Those earlier versions were **previews, not stable releases** — use v1.0.0.

---

## What AuraScribe does

1. Press **Ctrl + Shift + Space**.
2. Speak.
3. Release (or press again) — your words are cleaned up and typed wherever your cursor is.

It works system-wide, even when the AuraScribe window is closed.

---

## ✨ Highlights in 1.0.0

### 🎙️ Five voice engines, one simple picker
Pick a model in Settings and it downloads once, then runs offline forever:

| Model | Best for | Languages |
|-------|----------|-----------|
| **AuraScribe English** ⭐ *Recommended* | Everyday English — the most accurate real-time choice | English |
| **AuraScribe English Mini** | The lightest install (110 MB) | English |
| **AuraScribe European** | Auto-detected European dictation | 25 European languages |
| **AuraScribe Asian** | Auto-detected Asian dictation | ~40 Asian languages (Hindi, Tamil, Telugu, …) |
| **AuraScribe Malayalam / Kannada** | Accurate Indic dictation the fast engines don't cover | Malayalam · Kannada |

**AuraScribe English (Moonshine) is now the recommended default** — the most accurate model that
still runs faster than you speak, instead of the fractionally-faster-but-less-accurate Mini.

### 🧹 Cleanup that actually cleans
"Tidy up my dictation" fixes punctuation and capitalisation and drops background-noise artefacts;
"Remove filler words" strips the *um*s and *uh*s. Both run **on every model**, entirely on your
machine, with no delay.

### 🗂️ A real History
Everything you dictate is kept **only on this device**, and now it's easy to browse:
- **Grouped by day** — Today, Yesterday, then dated headings.
- **Show more** to page back through older entries.
- A **usage heatmap** — a GitHub-style grid of your last six months, so you can see your active days
  at a glance.
- **Delete a date range** — clear a specific span, not just everything.

### 🎨 Polished, consistent interface
Every dropdown now matches the app's design — rounded, themed, and fully keyboard-navigable in
light, dark, and glass — instead of the plain system popup.

### 📝 Words & Snippets
Your personal dictionary and canned phrases apply to every dictation, on every model.

---

## 🔒 Your voice stays here

- Audio is transcribed **on this device** and never uploaded.
- Cleanup is plain local text processing, not a cloud service.
- The **only** network request AuraScribe ever makes is downloading a model you chose.
- No telemetry, no analytics, no account — by design.

---

## 💻 Install (Windows)

Download the installer below, run it, and launch AuraScribe. On first run a short walkthrough sets
you up; pick **AuraScribe English** when prompted and you're dictating in under a minute.

> **macOS and Linux:** not in this release. AuraScribe 1.0.0 is Windows-only; cross-platform support
> is planned and tracked separately.

---

*Free, open-source, local-first. If AuraScribe saves you time, tell a friend.*
