# AGENTS.md — working agreements for AuraScribe

## Read this first

Before doing anything in this repo, read:

1. **`docs/HANDOFF.md`** — what this project is, what actually works, what's next
2. **`docs/PROJECT-JOURNAL.md`** — the journey: every major experiment, decision, and dead end,
   so you don't re-run one. Read it to understand *how we got here*.
3. **`docs/ARCHITECTURE.md`** — how it's built and why
4. **`docs/DESIGN.md`** — the visual system; read before touching any UI
5. **`docs/MAINTAINING-DOCS.md`** — how to keep the above current

Then verify reality: run `cargo check --manifest-path src-tauri/Cargo.toml` and check
`git status`. The docs describe the last known good state; the tree may have moved.

## Update the docs after every task

**At the end of every task that changes the project, update `docs/HANDOFF.md` before
finishing your turn.** Bump its "Last updated" date. See `docs/MAINTAINING-DOCS.md` for
what goes where.

**And for any MAJOR change, also append a dated entry to `docs/PROJECT-JOURNAL.md`.** Major =
a feature shipped, a release cut, an experiment run (success *or* failure), an architecture or
product decision, a reverted change, or a significant bug root-caused. Record it honestly —
especially the dead ends and *why* they failed — so the path is never re-walked. HANDOFF is the
current state; the journal is the story of how we got there. Small/cosmetic changes don't need a
journal entry; use judgement. Do not ask permission — it is part of finishing the work.

This is not optional bookkeeping. Chat context gets discarded; this folder is the only
memory that survives into the next session. Do not ask permission — it is part of finishing
the work, like running tests.

## What this project is

Free, open-source, **local-first** voice dictation. Hotkey → speak → clean text at your
cursor, in any app.

### Non-negotiables

- **Never add a cloud call.** The only permitted network request in the entire app is the
  one-time Whisper model download. No cloud STT, no LLM cleanup, no telemetry, no analytics
  — not even opt-in. This is the reason the product exists. The restrictive CSP in
  `tauri.conf.json` is deliberate so regressions are obvious.
- **Free forever.** No tiers, no caps, no account.
- **Stay lightweight.** ~4.6 MB installer, ~40 MB idle. Weigh every dependency.
- **Never claim more than the code does.** The previous version displayed an "AES-256
  encrypted" privacy card over a plaintext API key. Do not do anything in that family.
- **Don't fake success on unimplemented platforms.** macOS/Linux paths return explicit
  errors. Keep it that way rather than silently doing nothing.

### Scope discipline

The PRD names scope creep as the single biggest risk to this ever shipping. Explicit
non-goals for v1: meeting transcription, mobile, cloud options, team features, LLM agent
mode, wake words, paid tiers. Push back on these; they are v2+ conversations *after* daily
use is proven.

## Verify by running, not by reading

This project's first version looked complete and was almost entirely fake — Whisper had
never compiled once, and every CRUD command returned hardcoded data. Code review did not
catch it. Running it did.

So:

- Don't mark something "working" because the code looks right. Produce evidence: a real
  transcript, a real timing, real output.
- If something can't be verified without a human (interactive hotkey/injection testing),
  say so explicitly rather than implying it was tested.
- Report failures plainly, with the output.

## Code conventions

- **All status changes go through `commands::emit_status`** — it is the single place that
  updates the tray icon, the overlay, and the frontend. Bypassing it desyncs them.
- **Commands must return `Err` on failure.** Logging an error and returning `Ok(())` makes
  the UI silently do nothing — this was a real bug (`load_model`).
- **All IPC goes through `src/lib/ipc.ts`.** Don't call `invoke` directly from components.
- **Never edit an already-applied migration** — sqlx checksums them. Add a new file.
- **Secondary windows pick their page path with `tauri::is_dev()`.** `next dev` serves
  `/route/` and 404s on `/route/index.html`; the static export contains only
  `route/index.html`. Hardcoding either one breaks the other host. Never show such a window
  until its page has confirmed it loaded — see `overlay.rs` and the `overlay_ready` command.
- Dictation logic belongs in Rust, not React. The hotkey fires when no window exists.
- **Don't rely on events alone for state the UI depends on.** A missed `status-changed`
  once stranded the app claiming no model was loaded. Re-read authoritative state on
  navigation; treat events as the fast path, not the only path.
- Follow `docs/DESIGN.md`: colour means state, monospace is for machine-reported values,
  and the signal meter is the only element allowed real motion.

## Build and test

```bash
dev.bat
```

```bash
build.bat
```

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Requires LLVM/libclang, CMake, and MSVC build tools — see `DEVELOPMENT.md`.

**Build releases with the Tauri CLI, never plain `cargo build --release`.** Plain cargo
doesn't embed frontend assets, yielding a binary that falls back to the dev-server URL and
shows a connection error. `cargo test --release` overwrites the release binary the same way
— rebuild with `npm run build` afterwards.

## Git

Commit only when asked. If asked, branch first rather than committing to `master`.
