//! AST of a bard songbook

use std::collections::BTreeMap;
use std::fs;
use std::str;

use camino::Utf8Path as Path;
use serde::Serialize;

use crate::error::*;
use crate::music::Notation;
use crate::parser::{Parser, ParserConfig};
use crate::project::Settings;
use crate::util::{sort_lexical_by, BStr};

pub mod version;

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
    /// An HTML block contains inlines which can only be `Text`, `HtmlTag`, or `Break`.
    #[serde(rename = "b-html-block")]
    HtmlBlock(Inlines),
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

    /// Used to remove chorus numbers in case there's one chorus.
    pub fn remove_chorus_num(&mut self) {
        if let Self::Verse(verse) = self {
            if let VerseLabel::Chorus(num) = &mut verse.label {
                *num = None;
            }

            verse
                .paragraphs
                .iter_mut()
                .flat_map(|p| p.iter_mut())
                .for_each(Inline::remove_chorus_num);
        }
    }
}

/// Needed for Inline enum tagging in JSON and similar...
#[derive(Serialize, Debug)]
pub struct Inlines {
    pub inlines: Box<[Inline]>,
}

impl Inlines {
    pub fn new(inlines: Box<[Inline]>) -> Self {
        Self { inlines }
    }

    fn remove_chorus_num(&mut self) {
        self.inlines.iter_mut().for_each(Inline::remove_chorus_num);
    }
}

impl From<Vec<Inline>> for Inlines {
    fn from(inlines: Vec<Inline>) -> Self {
        Self {
            inlines: inlines.into(),
        }
    }
}

impl AsRef<[Inline]> for Inlines {
    fn as_ref(&self) -> &[Inline] {
        self.inlines.as_ref()
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum Inline {
    #[serde(rename = "i-text")]
    Text { text: BStr },
    #[serde(rename = "i-chord")]
    Chord(Chord),
    /// All line breaks are considered hard breaks
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
    #[serde(rename = "i-tag")]
    HtmlTag(HtmlTag),

    /// Only used internally by the parser to apply transposition.
    /// Removed from the resulting AST, except in tests where this
    /// is used to verify transposition extensions parsing.
    #[serde(rename = "i-transpose")]
    Transpose(Transpose),
}

impl Inline {
    pub fn text(text: impl Into<BStr>) -> Self {
        Self::Text { text: text.into() }
    }

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
    pub backticks: usize,
    pub baseline: bool,
    pub inlines: Box<[Inline]>,
}

impl Chord {
    pub fn new(
        chord: BStr,
        alt_chord: Option<BStr>,
        backticks: usize,
        baseline: bool,
        inlines: Vec<Inline>,
    ) -> Self {
        Self {
            chord,
            alt_chord,
            backticks,
            baseline,
            inlines: inlines.into(),
        }
    }

    fn remove_chorus_num(&mut self) {
        self.inlines.iter_mut().for_each(Inline::remove_chorus_num);
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

#[derive(Serialize, Debug)]
pub struct HtmlTag {
    pub name: BStr,
    pub attrs: BTreeMap<BStr, BStr>,
}

/// Transposition extensions. See Comment in `Inline`.
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
    /// Turn off alt chords
    #[serde(rename = "t-alt-none")]
    AltNone,
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

#[derive(Serialize, Debug)]
pub struct SongRef {
    pub title: BStr,
    /// index of the song in the Book::songs vector
    pub idx: usize,
}

impl SongRef {
    pub fn new((idx, songs): (usize, &Song)) -> Self {
        Self {
            title: songs.title.clone(),
            idx,
        }
    }
}

#[derive(Debug)]
pub struct Book {
    pub songs: Vec<Song>,
    pub songs_sorted: Vec<SongRef>,
    pub notation: Notation,
}

impl Book {
    pub fn new(settings: &Settings) -> Book {
        Book {
            songs: vec![],
            songs_sorted: vec![],
            notation: settings.notation,
        }
    }

    fn add_md(&mut self, input: &str, path: &Path) -> Result<()> {
        let config = ParserConfig::new(self.notation);
        let mut parser = Parser::new(input, path, config);
        parser
            .parse(&mut self.songs)
            .with_context(|| format!("Could not parse file `{}`", path))?;

        Ok(())
    }

    pub fn add_md_str(&mut self, source: &str) -> Result<()> {
        static STR_PATH: &str = "<buffer>";
        self.add_md(source, Path::new(&STR_PATH))
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

    /// Book-level postprocessing.
    /// Currently this is generation of the songs_sorted vec.
    /// This is unrelated to song postprocessing.
    pub fn postprocess(&mut self) {
        self.songs_sorted = self.songs.iter().enumerate().map(SongRef::new).collect();
        sort_lexical_by(&mut self.songs_sorted, |songref| songref.title.as_ref());
    }
}

#[cfg(test)]
pub trait AssertJsonEq {
    fn assert_json_eq(&self, value: serde_json::Value);
}

#[cfg(test)]
impl<T> AssertJsonEq for T
where
    T: Serialize,
{
    #[track_caller]
    fn assert_json_eq(&self, value: serde_json::Value) {
        use assert_json_diff::{assert_json_matches_no_panic, CompareMode, Config};

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
