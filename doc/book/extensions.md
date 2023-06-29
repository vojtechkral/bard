# Custom Extensions

Bard Markdown syntax can be extended using "HTML" code. Since inline HTML code is otherwise unused
(it would not be very useful in TeX/PDF output), it is repurposed as a way to call into user-defined extensions.

For example, in the following Markdown:

```md
# Song

1. <foo>example</foo>.
```

In [the AST](./templates.md#the-ast), the tag pair is represented by inlines of type `i-tag`.
The tag name is prefixed with `h-` and dispatched to an _inline partial_ of that name.
For the closing tag, the slash `/` is included in the name. So, in this example, first, a partial named `h-foo` is called, then the text `example` is rendered, and then a partial named `h-/foo` is called. Since, by default, the two partials `h-foo` and `h-/foo` don't exist, the tags don't do anything; only `example` will be rendered. However, you can define those two partials in your template.

"HTML" tags may also enclose whole blocks of text like so:

```md
<foo>

1. O the `G`summer `C`time `G`has come
And the `C`trees are sweetly `G`bloomin'...

</foo>
```

In this example, it is important that there is a newline between the opening tag (`<foo>`) and the following block. Due to Markdown parsing rules, if the block follows without a newline, it is considered part of the HTML code and not parsed as Markdown (Bard warns you if this happens).

### Example: Font size

Suppose we want an extension to render a part of the lyrics in a smaller font. We'll name it `small` and use it like this:

```md
# Song

1. Normal text size.
<small>This should be in a smaller font...</small>
```

To make this work, two _partial inlines_ need to be added to the TeX template:

```html
{{#*inline "h-small"}}\small{}{{/inline}}
{{#*inline "h-/small"}}\normalsize{}{{/inline}}
```

And in the HTML template, they will be defined as:

```hml
{{#*inline "h-small"}}<small>{{/inline}}
{{#*inline "h-/small"}}</small>{{/inline}}
```

This will render the text a bit smaller in both the PDF and HTML output.

### Attributes

It is possible to use HTML attributes to parametrize an extension. Every attribute defined on the HTML tag will
be available in the `h-` inline as a Handlebars variable.

### Example: Youtube embed

Let's suppose we'd like to make it possible to embed a YouTube video in the HTML output.
We'll be adding links in the song sources like this:

```md
<youtube id="zlfbhc3NBTA">
```

The `id` attribute references the YouTube video ID we'd like to link.

To make this work, we'll add the following inline in the HTML template:

```html
{{#*inline "h-youtube"}}
  <iframe src="https://www.youtube.com/embed/{{id}}" allowfullscreen frameborder="0"></iframe>
{{/inline}}
```

The `{{id}}` part renders the attribute `id` that we've passed in from Markdown.

In paper documents, video links are not very practical, so we won't be defining an `h-youtube` inline in the TeX template.
The element will simply be ignored in TeX.
