.PHONY: setup build test fmt fmt-check lint cov size package posix posix-musl check install-hooks clean

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

# The crates.io packaging gate: builds the .crate the registry would receive and
# verifies it compiles from its own contents. --locked so the packaged Cargo.lock
# is the one we tested. Never publishes; the real upload is a deliberate manual act.
package:
	cargo publish --dry-run --locked

# The POSIX conformance gate (docs/design/posix.md): a /bin/sh harness spawning
# the real binary at the process boundary — argv, streams, exit statuses,
# signals, lifecycle. `posix` gates the dev build; `posix-musl` the shipped
# x86_64-unknown-linux-musl artifact (the release pipeline runs the suite
# against the exact stripped binary it uploads).
posix:
	cargo build
	scripts/posix-suite.sh target/debug/frot

posix-musl:
	cargo build --release --target x86_64-unknown-linux-musl
	scripts/posix-suite.sh target/x86_64-unknown-linux-musl/release/frot

# The complete gate, in the old hook's order: the line cap first (every source
# language, ahead of the cargo steps a Rust-less tree would make vacuous), then
# cheap-to-expensive. This laptop never runs it; the builder does (bl-254b).
check: size fmt-check lint posix cov

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
