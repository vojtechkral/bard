# JSON and XML Output

Besides the usual PDF and HTML format, Bard can also output songbook data in JSON and XML.
This is primarily useful for template writing, debugging or further processing.

To get JSON and/or XML files, add them as additional outputs:

```toml
[[output]]
file = "songbook.json"

[[output]]
file = "songbook.xml"
```

The JSON data is the AST of the whole parsed songbook, and it is exactly the same as the data which gets passed to the rendering templates.

The XML data contains the same data semantically but is exported in a shape more suitable for this format.
**Warning:** The XML format is somewhat experimental and is not covered by the backwards compatibility guarantee, unlike JSON.

Both formats are defined within the source code; formal schema definitions are not available.
