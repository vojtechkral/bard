# Template helpers

This is a reference of [Handlebars helpers](https://handlebarsjs.com/guide/#custom-helpers) which bard provides to [templates](templates.md).

<div class="hbs-reference">

### `eq a b`

A basic equality check, returns `true` when a JSON value `a` equals `b`.

### `contains object key`

Returns `true` when a JSON `object` contains a value under `key`.

### `cat args…`

Concatenates any number of arguments as one string.\
For example: `{{ cat "Hello, " "World!" }}` will render `"Hello, World!"`.

### `default value default`

Returns `value` if it is not `null`, otherwise returns `default`.

### `matches string regex`

Return true if `string` matches the regular expression `regex`.

### `math a op b`

Evaluates a math expression.\
Examples:
- `{{ math 5 "+" 3 }}` renders `8`.
- `{{ math 23.8 "/" -1.4 }}` renders `-17.0`.

Supported operators:

<div class="table-compact table-no-thead">

| Op   | Meaning |
| ---- | ------- |
| `+`  | Addition |
| `-`  | Subtraction |
| `*`  | Multiplication |
| `/`  | Decimal division (integers are converted to floats)
| `//` | Integer division (both numbers must be integers)
| `%`  | Modulo
| `&`  | Bitwise and (integers only)
| <code>\|</code>  | Bitwise or (integers only) <!-- https://github.com/raphlinus/pulldown-cmark/issues/639 -->
| `^`  | Bitwise xor (integers only)
| `<<` | Bitwise shift left (integers only)
| `>>` | Bitwise shift right (integers only)

</div>

### `img_w path`

Returns the pixel width of an image at `path`.

### `img_h path`

Returns the pixel height of an image at `path`.

### `px2mm size`

Converts a `size` in pixels to millimeters using output's `dpi` settings.\
See also [`scale`](#scale-size) and [Images - DPI](images.md#dpi) for more details.

_Only in TeX templates._

### `pre text`

Performs TeX escaping of the string with spaces replaced by `~`.\
It is recommended to use this helper in triple braces `{{{ pre ... }}}` which suppresses the default escaping function.

_Only in TeX templates._

Example: `{{{ pre "Hello,      World!" }}}`

### `scale size`

Multiplies a `size` by the `dpi` factor in output's settings.\
The result is rounded to the nearest interger.\
See also [`px2mm`](#px2mm-size) and [Images - DPI](images.md#dpi) for more details.

_Only in HTML templates._

### `version_check version`

Performs a version check. The running bard program compares the `version` specified with its internal AST version
and outputs a warning in case the version is incompatible.

Example: `{{~ version_check "1.1.0" ~}}`

</div>
