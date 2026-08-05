# Maintaining these docs

> **For AI assistants (and humans) working on AuraScribe.**
> These docs are the memory of this project across sessions. Chat context windows fill up
> and get discarded; this folder does not. If it goes stale, the next session starts blind
> and repeats work that was already done — or worse, re-introduces a bug that was already
> fixed.

## The rule

**At the end of every task, update `docs/HANDOFF.md` before you finish your turn.**

A "task" means any unit of work the user asked for that changed the project: a feature, a
bug fix, a refactor, a dependency upgrade, a build change, or a decision about direction.
Reading code or answering a question is not a task and needs no update.

You do not need to ask permission to update these docs. It is part of finishing the work,
like running tests.

## What to update, and where

| Change | Update |
|---|---|
| A feature now works / stopped working | §2 "Current state" table in `HANDOFF.md` |
| You fixed a non-obvious bug | §3 "Bugs found only by actually running it" |
| You measured something (speed, size, RAM) | §2 "Measured numbers" |
| You finished or added a roadmap item | §6 "Roadmap" — tick it or add it |
| You learned a constraint the hard way | §7 "Things to be careful about" |
| Architecture / data flow changed | `docs/ARCHITECTURE.md` |
| Setup or build steps changed | §4 "How to run it" **and** `DEVELOPMENT.md` |
| User-facing behavior changed | §5 "How to use it" **and** `README.md` |

Always update the **Last updated** date at the top of `HANDOFF.md`.

## How to write it

- **Write what is true, not what is aspirational.** If something is half-done, say so and
  say what's missing. A doc that overstates progress is worse than no doc — the previous
  version of this project shipped a UI claiming "AES-256 encrypted" over a plaintext API
  key, and that lie cost real debugging time later.
- **Record evidence, not claims.** "Transcribes 6.59s of audio in 1.68s (integration test)"
  beats "fast transcription".
- **Keep the bug list.** Bugs that were only findable by running the app are the highest
  value thing in these docs. Future sessions cannot rediscover them from source alone.
- **Prune as you go.** Delete finished roadmap items rather than accumulating checkmarks.
  Keep `HANDOFF.md` scannable in a couple of minutes.
- **Don't duplicate the code.** Don't paste function bodies or list every file. Describe
  intent, decisions, and state — things the code cannot tell you.

## Starting a fresh session

Read in this order:

1. `docs/HANDOFF.md` — what this is, what works, what's next
2. `docs/ARCHITECTURE.md` — how it's put together
3. `CLAUDE.md` — working agreements for this repo

Then confirm reality before trusting the docs: run `cargo check` and look at
`git status`/`git log`. Docs describe the last known good state; the working tree may have
moved.

## A warning about verification

This project has a specific history: its first version *looked* complete and was almost
entirely non-functional. Whisper had never compiled once. Every CRUD command returned fake
data. Code review alone did not catch this — running it did.

So: **do not mark something "working" in `HANDOFF.md` because the code looks right.** Run
it. Ship an actual transcript, an actual timing, an actual screenshot. If you cannot verify
something (for example, interactive hotkey testing that needs a human at the keyboard), say
so explicitly in the docs rather than implying it was tested.
