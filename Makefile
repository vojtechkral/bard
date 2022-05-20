
MSRV = 1.56

.PHONY: msrv
msrv:
	cargo +$(MSRV) check --tests

.PHONY: check
check: msrv
	cargo fmt -- --check
	cargo clippy
#	cargo audit
#   ^ cargo-audit is currently broken on Arch

.PHONY: release
release: target/release/bard

target/release/bard:
	cargo build --release

.PHONY: examples
examples:
	cargo test
	cd default && cargo run -- make
	cd example && cargo run -- make
