use std::io::Write;
use std::fs::File;

use toml::Value;
use unicode_width::UnicodeWidthStr;

use crate::book::{Item, Verse, List};
use crate::project::{Project, OutputSpec};
use crate::error::*;
use super::Render;


trait StrExt {
    fn pad(&self, c: char, width: usize) -> String;
}

impl<T> StrExt for T
where
    T: AsRef<str>,
{
    fn pad(&self, c: char, width: usize) -> String {
        let mut res = self.as_ref().to_string();
        let n_pad = width.saturating_sub(self.as_ref().width());
        res.reserve(n_pad);
        for _ in 0..n_pad {
            res.push(c);
        }

        res
    }
}

pub struct RTxt;

impl RTxt {
    const MAX_MARGIN: usize = 6;

    fn write_verse(f: &mut File, verse: &Verse) -> Result<()> {
        let Verse { label, lines, .. } = verse;

        let width = if label.is_empty() {
            0
        } else {
            write!(f, "{} ", label)?;
            (label.width() + 1).min(Self::MAX_MARGIN)
        };

        let margin = "".pad(' ', width);

        let mut first = true;
        for line in lines {
            if first {
                first = false;
            } else {
                write!(f, "{}", margin)?;
            }

            let rows = line.chord_rows();
            if rows > 0 {
                for span in &line.spans {
                    write!(f, "{}", span.chord.pad(' ', span.width()))?;
                }
                write!(f, "\n{}", margin)?;
            }
            if rows > 1 {
                for span in &line.spans {
                    write!(f, "{}", span.chord_alt.pad(' ', span.width()))?;
                }
                write!(f, "\n{}", margin)?;
            }
            for span in &line.spans {
                write!(f, "{}", span.lyrics.pad(' ', span.width()))?;
            }

            writeln!(f, "")?;
        }

        writeln!(f, "")?;
        Ok(())
    }
}

impl Render for RTxt {
    fn render<'a>(project: &'a Project, output: &'a OutputSpec) -> Result<&'a OutputSpec> {
        let path = &output.file;
        let mut file = File::create(&path).map_err(|err| ErrorWritingFile(path.to_owned(), err))?;
        let f = &mut file;

        writeln!(f, "")?;

        let book = project.metadata();
        let songs = project.songs();

        if let Some(Value::String(title)) = book.get("title") {
            writeln!(f, "{}\n", title)?;
        }

        if let Some(Value::String(subtitle)) = book.get("subtitle") {
            writeln!(f, "{}\n", subtitle)?;
        }

        if let Some(Value::String(title_note)) = book.get("title_note") {
            writeln!(f, "{}", title_note)?;
        }

        writeln!(f, "\n")?;

        for song in songs {
            // Title
            writeln!(f, "{}", song.title)?;
            let underline: String = "".pad('=', song.title.len());
            writeln!(f, "{}\n", underline)?;

            // Subtitles
            for subtitle in &song.subtitles {
                writeln!(f, "{}", subtitle)?;
            }
            if song.subtitles.len() > 0 {
                writeln!(f, "")?;
            }

            // Items
            for item in &song.content {
                match item {
                    Item::Verse(verse) => Self::write_verse(f, &verse)?,
                    Item::List(List { items }) => {
                        for item in items {
                            writeln!(f, " - {}", item)?;
                        }
                        writeln!(f, "")?;
                    }
                    Item::Time { time } => writeln!(f, "{} / {}\n", time.0, time.1)?,
                    Item::Rule => writeln!(f, "{}\n", "".pad('-', 80))?,
                    Item::Pre { text } => writeln!(f, "{}", text)?,
                }
            }

            writeln!(f, "")?;
        }

        if let Some(Value::String(backmatter)) = book.get("backmatter") {
            writeln!(f, "{}", backmatter)?;
        }

        if let Some(debug) = project.parsing_debug() {
            writeln!(f, "Debug info:")?;

            writeln!(f, "evts_md:")?;
            for evt in &debug.evts_md {
                writeln!(f, "  {}", evt)?;
            }

            writeln!(f, "evts_bard:")?;
            for evt in &debug.evts_bard {
                writeln!(f, "  {}", evt)?;
            }
        }

        Ok(output)
    }
}
