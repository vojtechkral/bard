## 2.0.0 `2023-07-01`

This is a new major version release of bard. This release also comes with new documentation &ndash; [a book](https://bard.md/book).

### Breaking Changes

In bard 2.0, the PDF output via TeX has been overhauled.
The `tex` output has been replaced with a `pdf` output and there is no longer a `process` field.
Instead, bard auto-detects installed TeX and runs the appropriate rendering command. There are a few new options to (optionally) configure TeX lookup.

Along with this, the `bard.toml` project file format changed and is now also versioned..

See the [Migration Guide](https://bard.md/book/migration-2.html) and [TeX Configuration](https://bard.md/book/tex.html).

Other breaking changes:
- Paths in project settings now must be relative only.
- The `hovorka` format now needs to be specified explicitly.
- Alternative HTML file extensions (`.htm`, `.xhtml`, `.xht`) are no longer auto-detected, `.html` is the recommended one.
- `book.backmatter` setting is removed.

Additionally, the MSRV is now increased to Rust `1.64.0`.

### New Features

- New TeX invocation system, as mentioned above.
- TeX/PDF template improved significantly.
- [Tectonic](https://tectonic-typesetting.github.io/) embedding - an optional feature.
- MS Windows is now a fully supported platform.
- [Baseline chords](https://bard.md/book/songs.html?highlight=baselin#lyrics-and-chords) added.
- [Improved fonts support](https://bard.md/book/fonts.html).
    - Bard now brings fonts it uses along.
    - Font size and sans vs. serif can now be configured more easily.
- Integrated [ToC sorting](https://bard.md/book/project.html#toc-order), now also supported in HTML output.
- Improved [images](https://bard.md/book/images.html) support.
    - Image scaling setting for HTML output added.
    - Image sizes are now preloaded and available to templates in book AST.
    - Images are now part of `bard watch` watched paths.
- Markdown [smart punctuation](https://bard.md/book/songs.html?highlight=smart#punctuation) option, enabled by default.
- [Syntax extensions](https://bard.md/book/extensions.html) via "HTML".
- [Scripts](https://bard.md/book/scripts.html) support &ndash; a way to post-process results in arbitrary ways. This is a replacement for the previous `process` field.
- Syntax to turn off alt chords (`!!none`).
- [An XML output](https://bard.md/book/json-and-xml.html) (experimental).
- New prebuilt packages/binaries provided:
    - Windows binaries with and without Tectonic.
    - Deb and RPM packages with statically built binary (without Tectonic).

### Bugfixes & Misc

- Fixed `SIGINT`/interrupt handling.
- Fixed a few parsing errors.
- Fixed templates syntax compliance with reference implementation.
- Fixes in the HTML template.
- A few Handlebars helpers added.
- Dependencies upgrades.
- Testing improvements.

## 1.3.0 `2021-07-23`

In this release:
- Ability to sort ToC alphabetically in HTML and TeX/PDF ([documentation](https://github.com/vojtechkral/bard/blob/b43c5e0e965dd4d4fbc7333dfd9fe7a40ff8cf5b/doc/bard.toml.md#toc-sort-order))
- Simpler postprocess syntax when multiple commands are used

Detailed:
- New subcommand: `util`, so far only contains the `sort-lines` utility designed to sort TeX toc file alphabetically
- Path to bard itself now available in postprocess context, so that ^ can be called
- Bugfix: Return non-zero status on program error
- Support single-string commands in the extended postprocess syntax
- The `songs_sorted` array now avaiable in template context
- Internal changes and code maintanance

## 1.2.1 `2021-07-22`

Patch update only containing a dependency update and a minor fix.

## 1.2.0 `2021-07-16`

- Add support for optional/advanced chords with the ` ``X`` ` syntax.
- Fix auto numbering of numbered verses
- Use `latexmk` by default

## 1.1.0 `2021-05-28`

- Added [`process_win`](https://github.com/vojtechkral/bard/blob/main/doc/bard.toml.md#special-casing-ms-windows) option in `bard.toml`.
- Added the [`-p`/`--no-postprocess` flag](https://github.com/vojtechkral/bard/blob/main/doc/bard.toml.md#skipping-post-processing) to `bard make` and `bard watch`.
- Fixed interpolation of program name in the `process` field.
- Tests improvements and various smaller fixes

## 1.0.3 `2021-04-20`

This is a minor bugfix release, fixes:

- When initializing a template file with the default content, its parent directory wasn't being created.
- Display correct version info in the CLI.

## 1.0.2 `2021-04-10`

Minor bugfix release, fix of an extra dot in default projects' chorus label, code cleanup, test improvements.

## 1.0.1 `2021-04-07`

This is a bugfix release, fixes:

- Create template with default contents when the file doesn't exist (as documented)
- `chorus_label` was in the wrong place in default & example projects (remnant of previous state)

## 1.0.0 `2021-04-03`

The first real release.
