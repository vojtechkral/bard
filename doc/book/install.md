# Installation

Binary packages are provided for [Linux](#linux) and [MS Windows](#ms-windows) on x86-64.\
On other platforms or architectures, you can [build from sources](#form-sources).

## Linux

The following are provided:

- [Static binary](https://github.com/vojtechkral/bard/releases/latest/download/bard)
- [Deb package](https://github.com/vojtechkral/bard/releases/latest/download/bard.deb)
- [RPM package](https://github.com/vojtechkral/bard/releases/latest/download/bard.rpm)
- [AUR package](https://aur.archlinux.org/packages/bard), [AUR binary package](https://aur.archlinux.org/packages/bard-bin)
- Docker: [`vojtechkral/bard`](https://hub.docker.com/repository/docker/vojtechkral/bard)

To build PDFs, a TeX distribution is needed. Please consult your distribution package repository for the appropriate package.
The default Bard template typically requires an extended TeXLive package such as `texlive-xetex` on Ubuntu,
`texlive-latexextra` on Arch or `texlive-scheme-medium` on Fedora.

Alternatively, you can install the [Tectonic](https://tectonic-typesetting.github.io/) engine.

See the [TeX Configuration](./tex.md) chapter for how to configure TeX use.

## MS Windows

The following are provided:

- [Portable binary](https://github.com/vojtechkral/bard/releases/latest/download/bard.exe)
- [Portable binary without Tectonic](https://github.com/vojtechkral/bard/releases/latest/download/bard-no-tectonic.exe)

The portable binary linked above contains the [Tectonic](https://tectonic-typesetting.github.io/en-US/) TeX system, so no additional software is needed. If you'd like to use a different TeX system, such as [MiKTeX](https://miktex.org/), the Tectonic-less binary is recommended.

See the [TeX Configuration](./tex.md) chapter for how to configure TeX use.

## From Sources

You will first need the Rust toolchain to build from sources &ndash; it can be installed via [Rustup](https://rustup.rs/) or from your OS package repository, if available.

Once the Rust toolchain is installed, i.e., the `rustc` and `cargo` commands are available, use the following command to build and install from sources using `cargo`:

    cargo install -f bard
