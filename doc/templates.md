# Templates

_bard_ uses the [Handlebars](https://handlebarsjs.com/) template system (`*.hbs` files).
The template files are placed in the `temaplates` folder by default.
The [implementation](https://docs.rs/handlebars) is mostly compatible with the original JS implementation
with the notable exception of `{{else}}` clauses, which are unfortunatelly not supported.

If a template file is specified in `bard.toml` but missing in the `temaplates`,
_bard_ will generate one with default contents for the given output format.

The data context passed to the template is somewhat complex, as it needs to hold the whole songbook.
The data can be dumped as JSON by adding a JSON output in the `bard.toml` file:

```toml
[[output]]
file = "songbook.json"
```

In a nutshell, a songbook is made up of songs, a song is made up of blocks.
Block content depends on the block type. The most common block type - a verse - is made up of
paragraphs and a paragraph is made up of Markdown inlines of sereval types (such as text, chord, emphasis, etc.). Inlines may be nested rescursively.

The default templates make heavy use of Handlebars [Dynamic](https://handlebarsjs.com/guide/partials.html#dynamic-partials)
and [Inline](https://handlebarsjs.com/guide/partials.html#inline-partials) partials to dispatch various kinds of block
and inline types.
For example, for each block in a song, the temaplate will dispatch using

```Handlebars
{{#each blocks}}{{> (lookup this "type") }}{{/each}}
```
... to inlines defined as:

```Handlebars
{{#*inline "b-verse"}}...{{/inline}}

{{#*inline "b-bullet-list"}}...{{/inline}}

{{#*inline "b-horizontal-line"}}...{{/inline}}

{{#*inline "b-pre"}}...{{/inline}}

... etc. ...
```

## Helpers

Additionally to the helpers [provided by Handlebars](https://handlebarsjs.com/guide/builtin-helpers.html),
_bard_ provides a few extra helpers:

- `eq <a> <b>` → boolean
    - Returns `true` if `a` is equal to `b`, otherwise returns `false`.
- `contains <object> <key>` → boolean
    - Returns `true` iff JSON object `object` contains field named `key` (even if the field is set to `null` or `false`).
- `default <value> <default>` → any value
    - if `value` is `null`, returns `default`, otherwise returns `value`.
- `matches <string> <regex>` → boolean
    - Returns `true` iff `string` matches the regular expression `regex`.
- `px2mm <number>` → number
    - Converts `number` to millimeters based on current output's DPI setting (see [Images](./markdown.md#images)).
- `img_w <path>` → number
    - Returns the width of image at `path` in pixels.
- `img_h <path>` → number
    - Returns the height of image at `path` in pixels.
- `pre <text>` → text (TeX format only)
    - Returns `text` with spaces escaped as `~` for whitespace retention in TeX.
