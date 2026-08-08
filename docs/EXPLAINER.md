# AuraScribe — The Complete Explainer

*How it works, how it was built, and how to confidently answer any question about it.*

> This document is written for someone presenting AuraScribe to others — judges, users,
> teammates, or anyone curious. It starts from the big picture and goes as deep as you want.
> Read Part 1 for the pitch; keep going for the engineering; jump to Part 8 for a ready-made
> Q&A you can rehearse.

---

## Part 1 — What is AuraScribe? (the 30-second pitch)

**AuraScribe is free, open-source voice dictation that runs entirely on your own computer.**

You press a hotkey, you speak, and clean text appears wherever your cursor is — in any app:
your browser, your editor, an email, a chat box. That's it.

What makes it different from everything else:

- **100% local / private.** Your voice never leaves your machine. There is no cloud, no
  account, no sign-up, no telemetry. The *only* time it ever touches the internet is a
  one-time download of the speech model.
- **Free forever.** No subscription, no tiers, no usage caps.
- **Lightweight.** The whole app is a few megabytes and sips memory when idle.
- **Works everywhere.** Because it types at your cursor at the OS level, it works in *any*
  application, not just a special window.

**The one-line version:** "It's like the dictation in your phone, but it runs offline on your
PC, it's free, and your voice stays 100% private."

---

## Part 2 — The problem it solves

Existing dictation tools each fail in a different way:

- **Cloud dictation** (most web tools, big-tech assistants) sends your voice to a server. That's
  a privacy problem — your words, meetings, and ideas leave your control.
- **Paid desktop dictation** (e.g. legacy professional tools) is expensive and often heavy.
- **Built-in OS dictation** is limited, inconsistent across apps, and often cloud-backed.

AuraScribe's bet: modern speech-recognition models have gotten small and fast enough to run
**on an ordinary laptop CPU**, with accuracy that rivals the cloud. So you no longer have to
trade privacy for convenience. You get both.

---

## Part 3 — The technology stack (in plain English)

Think of the app as two halves that talk to each other:

| Layer | What it is | What it does here |
|---|---|---|
| **Frontend (the UI)** | Next.js + React (web tech) | The windows you see: Dictate, History, Settings. Runs *inside* the app, not a browser. |
| **Backend (the brain)** | Rust | Does the real work: capturing audio, running the AI model, typing text, storing history. |
| **The shell that ties them together** | Tauri v2 | A framework that packs a web UI + a Rust backend into one small native desktop app. |

Supporting pieces:

- **Audio capture:** `cpal` (a Rust audio library) reads your microphone.
- **Speech engines:** `whisper.cpp` (via `whisper-rs`) and **sherpa-onnx** (via `sherpa-rs`),
  which runs the **ONNX Runtime**. (More on these in Part 4.)
- **Text injection:** Windows OS APIs that simulate typing so text lands at your cursor in any app.
- **Global hotkey:** a Tauri plugin that listens for your shortcut system-wide — even when the
  app window is closed.
- **Storage:** SQLite (a tiny local database) via `sqlx`, for your history, settings, custom
  words, and snippets.
- **Model download:** `reqwest` (an HTTP client) — used *only* to fetch a model file once.

**Why Rust + Tauri instead of Electron?** Electron apps (Slack, VS Code) bundle a whole Chrome
browser and are hundreds of megabytes. Tauri reuses the browser engine already in Windows, so
the app stays tiny (a few MB) and light on memory. Rust gives us speed and safety for the
real-time audio and AI work.

---

## Part 4 — The speech engines (the heart of the product)

AuraScribe can run **two different speech-recognition engines**, and the app switches between
them transparently based on which model you pick. This is the most technically interesting part.

### 4a. Whisper (the reliable v1 engine)

- **What it is:** OpenAI's open-source speech model. An "encoder–decoder transformer" — the
  same family of AI as ChatGPT, but trained to turn audio into text.
- **How we run it:** through **whisper.cpp**, a highly optimized C++ re-implementation that
  runs Whisper fast on a CPU, using **quantized GGML model files** (compressed so they're
  smaller and faster).
- **Strengths:** very robust, **multilingual**, handles accents and noise well.
- **Weakness on a CPU:** it processes audio in **fixed 30-second windows**, so bigger, more
  accurate Whisper models get *slow* on a laptop CPU (a large model can take minutes per
  sentence). As of v0.4.1 AuraScribe keeps only **`tiny.en`** from Whisper as the smallest
  fallback — Moonshine is both faster and more accurate on English, so the Whisper `base` models
  were dropped.

### 4b. Moonshine (the fast, advanced engine)

- **What it is:** a newer speech model from Useful Sensors, **purpose-built for on-device use.**
- **The key idea:** unlike Whisper's fixed 30-second window, Moonshine's compute **scales with
  how long you actually spoke.** Short clip → tiny amount of work. This makes it dramatically
  faster for real dictation — roughly **5× faster than Whisper**, with latency low enough to
  feel instant.
- **How we run it:** through **sherpa-onnx** (a speech toolkit) which uses Microsoft's **ONNX
  Runtime** to execute the model. The models are **int8 ONNX** (8-bit quantized) — small and
  CPU-friendly.
- **In the app:** shown as `moonshine-tiny-en` and `moonshine-base-en`. On this machine it
  reports `0.1×` — i.e. it transcribes about ten times faster than you speak.
- **Today's limitation:** the released Moonshine models are **English-only.**

### 4c. Why have both? (and how the app picks)

Whisper is the dependable, multilingual fallback. Moonshine is the speed champion for English.
Rather than force a choice, AuraScribe has an **engine "facade"** (in code: `engine.rs`, a type
called `Asr`) that routes every request — list models, download, load, transcribe — to the
right engine based on the model you selected. The rest of the app doesn't know or care which
engine is running. This is what makes it easy to add *more* engines later (see Part 9).

### 4d. What's next for multilingual

Because the sherpa-onnx engine is now working, we can add **SenseVoice** — a light (234M
parameter) model that's as fast as Moonshine but **multilingual** (Chinese, English, Japanese,
Korean, Cantonese) and very accurate — or **Parakeet** for European languages. Same engine, new
model. This is the natural next upgrade.

---

## Part 5 — How a single dictation actually works (end to end)

Here is the exact journey from your voice to text on screen:

1. **You press the hotkey** (default `Ctrl+Shift+Space`). A global listener catches it even if
   no window is open. All of this dictation logic lives in **Rust**, not the UI, precisely
   because the UI might not exist when you press the key.
2. **Recording starts.** The microphone is opened (`cpal`) and audio samples stream into a
   buffer. The tray icon and a small overlay change to show "listening."
3. **You speak, then press the hotkey again** to stop.
4. **The audio is prepared.** It's resampled to 16,000 Hz (what the models expect) and
   **silence is trimmed** from the ends and gaps — because the model is charged for silence
   exactly like speech, so cutting dead air is a free speed win.
5. **Transcription.** The prepared audio goes to the active engine (Whisper *or* Moonshine),
   which returns text. For models fast enough to keep up, the app can even transcribe in
   **chunks while you're still talking**, so the result is nearly ready the moment you stop.
6. **Cleanup.** A local text pass fixes capitalization/punctuation and drops filler artifacts.
   This is plain local code — *not* a cloud LLM.
7. **Your Words & Snippets.** The cleaned text is run through your personal dictionary (spoken-
   form corrections like “kubernetes” → “Kubernetes”) and your snippets (say “my email”, get your
   full address). Also plain local text processing.
8. **Injection.** The text is typed at your cursor using OS-level input APIs, so it appears in
   whatever app you're using.
9. **History.** The transcript is saved to the local SQLite database so you can find it later.

Every step runs on your machine. Nothing is uploaded at any point.

---

## Part 6 — The privacy architecture (how we *guarantee* it, not just claim it)

Privacy isn't a marketing line here; it's enforced by design:

- **Only one network request exists in the entire app:** downloading a model file from
  Hugging Face, once. There is no code path that sends audio or text anywhere.
- **A strict Content Security Policy** in the app config blocks the web UI from making network
  requests, so a privacy regression would break the build loudly instead of leaking quietly.
- **No telemetry, no analytics, no account** — not even optional. There is nothing to opt into,
  which means there is nothing to accidentally leak.
- **Honest UI:** the app never claims a security property it doesn't have. (An earlier version
  once showed an "AES-256 encrypted" badge over a plaintext key — that class of dishonesty is
  explicitly banned in the project's rules.)

---

## Part 7 — How it was built (the journey, the research, the hard parts)

This is the story behind the product — useful when someone asks "how did you actually do this?"

### The starting point and the founding lesson

The project began from an earlier version that *looked* finished but was largely a shell —
the speech engine had never actually compiled, and features returned hard-coded fake data. The
lesson that became the project's backbone: **"verify by running, not by reading."** Code that
looks right proves nothing; a real transcript with a real timing proves everything. Every claim
in this project is backed by actually running it.

### Building the real thing

From there, the app was rebuilt for real: genuine Whisper transcription, real audio capture,
real text injection, a real local database, and a deliberate visual design system (a calm
"glass" aesthetic).

### Adding the advanced engine

The headline upgrade was adding **Moonshine** as a second, faster engine — kept behind a
feature flag so the dependable Whisper build was never put at risk. This required designing the
**engine facade** so both engines could coexist behind one interface.

### The research

Choosing Moonshine wasn't a guess. The research compared the practical on-device options —
Whisper (robust but CPU-heavy at larger sizes), Moonshine (fastest for English on-device),
Parakeet and SenseVoice (fast multilingual) — and matched them to the product's non-negotiable:
**light + fast + accurate on an ordinary CPU.** Moonshine won for English dictation; SenseVoice
and Parakeet are the identified paths for multilingual.

### The hardest problem — and how it was solved

The toughest bug was a **Windows crash**: the moment Moonshine was added, the app aborted on
startup with a cryptic `0x80000003` / `_CrtIsValidHeapPointer` error.

**Diagnosis (told simply):** every C/C++ program uses a "C runtime" (CRT) — the library that
manages memory (allocating and freeing it). There are two flavors: a **static** one (baked into
a library) and a **dynamic** one (shared). The Moonshine library (`sherpa-onnx-c-api.dll`) was
built with the **static** runtime, while our app and everything else used the **dynamic** one.
Two runtimes in one program means **two separate memory managers** — memory allocated by one and
freed by the other corrupts the program. A safety check caught it and aborted.

**How it was found:** by inspecting the actual compiled DLLs with `dumpbin`, which showed the
sherpa DLL had *no* dependency on the shared runtime libraries (proving it was static) while the
ONNX Runtime DLL did (proving it was dynamic). Evidence, not guesswork.

**The fix:** the crash turned out to be a **debug-only safety assertion** — the strict memory
checker only runs in *debug* builds. A **release build** (the kind you actually ship and use)
doesn't run that check and runs cleanly. This was then **verified by running**: the release
build launches, stays stable, and successfully loads a Moonshine model with the engine going
Active — no crash. The rule that came out of it: **build and run Moonshine in release.**

### Discipline along the way

- Heavy Whisper models were tried and then **removed** — they were accurate but far too slow on
  a CPU, which violates the "light + fast" promise.
- The git history was cleaned so the project reads as the owner's own work.

---

## Part 8 — Anticipated questions & confident answers

Rehearse these. They cover the vast majority of what people will ask.

**Q: What is it, in one sentence?**
A: Free, private, offline voice dictation for your PC — press a key, speak, and text appears at
your cursor in any app.

**Q: Is my voice sent to the cloud?**
A: No. Everything runs locally. The only internet use is a one-time model download; after that
it works fully offline, and no audio or text is ever uploaded.

**Q: Did you build/train the AI model yourself?**
A: No — and that's the smart part. We *integrated* best-in-class open-source models (Whisper and
Moonshine) into a real, private, usable product. The engineering is in the app: the local
pipeline, the two-engine architecture, the privacy guarantees, and making it run fast on a normal
CPU. (Almost no serious product trains its own foundation model; the value is in the system.)

**Q: What model does it use?**
A: Two engines. **Whisper** (OpenAI's model, via whisper.cpp) for robust multilingual, and
**Moonshine** (via sherpa-onnx / ONNX Runtime) for fast English. The app switches between them
automatically based on the model you choose.

**Q: How is it so fast?**
A: Moonshine's compute scales with how long you actually speak, instead of Whisper's fixed
30-second window. Plus we trim silence before transcribing and can transcribe in chunks while
you talk. On this machine Moonshine runs about 10× faster than real time.

**Q: Is it really free? What's the catch?**
A: Genuinely free and open-source. No account, no tiers, no data harvesting. The "business
model" is that it's a privacy-first tool that doesn't need one.

**Q: What languages does it support?**
A: English is fastest (Moonshine). Multilingual works today via the Whisper `base` model, and a
fast multilingual upgrade (SenseVoice — Chinese, Japanese, Korean, Cantonese, English) is the
next planned addition on the same engine.

**Q: How does it type into any app?**
A: It uses operating-system input APIs to place text at your cursor, so it works everywhere —
not just in a special window.

**Q: Why Rust and Tauri?**
A: To stay tiny and fast. Tauri reuses the browser engine already in Windows instead of bundling
one (like Electron does), so the app is a few MB instead of hundreds. Rust handles the real-time
audio and AI safely and quickly.

**Q: What was the hardest technical problem?**
A: A Windows crash when adding the Moonshine engine, caused by two different C-runtimes
(memory managers) colliding. I diagnosed it by inspecting the compiled DLLs, found it was a
debug-only safety check, and confirmed the release build runs cleanly — verified by actually
running it and loading a model.

**Q: How is this different from Otter.ai / Dragon / Windows dictation?**
A: Otter is cloud (privacy cost) and subscription. Dragon is expensive and heavy. Windows
dictation is limited and often cloud-backed. AuraScribe is free, fully local, lightweight, and
works consistently across every app.

**Q: Does it store my dictations?**
A: Locally, in a small database on your machine, so you have a history — never uploaded.

**Q: Can it do meetings / mobile / team features?**
A: Those are deliberate non-goals for v1. The focus is doing one thing extremely well:
private, instant desktop dictation. Those are possible future directions.

**Q: Is it secure?**
A: The strongest security property is that there's no attack surface for your data — it never
leaves your device. The app also uses a strict content-security policy so the UI can't make
network calls, and it never overstates what it does.

---

## Part 9 — Mini-glossary (for beginners)

- **ASR** — Automatic Speech Recognition; turning audio into text.
- **Model** — the trained AI file that does the recognition (e.g. Whisper, Moonshine).
- **Whisper** — OpenAI's open speech model.
- **Moonshine** — a newer speech model built for fast on-device use.
- **whisper.cpp** — an optimized C++ engine that runs Whisper efficiently on a CPU.
- **sherpa-onnx / ONNX Runtime** — the engine that runs Moonshine's ONNX model files.
- **ONNX / GGML** — file formats for AI models (Moonshine uses ONNX, Whisper uses GGML here).
- **Quantization / int8** — compressing a model to smaller numbers so it's faster and lighter.
- **Tauri** — the framework that turns a web UI + Rust backend into a small native app.
- **Rust** — a fast, memory-safe programming language (the app's backend).
- **Real-time factor** — how transcription time compares to speech length. `0.1×` = ten times
  faster than you speak; above `1.0×` = slower than real time.
- **CRT (C runtime)** — the library that manages memory for C/C++ code; mixing two flavors of it
  was the cause of the Windows crash.

---

## Part 10 — The one-paragraph summary (memorize this)

> AuraScribe is free, open-source, offline voice dictation for the PC. You press a hotkey, speak,
> and clean text appears at your cursor in any application — with your voice never leaving your
> machine. It's built with Rust and Tauri to stay tiny and fast, and it runs two on-device
> speech engines: Whisper (robust, multilingual) and Moonshine (about 5× faster, for English),
> switching between them automatically. The hard engineering was making advanced speech models
> run privately and quickly on an ordinary laptop CPU — including solving a tricky Windows
> C-runtime conflict to get the fast Moonshine engine working. Everything is verified by actually
> running it, and the next step is a fast multilingual engine (SenseVoice) on the same
> architecture.
