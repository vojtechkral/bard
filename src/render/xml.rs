//! XML Renderer.

use std::fs::File;
use std::io;
use std::io::Write;

use super::Render;
use super::RenderContext;
use crate::app::App;
use crate::prelude::*;
use crate::ProgramMeta;

use crate::project::Format;
use crate::project::Output;
use crate::util::xml_support::*;
use crate::xml_write;

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
        .field_opt(dpi)?
        .field(tex_runs)?
        .field_opt(script)?
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
