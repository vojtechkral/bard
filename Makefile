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
#
# NB. I am not using cargo-vcpkg for this because it can't do the release-only build trick
# from https://github.com/microsoft/vcpkg/issues/10683
# Also tbh cargo-vcpkg doesn't report errors very well.

VCPKG_REV = 6f7ffeb18f99796233b958aaaf14ec7bd4fb64b2

target-tectonic/vcpkg/vcpkg:
	git clone https://github.com/Microsoft/vcpkg.git -- target-tectonic/vcpkg
	cd target-tectonic/vcpkg && git checkout $(VCPKG_REV)
	VCPKG_DISABLE_METRICS=1 target-tectonic/vcpkg/bootstrap-vcpkg.sh
	echo 'set(VCPKG_BUILD_TYPE release)' >> target-tectonic/vcpkg/triplets/x64-linux.cmake

.PHONY: with-tectonic
with-tectonic: target-tectonic/vcpkg/vcpkg
	target-tectonic/vcpkg/vcpkg install fontconfig freetype 'harfbuzz[icu,graphite2]'
	CARGO_TARGET_DIR=target-tectonic VCPKG_ROOT="$(PWD)/target-tectonic/vcpkg" TECTONIC_DEP_BACKEND=vcpkg cargo build --release --features tectonic

.PHONY: test-tectonic
test-tectonic: target-tectonic/vcpkg/vcpkg
	target-tectonic/vcpkg/vcpkg install fontconfig freetype 'harfbuzz[icu,graphite2]'
	CARGO_TARGET_DIR=target-tectonic VCPKG_ROOT="$(PWD)/target-tectonic/vcpkg" TECTONIC_DEP_BACKEND=vcpkg cargo nextest run --release --run-ignored all --features tectonic

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
