# bard

[![crates.io](https://img.shields.io/crates/v/bard.svg)](https://crates.io/crates/bard) [![CI](https://github.com/vojtechkral/bard/actions/workflows/CI.yaml/badge.svg)](https://github.com/vojtechkral/bard/actions/workflows/CI.yaml)

Markdown → songbooks.

_bard_ is a songbook compiler that reads Markdown files and produces songbooks in PDF, HTML, and [Hovorka](http://karel-hovorka.eu/zpevnik/).

_bard_ reads files like this:

```Markdown
# Wild Mountain Thyme
## Irish & Scottish traditional

1. Oh the `G`summer `C`time `G`has come
And the `C`trees are sweetly `G`bloomin'
And the `C`wild `G`mountain `Em`thyme
Grows `C`around the `Am`bloomin' `C`heather
Will ye `G`go `C`lassie `G`go?

> And we'll `C`all go `G`together to pull `C`wild `G`mountain `Em`thyme
All `C`around the `Am`bloomin' `C`heather, will ye `G`go `C`lassie `G`go?
```

... and creates output like this:

![example-output](./doc/example.png "Example PDF output")

Check out the [Example PDF](https://raw.githubusercontent.com/vojtechkral/bard/main/example/output/songbook.pdf)
from the [Example project](./example).

---

### [Getting Started](https://bard.md/book/install.html)

---

## Features

- _bard_ is a command-line tool
- Project-oriented: A single `bard.toml` defines inputs, outputs and other configuration
- Easy-to-use source format: Markdown
- Output formats:
    - PDF via TeX
    - HTML
    - [Hovorka XML](http://karel-hovorka.eu/zpevnik/)
    - JSON and XML for machine processing
- Transposition and notation conversion
    - Optional auto-generated second chord set
- Templating: Outputs are fully customizable with [Handlebars](https://handlebarsjs.com/) templates

## Code Contributors

[![cotributors](https://contrib.rocks/image?repo=vojtechkral/bard&anon=1)](https://github.com/vojtechkral/bard/graphs/contributors)
