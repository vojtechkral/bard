fmt:
	cargo check --tests
	cargo fmt

.PHONY: msrv
msrv:
	CARGO_TARGET_DIR=target-msrv cargo +$(shell yq -r .env.MSRV .github/workflows/CI.yaml) check --tests

.PHONY: lint
lint:
	cargo fmt -- --check
	cargo clippy
	cargo check --features clap/deprecated

.PHONY: audit
audit:
	cargo audit

.PHONY: release
release: target/release/bard

target/release/bard:
	cargo build --release

.PHONY: tests-quick
tests-quick:
	cargo test

.PHONY: tests-ignored
tests-ignored:
	cargo test -- --ignored --nocapture

.PHONY: test
test: tests-quick tests-ignored msrv audit lint

.PHONY: examples
examples:
	cd default && cargo run -- make
	cd example && cargo run -- make
	cd tests/test-projects/all-features && cargo run -- make
