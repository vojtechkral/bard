//! XML Renderer.
//!
//! This module defines `RXml` and how AST from `book` is serialized into XML.

use std::fs::File;
use std::io;
use std::io::Write;

use super::Render;
use super::RenderContext;
use crate::app::App;
use crate::book::{
    Block, BulletList, Chord, ChorusRef, HtmlTag, Image, Inline, Link, Song, SongRef, Verse,
    VerseLabel,
};
use crate::prelude::*;
use crate::ProgramMeta;

mod xml_support;
use crate::project::Format;
use crate::project::Output;
use crate::xml_write;
use xml_support::*;

xml_write!(struct Chord {
    chord,
    alt_chord,
    backticks,
    baseline,
    inlines,
} -> |w| {
    w.tag("chord")
        .attr(chord)
        .attr_opt("alt-chord", alt_chord.unwrap())
        .attr(backticks)
        .attr(baseline)
        .content()?
        .many(inlines)?
});

xml_write!(struct Link {
    url,
    title,
    text,
} -> |w| {
    w.tag("link")
        .attr(url)
        .attr(title)
        .content()?
        .text(text)?
});

xml_write!(struct Image {
    path,
    title,
    class,
    width,
    height,
    full_path,
} -> |w| {
    let _ = full_path;
    w.tag("image",)
        .attr(path)
        .attr(title)
        .attr(class)
        .attr(width)
        .attr(height)
});

xml_write!(struct ChorusRef {
    num,
    prefix_space,
} -> |w| {
    w.tag("chorus-ref")
        .attr_opt("num", &num.unwrap().map(|n| format!("{}", n)))
        .attr(prefix_space)
});

xml_write!(struct HtmlTag {
    name,
    attrs,
} -> |w| {
    let tag = w.tag("tag").attr(name);
    let attrs = attrs.unwrap();
    if attrs.is_empty() {
        return tag.finish();
    } else {
        tag.content()?.value(attrs)?
    }
});

xml_write!(enum Inline |w| {
    Text { text } => { w.write_text(text)?; },
    Chord(c) => { w.write_value(c)?; },
    Break => { w.tag("br").finish()?; },
    Emph(i) => { w.tag("emph").content()?.many(i)?.finish()?; },
    Strong(i) => { w.tag("strong").content()?.many(i)?.finish()?; },
    Link(l) => { w.write_value(l)?; },
    Image(i) => { w.write_value(i)?; },
    ChorusRef(cr) => { w.write_value(cr)?; },
    HtmlTag(tag) => { w.write_value(tag)?; },

    Transpose(..) => { unreachable!() },
});

xml_write!(struct Verse {
    label,
    paragraphs,
} -> |w| {
    use VerseLabel::*;
    let label = label.unwrap();
    let label_type = match label {
        Verse(..) => "verse",
        Chorus(..) => "chorus",
        Custom(..) => "custom",
        None {} => "none",
    };

    let label = match label {
        Verse(n) | Chorus(Some(n)) => Some(format!("{}", n)),
        Custom(s) => Some(s.to_string()),
        _ => Option::None,
    };

    w.tag("verse")
        .attr(("label-type", label_type))
        .attr_opt("label", &label)
        .content()?
        .many_tags("p", paragraphs)?
});

xml_write!(struct BulletList { items, } -> |w| {
    w.tag("bullet-list").content()?.many_tags("item", items)?
});

xml_write!(enum Block |w| {
    Verse(verse) => { w.write_value(verse)?; },
    BulletList(l) => { w.write_value(l)?; },
    HorizontalLine => { w.tag("hr").finish()?; },
    Pre { text } => { w.tag("pre").content()?.text(text)?.finish()?; },
    HtmlBlock(i) => { w.tag("html-block").content()?.many(i)?.finish()?; },
});

xml_write!(struct Song {
    title,
    subtitles,
    blocks,
    notation,
} -> |w| {
    w.tag("song")
        .attr(title)
        .attr(notation)
        .content()?
        .many_tags("subtitle", subtitles)?
        .many(blocks)?
});

xml_write!(struct ProgramMeta {
    name,
    version,
    description,
    homepage,
    authors,
} -> |w| {
    w.tag("program")
        .content()?
        .field(name)?
        .field(version)?
        .field(description)?
        .field(homepage)?
        .field(authors)?
});

impl XmlWrite for Format {
    fn write<W>(&self, mut writer: &mut Writer<W>) -> quick_xml::Result<()>
    where
        W: io::Write,
    {
        writer.write_text(self)
    }
}

xml_write!(struct Output {
    file,
    template,
    format,
    toc_sort,
    toc_sort_key,
    sans_font,
    font_size,
    dpi,
    tex_runs,
    script,
    book_overrides,
} -> |w| {
    let _ = file;
    let _ = template;
    let _ = book_overrides;
    w.tag("output")
        .content()?
        .field_opt(format)?
        .field(sans_font)?
        .field(font_size)?
        .field(toc_sort)?
        .field(toc_sort_key)?
        .field(dpi)?
        .field(tex_runs)?
        .field_opt(script)?
});

xml_write!(struct SongRef {
    title,
    idx,
} -> |w| {
    w.tag("song-ref")
        .attr(title)
        .attr(idx)
});

xml_write!(struct RenderContext<'a> {
    book,
    songs,
    songs_sorted,
    notation,
    output,
    program,
} -> |w| {
    w.tag("songbook")
        .attr(notation)
        .content()?
        .comment("The [book] section in bard.toml")?
        .field(book)?
        .comment("References to <song> elements in alphabetically-sorted order")?
        .value_wrap("songs-sorted", songs_sorted)?
        .comment("Fields in the [[output]] section in bard.toml")?
        .value_wrap("output", output)?
        .comment("Software metadata")?
        .value(program)?
        .comment("Song data")?
        .field(songs)?
});

#[derive(Debug, Default)]
pub struct RXml;

impl RXml {
    pub fn new() -> Self {
        Self
    }
}

impl Render for RXml {
    fn render(&self, _app: &App, output: &Path, context: RenderContext) -> anyhow::Result<()> {
        File::create(output)
            .map_err(Error::from)
            .and_then(|f| {
                let mut writer = Writer::new_with_indent(f, b' ', 2);
                context.write(&mut writer)?;

                let mut f = writer.into_inner();
                f.write_all(b"\n")?;
                Ok(())
            })
            .with_context(|| format!("Error writing output file: {:?}", output))
    }
}
