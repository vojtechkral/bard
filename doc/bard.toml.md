# Project configuration

Each _bard_ project is configured with a single file named `bard.toml` in the root of the project folder.

Apart from the `bard.toml` file, a bard project folder contains these subfolders:
 - `songs`: Contains Markdown input files.
 - `templates`: Contains rendering templates.
 - `output`: Output files will be placed here.

The `bard.toml` file has three main parts: top-level section, outputs and book metadata:

### The top-level section

Most important setting in the top section is the `songs` field, which defines inputs, relative to the `songs` folder. The default is:

```toml
songs = "*.md"
```

This is a [glob pattern](https://en.wikipedia.org/wiki/Glob_(programming)). 
An array of filenames or globs can also be used.

Songs are sorted in the resulting songbook in the same order as specified in the `songs` field.
Filenames matched by a glob pattern are sorted alphabetically. If you want the songs to be in a specific order,
you may either prefix each song filename with a number or list them explicitly, as in:

```toml
songs = [
    "Danny Boy.md",
    "Handsome Molly.md",
    "Whiskey in the Jard.md",
    "Wild Mountain Thyme.md",
]
```

There may also be multiple songs in one Markdown file.

Additionally the top-level section may also specify the `notation` field.
This is the [musical notation](https://en.wikipedia.org/wiki/Musical_note#12-tone_chromatic_scale) the source files use.
Default is `"english"`, other choices are `"german"`, `"nashville"`, and `"roman"`. Refer to the [Transposition doc](./transposition.md) for more information.

### Outputs

There may be one or several outputs (`outputs` is an [array of tables](https://github.com/toml-lang/toml#array-of-tables)).
Each output section looks like this:

```toml
[[output]]
file = "songbook.html"
```
The `file` field is optional. Other possible fields understood are:
  - `temaplate` - the path to Handlebars template used for this output. If missing, default template will be created
    under the `templates` folder. This field is not applicable to JSON output.
  - `format` - Output format, can be any of "html", "tex", "hovorka", "json", "auto". If missing, the format is
    guessed from the output file extension.
  - `process` (and `process_win`) - A command that _bard_ executes once the output file is generated.
    By default this is used to run TeX engine, but may be used for arbitrary purposes. See the [Post-processing](#post-processing) section
  - `dpi` - See [Images](./markdown.md#images).

There may be any number of additional fields unrecognized explicitly by _bard_, they will be all passed
to the rendering templates and may be used by them.


### Book metadata

Book metadata are in the final section named `book`, these are all simply passed to the templates.
The ones used by the default templates are:

```toml
[book]
title = "My Songbook"
subtitle = "Collection of songs I like to play"
chorus_label = "Ch."
front_img = "some-image.jpg"
title_note = "John Smith, 2020"
backmatter = "Backmatter page..."
```

... of these only the `title` is mandatory, the rest is optional.

#### Post-processing

Each `[[output]]` section may contain a `process` field with a command and arguments that _bard_ will
run upon succesful rendering of the resulting file.
For example, this is the default for generating PDF from TeX:

```toml
process = "xelatex {{file}}"
```

The field is actually a Handlebars string where the `file` variable will be replaced by the path to the output file.
The variables that are available in the `process` field are:

- `file`: Full (absolute) path to the output file.
- `file_name`: The filename of the output file only, ie. without path.
- `file_stem`: Filename without extension. For example, the stem of `exameple.tex` is `example`.
- `project_dir`: The path to the current project root dir (ie. where `bard.toml` is placed).
- `bard`: Full (absolute) path to the bard executable.

The post-process commands are run in the output directory.

The `process` field also has an extended syntax, which can be used to write multiple commands and/or arguments containing spaces (which is not possible with the simple syntax).
The extended syntax is an array of commands, each of which is an array of command and arguments.
For example, if you wanted to generate PDF and then to zip up the original TeX file, you could write:

```toml
process = [
    ["xelatex", "{{file}}"],
    ["zip", "-9", "{{file_stem}}.zip", "{{file_name}}"],
]
```

##### Special-casing MS Windows

You may also add the `process_win` field. The format is exactly the same as `process`.
This field will be used on MS Windows instead of `process`, which allows to customize the post-processing command for this OS.

##### Skipping post-processing

To skip the post-processing step, pass `-p` or `--no-postprocess` to `bard make` or `bard watch`
