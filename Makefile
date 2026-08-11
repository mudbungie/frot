.PHONY: setup build test fmt lint cov package precommit-install precommit-run clean

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

# The crates.io packaging gate: builds the .crate the registry would receive and
# verifies it compiles from its own contents. --locked so the packaged Cargo.lock
# is the one we tested. Never publishes; the real upload is a deliberate manual act.
package:
	cargo publish --dry-run --locked

clean:
	cargo clean
