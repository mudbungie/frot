.PHONY: setup build test fmt fmt-check lint cov size leak-scan package posix posix-musl check fidelity-record install-hooks clean

# The build authority. Every gate step has ONE home here; `make check` is the
# complete gate and the exact target the noodlezoo builder runs on a staged
# tree (.githooks/pre-commit, AGENTS.md "The gate"), so the hook, CI and a
# hand-run `make check` cannot drift into three definitions of "green".

setup:
	@command -v cargo-llvm-cov >/dev/null 2>&1 || cargo install cargo-llvm-cov --locked

build:
	cargo build --release

test:
	cargo test

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

lint:
	cargo clippy --all-targets -- -D warnings

cov:
	cargo llvm-cov --fail-under-lines 100 --fail-under-regions 100

# The source-file line cap. Two facts, one home each and composed only here, so
# the hook, CI and tests/hygiene/size.rs cannot disagree: source-files.sh owns
# WHICH files are source (Rust, the JS prelude, the gate shell — never
# tests/fixtures, which is data), line-limit.sh owns HOW LONG one may be.
size:
	scripts/source-files.sh | scripts/line-limit.sh

# The disclosure gate (bl-59dc): scripts/leak-rules.sh is the table of what may
# not be published — addresses, ASNs, coordinates, credentials, home paths,
# emails, transcripts — and leak-scan.sh the mechanism, reading the INDEX.
# --self-test first: a leak gate dies by silently matching nothing, so every
# rule must still flag its own fixture and a planted address must fail the
# scan. The machine-global balls plugin bl-leak-gate runs this same scanner
# over each task-store commit; the scanner's presence is that opt-in.
leak-scan:
	@scripts/leak-scan.sh --self-test
	@scripts/leak-scan.sh

# The crates.io packaging gate: builds the .crate the registry would receive,
# verifies it compiles from its own contents, then leak-scans the files INSIDE
# it — the tarball, not the tree, is what a release publishes and can never
# take back. --locked so the packaged Cargo.lock is the one we tested. Never
# publishes; the upload belongs to the release workflow.
package:
	cargo publish --dry-run --locked
	scripts/leak-scan.sh --crate "$$(ls -t target/package/frot-*.crate | head -n1)"

# The POSIX conformance gate (docs/design/posix.md): a /bin/sh harness spawning
# the real binary at the process boundary — argv, streams, exit statuses,
# signals, lifecycle. `posix` gates the dev build; `posix-musl` IS the shipped
# x86_64-unknown-linux-musl artifact's gate — build, strip, the 5-15 MiB
# envelope (scripts/check-size.sh), then the suite against the stripped binary.
# CI runs it on every tree and keeps the file; the release pipeline attaches
# that exact file to the GitHub Release and never rebuilds (ci.yml `artifact`).
posix:
	cargo build
	scripts/posix-suite.sh target/debug/frot

MUSL_BIN := target/x86_64-unknown-linux-musl/release/frot
posix-musl:
	cargo build --release --locked --target x86_64-unknown-linux-musl
	strip $(MUSL_BIN)
	scripts/check-size.sh $(MUSL_BIN)
	scripts/posix-suite.sh $(MUSL_BIN)

# The complete gate, in the old hook's order: the line cap first (every source
# language, ahead of the cargo steps a Rust-less tree would make vacuous), then
# cheap-to-expensive. This laptop never runs it; the builder does (bl-254b).
check: size leak-scan fmt-check lint posix cov

# Re-record the fidelity oracle's captures (docs/design/fidelity.md): drives a
# local headless Chrome over tests/fixtures/fidelity/*.html and rewrites the
# stamped capture beside each page. NOT a gate step and never run by CI — the
# comparison (tests/fidelity.rs, inside `cargo test`) is offline. A deliberate,
# dated act: every new delta it surfaces must be dispositioned in
# tests/fidelity/ledger.rs before the captures can land.
fidelity-record:
	scripts/fidelity-record

# Arm this clone's git hooks: one symlink per file in .githooks/, seated in the
# repo's own hooks directory. Symlinks, not copies, so an updated hook is live
# without a re-run. NOT `core.hooksPath`: this machine sets it globally to a
# chain hook that execs .git/hooks/<name>, so seating the links where git
# already looks keeps both. Refused from a linked worktree: `bl close` deletes
# those, and links pointing into one would rot the moment the ball closed.
install-hooks:
	@top=$$(git rev-parse --path-format=absolute --show-toplevel) && \
	common=$$(git rev-parse --path-format=absolute --git-common-dir) && \
	if [ "$$common" != "$$top/.git" ]; then \
	  echo "install-hooks: run this in the main checkout, not a linked worktree" >&2; \
	  exit 1; \
	fi; \
	mkdir -p "$$common/hooks"; \
	for h in .githooks/*; do \
	  ln -sfn "$$top/$$h" "$$common/hooks/$${h#.githooks/}"; \
	done; \
	echo "hooks: seated $$(ls .githooks | tr '\n' ' ')in $$common/hooks"

clean:
	cargo clean
