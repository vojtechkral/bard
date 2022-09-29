fmt:
	cargo check --tests
	cargo fmt

.PHONY: msrv
msrv:
	cargo +$(shell yq -r .env.MSRV .github/workflows/CI.yaml) check --tests

.PHONY: lint
lint: msrv
	cargo fmt -- --check
	cargo clippy
	cargo check --features clap/deprecated
	cargo audit

.PHONY: release
release: target/release/bard

target/release/bard:
	cargo build --release

.PHONY: test
test:
	cargo test
	cargo test -- --ignored

.PHONY: examples
examples:
	cd default && cargo run -- make
	cd example && cargo run -- make
	cd tests/test-projects/all-features && cargo run -- make
