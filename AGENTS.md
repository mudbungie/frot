# AGENTS.md (frot)

This repo defers to `~/AGENTS.md` for general practices. Project-specific notes follow.

## What frot is

A stateless, single-binary tool that takes structural impressions of web pages. Read `VISION.md` before designing anything; the scope and non-goals are deliberate and load-bearing.

## Hard rules

- **Source files ≤ 300 lines.** Enforced by pre-commit hook. Docs and config are exempt. If a file is growing, split it; don't widen the budget.
- **Test coverage = 100%.** Enforced by pre-commit hook via `cargo llvm-cov`. If something can't be tested, redesign it so it can be. Untestable code is not built.
- **Statelessness is non-negotiable.** No globals, no module-level mutable state, no implicit sessions. Pass state explicitly through call signatures.
- **Output formats are machine-first.** New outputs must be parseable without heuristics. Pretty-printing is a separate concern.

## Workflow

Task tracking is `bl` (see `bl --skill`). One agent takes a task all the way through — there is no separate review step or reviewer. All edits happen in `bl claim`-created worktrees; never edit `main` directly. Standard flow: `bl claim` (prints the worktree) → work in the worktree → `cd` to repo root → `bl close -m "<message>"`.

`bl close` is the sole delivery and gate: it folds `main` in, runs the repo's `pre-commit` hook (100% coverage, ≤300-line files, clippy), then squashes the worktree diff to `main` and tears the worktree down — all in one move. A hook failure aborts the close and leaves the task claimed for the fix, so tests must pass in the worktree before closing.

## Build

```
make setup              # one-time: install cargo-llvm-cov
make precommit-install  # one-time: wire pre-commit hook
make test               # cargo test
make cov                # cargo llvm-cov, requires 100%
make lint               # cargo clippy with -D warnings
make build              # release build
```

## Design constraints driving everything

- Binary size matters. Prefer small, focused crates. Audit dependency trees.
- Startup time matters. Avoid lazy_static / global init where a function-local pattern suffices.
- Single static binary as the deliverable. No dynamic library deps beyond libc / standard system libs.

## When to escalate to a discussion

- Any change that touches the public CLI surface or output schema.
- Any new dependency, especially one pulling in C code or large transitive trees.
- Anything that would soften "no JS execution" before Phase 4.
- Anything that introduces hidden state.
