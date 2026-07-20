.PHONY: setup build test fmt lint cov precommit-install precommit-run clean

setup:
	cargo install cargo-llvm-cov --locked

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

clean:
	cargo clean
