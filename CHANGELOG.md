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
