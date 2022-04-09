
MSRV = 1.56

.PHONY: check
check:
	cargo fmt -- --check
	cargo clippy
	cargo +$(MSRV) check --tests
	cargo audit

.PHONY: release
release: target/release/bard

target/release/bard:
	cargo build --release

.PHONY: examples
examples:
	cargo test
	cd default && cargo run -- make
	cd example && cargo run -- make
