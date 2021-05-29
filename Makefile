.PHONY: release
release: target/release/bard

target/release/bard:
	cargo build --release

.PHONY: examples
examples: release
	cargo test
	cd default && cargo run -- make
	cd example && cargo run -- make

.PHONY: compat
compat:
	cargo +1.46 check --tests
