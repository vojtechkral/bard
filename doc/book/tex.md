# TeX Configuration

The T<sub>E</sub>X typesetting system is used to render PDFs. Currently, bard supports the [XeLaTeX](https://www.overleaf.com/learn/latex/XeLaTeX) and [Tectonic](https://tectonic-typesetting.github.io/) implementations. By default, bard attempts to use one of these by searching for the `xelatex` and `tectonic` binary, in that order.

Additionally, the Windows binary embeds the Tectonic software inside, so it doesn't need a TeX distribution to be installed on the system.
However, it still performs the lookup so that if a XeLaTeX distro or Tectonic is installed, it will be preferred over the embedded one.

The automatic lookup can be overridden in two ways:
- using the `BARD_TEX` environment variable, and
- using the `tex` option in `bard.toml` top-level section.

If both are used, the `BARD_TEX` variable takes precedence.

The syntax for both is:

```
distro_type
```
or
```
distro_type:path
```
The `distro_type` can be `xelatex`, `tectonic`, or `none` (see below). On Windows, it may also be `tectonic-embedded` to force the usage of embedded Tectonic.

Optionally a path to a specific binary may be specified after the `:`.

##### Examples

On the command line via the `BARD_TEX` variable:

```sh
BARD_TEX=tectonic bard make
```
&ndash; use Tectonic, look up the `tectonic` binary in `$PATH`.

```sh
BARD_TEX=tectonic:/opt/tectonic/bin/tectonic bard make
```
&ndash; use a Tectonic binary at `/opt/tectonic/bin/tectonic`.

Using `bard.toml`:

```toml
tex="xelatex:C:\\Programs\\TeX\\xelatex.exe"
```
&ndash; use a XeLaTeX binary at `C:\Programs\TeX\xelatex.exe`.

### Number of TeX passes

By default, bard runs three passes of the TeX engine to ensure page numbers are correctly computed.

To adjust this amount, modify the `tex_runs` variable in the relevant `[[output]]` section.
For example, to only run TeX once:

```toml
[[output]]
file = "songbook.pdf"
tex_runs = 1
```
### Preserving TeX files

The TeX file as well as temporary files produced by TeX are automatically removed by bard.

To keep them, use `bard make -k` to keep the TeX file and `bard make -kk` to also keep the temporary TeX files.

### Preventing running TeX

If you wish the TeX engine is not run at all, you can:
- use `bard make -p`,
- set `BARD_TEX=none`, or
- set `tex=none` in `bard.toml`

### ToC sorting configuration

When [sorted ToC](./project.md#toc-order) is enabled, bard modifies a TeX intermediate `.toc` file between TeX runs
by sorting its lines with a regex. The built-in default regex should work with the default template and widely used TeX engines.
If, however, the default doesn't work or a [custom template](templates.md) is used, the regex can be set using `toc_sort_key`:

```toml
[[output]]
file = "songbook.pdf"
toc_sort = true
toc_sort_key = "numberline\\s+\\{[^}]*}([^}]+)"
```

The regex must contain a capture group, ie. `(...)`, which is the sorting key.
