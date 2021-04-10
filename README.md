# bard

[![Build Status](https://travis-ci.com/vojtechkral/bard.svg?branch=main)](https://travis-ci.com/vojtechkral/bard) [![crates.io](https://img.shields.io/crates/v/bard.svg)](https://crates.io/crates/bard)

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
and the [Example project](./example).

## Features

- _bard_ is project-oriented: A single [`bard.toml` file](./doc/bard.toml.md) defines inputs, outputs and
  configuration options, similar to how many static site generators work.
- Easy to use [input format](./doc/markdown.md), you probably already understand it.
- Output formats:
    - PDF via TeX
    - HTML
    - [Hovorka XML](http://karel-hovorka.eu/zpevnik/)
    - JSON (for machine processing)
- [Transposition and notation conversion](./doc/transposition.md)
    - Optionally with a secondary chord set
- [Templating](./doc/templates.md): Outputs are fully customizable with [Handlebars](https://handlebarsjs.com/) templates.

## Installation

There are no packages yet. For now, you'll probably have to compile from sources using [Rust toolchain](https://rustup.rs/):

    cargo install -f bard

Windows executables are [available](https://github.com/vojtechkral/bard/releases), but they were not tested yet.

To generate PDFs a TeX engine is needed. On Linux it is recommended to use either `xelatex` provided by your distro or install [Tectonic](https://tectonic-typesetting.github.io/en-US/). On Windows [MiKTeX](https://miktex.org/) could hopefully work.

Improvements to this situation are Coming Soon™.

## Usage

To start a new songbook project, create a new directory, navigate in it with a command line and type:

    bard init

This will create a skeleton project with a `bard.toml` file and a `songs` subdirectory with one example Markdown song file.

To compile the project and generate output files type:

    bard make

While editing the `bard.toml` file or song source files, it would become annoying to have to type `bard make` every time there's a change. For this reason there's another command:

    bard watch

... which will make _bard_ run continuously, watching for changes in sources files.
It will then re-compile the songbook every time there's a change. Use `Ctrl` + `C` to stop it.

## FAQ

#### Why is the default TeX template done the way it is?

The default layout is optimized for songbooks that are fairly portable (A5 format)
and yet offer hopefully fairly good legibility at that size. They are meant to handle
travel and outdoor situations as well as possible.
This is why the font is fairly large, the chords in bold and color,
and generally the page real estate tends to be used as much as possible.

I've tried reading a songbook illuminated only by a campfire or a half-working flashlight
over someone's shoulder way too many times to tolerate small fonts and mostly empty pages.

#### Was this software developed with <3 ?

As a matter of fact, yes, this tool was made by less than three
developers. It's really just me so far.
