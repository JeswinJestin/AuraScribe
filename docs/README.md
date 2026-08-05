# AuraScribe documentation

Start here.

| Document | What it's for |
|---|---|
| **[HANDOFF.md](HANDOFF.md)** | **Read first.** What this project is, what actually works today, what's next, and the bugs that were only findable by running it. |
| [ARCHITECTURE.md](ARCHITECTURE.md) | How the app is put together and why — the dictation pipeline, state handling, database rules, build gotchas. |
| [DESIGN.md](DESIGN.md) | The visual system — palette, type, layout, the signature signal meter, and the writing rules. Read before changing UI. |
| [TESTING.md](TESTING.md) | How to actually run and test it, including across other applications. Written for the owner, not CI. |
| [MAINTAINING-DOCS.md](MAINTAINING-DOCS.md) | The rules for keeping these docs current. Every task must update `HANDOFF.md`. |

Also in the repo root:

- [`CLAUDE.md`](../CLAUDE.md) — working agreements every AI session should follow
- [`README.md`](../README.md) — the public-facing project README
- [`DEVELOPMENT.md`](../DEVELOPMENT.md) — toolchain setup and dev workflow

## Why this folder exists

Chat context windows fill up and get discarded. This folder is the project's memory across
sessions — it's what lets a fresh session pick up without re-deriving everything or
re-introducing bugs that were already fixed.

It is only useful if it stays honest. This project's first version *looked* finished and
was almost entirely non-functional, and its docs said otherwise. Record what is true and
what was actually verified, not what was intended.
