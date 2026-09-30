# AGENTS.md (frot)

This repo defers to `~/AGENTS.md` for general practices. Project-specific notes follow.

## What frot is

A stateless, single-binary tool that takes structural impressions of web pages. Read `VISION.md` before designing anything; the scope and non-goals are deliberate and load-bearing.

## Hard rules

- **Source files ≤ 300 lines.** Enforced by `make size`, a step of the gate (below). Docs and config are exempt. If a file is growing, split it; don't widen the budget.
- **Test coverage = 100%, lines *and* regions.** Enforced by `make cov` (`cargo llvm-cov --fail-under-lines 100 --fail-under-regions 100`), a step of the gate. Regions catch what lines miss: an untaken branch or an unreached `?` error edge on a line that ran. If something can't be tested, redesign it so it can be — delete the dead path or take its dependency through the call signature. Untestable code is not built, and a region is never suppressed with an attribute or a `#[cfg]`.
- **Statelessness is non-negotiable.** No globals, no module-level mutable state, no implicit sessions. Pass state explicitly through call signatures.
- **Output formats are machine-first.** New outputs must be parseable without heuristics. Pretty-printing is a separate concern.

## Workflow

Task tracking is `bl` (see `bl --skill`). One agent takes a task all the way through — there is no separate review step or reviewer. All edits happen in `bl claim`-created worktrees; never edit `main` directly. Standard flow: `bl claim` (prints the worktree) → work in the worktree → `cd` to repo root → `bl close -m "<message>"`.

`bl close` is the sole delivery and gate. It never merges `main` for you: it refuses unless `main` is already in your work branch, so you merge and re-test in the worktree first. Then it runs the repo's `pre-commit` hook on that exact tree, squashes the worktree diff to `main`, and tears the worktree down. A hook failure aborts the close and leaves the task claimed for the fix.

## The gate

`make check` is the complete gate: `size → fmt-check → lint → posix → cov` (≤300-line files in every source language, `cargo fmt --check`, clippy `-D warnings`, the POSIX conformance suite, 100% line + region coverage). The pre-commit hook (`.githooks/pre-commit`) does not run them on this machine — this laptop does not compile in a gate (ops bl-1f80, `~/ops/remote-builds.md` "Phase 2"). It `exec`s `bl-gate` (userconf; the one copy of the gate body for every repo here), which exports `BALLS_TOOLCHAIN` (`rustc -V`, the `rust-toolchain.toml` pin on both sides), asks `bl-speculate check` for a verified verdict on the staged tree, and otherwise has the noodlezoo builder run `make check` and sign one (`bl-remote-gate`; runbook `~/ops/noodlezoo/docs/builder.md`): exit 0 is a pass, 1 means the builder failed the tree (`ssh builder cat /tank/build/out/<sha>/log`), 75 means no verdict — nothing recorded, commit refused, never `cargo test` instead. `cargo tarpaulin` and `cargo llvm-cov` are shimmed on this laptop and refuse to run; to see tests or coverage before committing, `bl-remote-run <target>` runs any make target on the builder and streams the log. `.github/workflows/ci.yml` runs the same targets; nobody restates a step the Makefile defines. Run `make install-hooks` once per clone.

The gate judges the **tree**, never the index: every step sweeps `git ls-files`, and the builder's verdict is keyed on the tree hash, so a docs-only commit pays for clippy, the POSIX suite and coverage too — or hits the verdict cache. A gate keyed on the staged set silently skipped on every `bl close`, whose worktree is already committed (bl-0066, bl-a68b).

## Build

```
make install-hooks      # one-time: seat .githooks/* in .git/hooks
make check              # the complete gate (what the builder runs)
make setup              # install cargo-llvm-cov, if you run `make check` here
make test               # cargo test
make cov                # cargo llvm-cov, requires 100% lines and regions
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
