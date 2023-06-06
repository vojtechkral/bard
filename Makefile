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

.PHONY: test
test:
	cargo nextest run --run-ignored all

.PHONY: check
check: test test-tectonic msrv lint audit

.PHONY: examples
examples:
	cd default && cargo run -- make
	cd example && cargo run -- make
	cd tests/test-projects/all-features && cargo run -- make

.PHONY: book
book:
	$(MAKE) -C doc book

.PHONY: serve-site
serve-site:
	$(MAKE) -C doc serve-site

.PHONY: serve-book
serve-book:
	$(MAKE) -C doc serve-book

.PHONY: book-clean
book-clean:
	$(MAKE) -C doc clean


# tectonic embedding targets

.PHONY: with-tectonic
with-tectonic:
	CARGO_TARGET_DIR=target-tectonic cargo build --release --features tectonic

.PHONY: test-tectonic
test-tectonic:
	CARGO_TARGET_DIR=target-tectonic cargo nextest run --release --run-ignored all --features tectonic

# using vcpkg on windows

VCPKG_REV = 1c5a340f6e10985e2d92af174a68dbd15c1fa4e1 # https://github.com/microsoft/vcpkg/pull/29067

target/vcpkg/vcpkg.exe:
	git clone https://github.com/Microsoft/vcpkg.git -- target/vcpkg
	cd target/vcpkg && git checkout $(VCPKG_REV)
	VCPKG_DISABLE_METRICS=1 target/vcpkg/bootstrap-vcpkg.bat
	echo 'set(VCPKG_BUILD_TYPE release)' >> target/vcpkg/triplets/x64-windows-static.cmake

.PHONY: with-tectonic-windows
with-tectonic-windows: target/vcpkg/vcpkg.exe
	VCPKG_DISABLE_METRICS=1 target/vcpkg/vcpkg install --triplet x64-windows-static icu graphite2 fontconfig freetype 'harfbuzz[icu,graphite2]'
	TECTONIC_DEP_BACKEND=vcpkg VCPKG_ROOT="$(PWD)/target/vcpkg" VCPKGRS_TRIPLET='x64-windows-static' RUSTFLAGS='-Ctarget-feature=+crt-static' cargo build --release --features tectonic

.PHONY: test-tectonic-windows
test-tectonic-windows: target/vcpkg/vcpkg.exe
	VCPKG_DISABLE_METRICS=1 target/vcpkg/vcpkg install --triplet x64-windows-static icu graphite2 fontconfig freetype 'harfbuzz[icu,graphite2]'
	TECTONIC_DEP_BACKEND=vcpkg VCPKG_ROOT="$(PWD)/target/vcpkg" VCPKGRS_TRIPLET='x64-windows-static' RUSTFLAGS='-Ctarget-feature=+crt-static' cargo nextest run --release --run-ignored all --features tectonic
