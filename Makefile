.PHONY: setup build test fmt lint cov size package posix posix-musl precommit-install precommit-run clean

setup:
	@command -v cargo-llvm-cov >/dev/null 2>&1 || cargo install cargo-llvm-cov --locked

precommit-install:
	@mkdir -p .git/hooks
	@ln -sf ../../.githooks/pre-commit .git/hooks/pre-commit
	@echo "pre-commit installed"

precommit-run:
	@.githooks/pre-commit

build:
	cargo build --release

test:
	cargo test

fmt:
	cargo fmt

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

clean:
	cargo clean
