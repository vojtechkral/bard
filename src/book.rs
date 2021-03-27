//! AST of a bard songbook

use std::fs;
use std::path::Path;

use serde::ser::Serialize;

use crate::util::BStr;
use crate::error::*;
use crate::music::Notation;
use crate::parser::Parser;

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum Block {
    #[serde(rename = "b-verse")]
    Verse(Verse),
    #[serde(rename = "b-bullet-list")]
    BulletList(BulletList),
    #[serde(rename = "b-horizontal-line")]
    HorizontalLine,
    #[serde(rename = "b-pre")]
    Pre { text: BStr },
}

impl Block {
    pub fn chorus_num(&self) -> Option<u32> {
        if let Self::Verse(Verse {
            label: VerseLabel::Chorus(num),
            ..
        }) = self
        {
            *num
        } else {
            None
        }
    }

    pub fn remove_chorus_num(&mut self) {
        if let Self::Verse(verse) = self {
            if let VerseLabel::Chorus(num) = &mut verse.label {
                *num = None;
            }

            verse
                .paragraphs
                .iter_mut()
                .map(|p| p.iter_mut())
                .flatten()
                .for_each(Inline::remove_chorus_num);
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Heading {
    /// There will actually only be headings of level 2 and more,
    /// because level 1 heading always starts a new song
    pub level: u32,
}

/// Like `()` but doesn't implement `Serialize` so that
/// we can serialize `Inlines<Void>` as just a sequence.
#[derive(Debug)]
pub struct Void;

/// Generic container for inlines.
/// Allows to add arbitrary serializable data to an array of inlines.
#[derive(Serialize, Debug)]
pub struct Inlines<T = ()> {
    #[serde(flatten)]
    pub data: T,
    pub inlines: Box<[Inline]>,
}

impl Inlines<()> {
    pub fn new(inlines: Box<[Inline]>) -> Self {
        Self { data: (), inlines }
    }
}

impl<T: Serialize> Inlines<T> {
    pub fn with(data: T, inlines: Box<[Inline]>) -> Self {
        Self { data, inlines }
    }

    fn remove_chorus_num(&mut self) {
        self.inlines.iter_mut().for_each(Inline::remove_chorus_num);
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum Inline {
    #[serde(rename = "i-text")]
    Text { text: BStr },
    #[serde(rename = "i-chord")]
    Chord(Inlines<Chord>),
    /// In bard all line breaks are considered hard breaks
    #[serde(rename = "i-break")]
    Break,
    #[serde(rename = "i-emph")]
    Emph(Inlines),
    #[serde(rename = "i-strong")]
    Strong(Inlines),
    #[serde(rename = "i-link")]
    Link(Link),
    #[serde(rename = "i-image")]
    Image(Image),
    #[serde(rename = "i-chorus-ref")]
    ChorusRef(ChorusRef),

    /// Only used internally by the parser to apply transposition
    #[serde(rename = "i-transpose")]
    Transpose(Transpose),
}

impl Inline {
    pub fn is_break(&self) -> bool {
        matches!(self, Self::Break)
    }

    pub fn is_xpose(&self) -> bool {
        matches!(self, Self::Transpose(..))
    }

    pub fn unwrap_xpose(&self) -> Transpose {
        match self {
            Self::Transpose(xpose) => *xpose,
            _ => panic!("Unexpected inline: {:?}", self),
        }
    }

    fn remove_chorus_num(&mut self) {
        match self {
            Inline::Chord(c) => c.remove_chorus_num(),
            Inline::Emph(e) => e.remove_chorus_num(),
            Inline::Strong(s) => s.remove_chorus_num(),
            Inline::ChorusRef(cr) => cr.num = None,
            _ => {}
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Chord {
    pub chord: BStr,
    pub alt_chord: Option<BStr>,
    #[serde(skip)]
    line: u32,
}

impl Chord {
    pub fn new(chord: BStr, alt_chord: Option<BStr>, line: u32) -> Self {
        Self {
            chord,
            alt_chord,
            line,
        }
    }
}


#[derive(Serialize, Debug)]
pub struct Link {
    pub url: BStr,
    pub title: BStr,
    pub text: BStr,
}

impl Link {
    pub fn new(url: BStr, title: BStr, text: BStr) -> Self {
        Self { url, title, text }
    }
}

#[derive(Serialize, Debug)]
pub struct Image {
    // TODO: if local file, add to watches for bard watch?
    pub path: BStr,
    pub title: BStr,
    pub class: BStr,
}

impl Image {
    pub fn new(path: BStr, title: BStr, class: BStr) -> Self {
        Self { path, title, class }
    }
}

#[derive(Serialize, Debug)]
pub struct ChorusRef {
    pub num: Option<u32>,
    pub prefix_space: BStr,
}

impl ChorusRef {
    pub fn new(num: Option<u32>, prefix_space: bool) -> Self {
        Self {
            num,
            prefix_space: if prefix_space { " ".into() } else { "".into() },
        }
    }
}

#[derive(Serialize, Clone, Copy, Debug)]
pub enum Transpose {
    #[serde(rename = "t-transpose")]
    Transpose(i32),
    #[serde(rename = "t-notation")]
    Notation(Notation),
    #[serde(rename = "t-alt-transpose")]
    AltTranspose(i32),
    #[serde(rename = "t-alt-notation")]
    AltNotation(Notation),
}

#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum VerseLabel {
    Verse(u32),
    Chorus(Option<u32>),
    Custom(BStr),
    None {},
}

impl VerseLabel {
    fn is_some(&self) -> bool {
        !matches!(self, Self::None {})
    }
}

pub type Paragraph = Box<[Inline]>;

#[derive(Serialize, Debug)]
pub struct Verse {
    pub label: VerseLabel,
    pub paragraphs: Vec<Paragraph>,
}

impl Verse {
    pub fn new(label: VerseLabel, paragraphs: Vec<Paragraph>) -> Self {
        Self { label, paragraphs }
    }

    pub fn is_empty(&self) -> bool {
        self.paragraphs.is_empty()
    }
}


#[derive(Serialize, Debug)]
pub struct BulletList {
    pub items: Box<[BStr]>,
}

#[derive(Serialize, Debug)]
pub struct Song {
    pub title: BStr,
    pub subtitles: Box<[BStr]>,
    pub blocks: Vec<Block>,
    pub notation: Notation,
}

impl Song {
    /// AST postprocessing.
    /// At the moment this entails removing empty paragraphs and verses
    /// which linger when transposition extensions are applied & removed.
    pub fn postprocess(&mut self) {
        // Remove paragraphs which contain nothing or linebreaks only
        for block in self.blocks.iter_mut() {
            if let Block::Verse(verse) = block {
                verse
                    .paragraphs
                    .retain(|para| para.iter().any(|inline| !inline.is_break()))
            }
        }

        // Remove verses which have no paragraphs and no label
        self.blocks.retain(|block| match block {
            Block::Verse(verse) => verse.label.is_some() || !verse.paragraphs.is_empty(),
            _ => true,
        });
    }
}

#[derive(Debug)]
pub struct Book {
    pub songs: Vec<Song>,
    pub notation: Notation,
    pub chorus_label: BStr,
}

impl Book {
    pub fn new(notation: Notation, chorus_label: &str) -> Book {
        Book {
            songs: vec![],
            notation,
            chorus_label: chorus_label.into(),
        }
    }

    fn add_md<'s>(&mut self, input: &'s str, path: &Path) -> Result<()> {
        let mut parser = Parser::new(input, self.notation, "[Untitled]");
        parser
            .parse(&mut self.songs)
            .with_context(|| format!("Could not parse file `{}`", path.display()))?;

        Ok(())
    }

    pub fn add_md_str(&mut self, source: &str) -> Result<()> {
        static STR_PATH: &'static str = "<buffer>";
        self.add_md(source, &Path::new(&STR_PATH))
    }

    pub fn add_md_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        let source = fs::read_to_string(&path)?;

        self.add_md(&source, path)
    }

    pub fn load_files<P: AsRef<Path>>(&mut self, input_paths: &[P]) -> Result<()> {
        for path in input_paths.iter() {
            let path = path.as_ref();
            self.add_md_file(&path)?;
        }

        self.songs.shrink_to_fit();

        Ok(())
    }
}

#[cfg(test)]
pub trait AssertJsonEq {
    fn assert_eq(&self, value: serde_json::Value);
}

#[cfg(test)]
impl<T> AssertJsonEq for T
where
    T: Serialize,
{
    #[track_caller]
    fn assert_eq(&self, value: serde_json::Value) {
        use assert_json_diff::{assert_json_matches_no_panic, Config, CompareMode};

        let config = Config::new(CompareMode::Strict);
        if let Err(diff) = assert_json_matches_no_panic(self, &value, config) {
            panic!(
                "JSON equality assertion failed: \n== LHS ==\n{}\n== RHS ==\n{}\n== DIFF \
                 ==\n{}\n\n",
                serde_json::to_string_pretty(self).unwrap(),
                serde_json::to_string_pretty(&value).unwrap(),
                diff
            )
        }
    }
}
