# Fonts

bard brings along the default font files so that output is rendered consistently on all systems.

The font files are stored in the `output/fonts` directory. These files are used by TeX when generating PDF files, but
once the PDF file is generated, it no longer needs to refer to the `fonts` directory &ndash; you can distribute just the PDF file.

The HTML output, on the other hand, links to these files, so it is needed to distribute the `fonts` directory along with the HTML file,
such as when uploading it onto the web.

By default, the _Droid Sans_ font is used to display all text except chords and _Noto Sans_ is used to display chords, so that they are more distinguished from lyrics.

### Using sans font everywhere

To use sans font for everything, use the `sans_font` setting in the `output` section in `bard.toml`:

```toml
[[output]]
file = "songbook.pdf"
sans_font = true
```

This works for the HTML output as well.

### PDF font size

The default TeX template uses the `12pt` font size. To use a different size, configure the `font_size` variable in the `output` section in `bard.toml`:

```toml
[[output]]
file = "songbook.pdf"
font_size = 11
```

This only applies to the PDF output. Please note that TeX classes usually support only certain font sizes, the default template
supports the following sizes: 9, 10, 11, 12, 14, 17, 20, 25, 30, 36, 48, and 60.

The HTML file doesn't set a specific font size and instead relies on the default font size used by the browser.
