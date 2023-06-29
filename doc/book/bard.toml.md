# bard.toml Reference

This is a list of fields recognized in the `bard.toml` settings file.\
Most of the fields are optional; only a few are required; these are marked as such.

<div class="thin-code">

```toml
version = 2
```
*Required.* Major version of Bard this project was created with. Used for compatibility checks.

```toml
songs = "*.md"
# or
songs = [ "file1.md", "file2.md", "foo/*.md", "..." ]
```
*Required.* The input files specification. See [Inputs](./project.md#inputs).

```toml
notation = "english"
```
Chord notation used in the input files. Only needed for transposition. See [Notation and Transposition](./transposition.md).

```toml
smart_punctuation = true
```
Whether the Markdown parser should produce smart quotations and ellipsis. See [Punctuation](./songs.md#punctuation).

 ```toml
tex = "xelatex"
```
Specifies which TeX implementation should be used. See [TeX configuration](./tex.md).

### `[[output]]`

The `output` field is an [array of tables](https://toml.io/en/v1.0.0#array-of-tables). Each output may have the following fields:

```toml
file = "songbook.pdf"
```
*Required.* Output file name relative to the `output` directory.

```toml
template = "pdf.hbs"
```
Path to the template file for this output relative to the `templates` directory. (See also [Templates](./templates.md).)

```toml
format = "pdf"
```
Output format. Possible choices: `"pdf"`, `"html"`, [`"hovorka"`](./hovorka.md), [`"json"`](./json-and-xml.md), or [`"xml"`](./json-and-xml.md).
Usually, this isn't required since the format is detected from the `file`'s extension.

```toml
sans_font = false
```
Whether a sans-serif font should be used in PDF and HTML outputs instead of the default serif font.

```toml
font_size = 12
```
Font size in PDF outputs. See [PDF font size](./fonts.md#pdf-font-size).

```toml
toc_sort = true
```
Whether the table of contents should be sorted alphabetically in PDF and HTML outputs. See [ToC order](./project.md#toc-order).

```toml
toc_sort_key = "numberline\\s+\\{[^}]*}([^}]+)"
```
A customized ToC sort key extraction regex for PDF outputs. See [ToC sorting configuration](./tex.md#toc-sorting-configuration).

```toml
dpi = 144.0
```
For PDF outputs, this is the resolution of images in points per inch. For HTML outputs, this is the image scaling factor.
See [DPI settings](./images.md#dpi-settings).

```toml
tex_runs = 3
```
Number of TeX rendering passes when generating PDFs. See [Number of TeX passes](./tex.md#number-of-tex-passes).

```toml
script = "postprocess"
```
Base name of a post-processing script file used for this output _without_ the extension. See [Scripts](./scripts.md).

```toml
book = { front_img = "guitar_pdf.jpg" }
```
Override any field of the `[book]` section (see below) specifically for this output.

### `[book]`

The `book` table describes basic metadata about your songbook; it is used by the rendering templates.

```toml
title = "Bard Songbook"
```
The main title of the songbook.

```toml
subtitle = "An example project"
```
Sub-title, shown on the title page as well, but in smaller font.

```toml
chorus_label = "Ch"
```
Label to be used for chorus verses without the dot.

```toml
front_img = "guitar.jpg"
```
An image shown on the title page.

```toml
title_note = "A set of a few non-copyrighted songs."
```
An additional note in small font on the bottom of the title page.

</div>
