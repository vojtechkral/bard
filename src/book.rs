//! AST of a bard songbook

use std::collections::BTreeMap;

use image::image_dimensions;
use serde::Serialize;

use crate::music::Notation;
use crate::prelude::*;
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

    fn verse(&self) -> Option<&Verse> {
        match self {
            Self::Verse(verse) => Some(verse),
            _ => None,
        }
    }

    fn verse_mut(&mut self) -> Option<&mut Verse> {
        match self {
            Self::Verse(verse) => Some(verse),
            _ => None,
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

    fn image(&self) -> Option<&Image> {
        match self {
            Self::Image(image) => Some(image),
            _ => None,
        }
    }

    fn image_mut(&mut self) -> Option<&mut Image> {
        match self {
            Self::Image(image) => Some(image),
            _ => None,
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
    /// Size in pixels, initially `0`, resolved during book postprocessing.
    pub width: u32,
    /// Size in pixels, initially `0`, resolved during book postprocessing.
    pub height: u32,

    /// Absolute path to the image file, resolved during book postprocessing,
    /// **not** part of AST.
    #[serde(skip)]
    pub full_path: Option<PathBuf>,
}

impl Image {
    pub fn new(path: BStr, title: BStr, class: BStr) -> Self {
        Self {
            path,
            title,
            class,
            width: 0,
            height: 0,
            full_path: None,
        }
    }

    fn resolve(&mut self, output_dir: &Path) -> Result<()> {
        let path = Path::new(&*self.path);
        if self.path.contains("://") || path.is_absolute() {
            bail!("Image path has to be relative and pointing to a local file.");
        }

        let full_path = output_dir.join(path);
        let (w, h) = image_dimensions(&full_path)
            .with_context(|| format!("Couldn't read image file {:?}", full_path))?;

        self.width = w;
        self.height = h;
        self.full_path = Some(full_path);

        Ok(())
    }

    pub fn full_path(&self) -> &Path {
        self.full_path.as_deref().unwrap()
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

    fn inlines(&self) -> impl Iterator<Item = &Inline> {
        self.paragraphs.iter().flat_map(|p| p.iter())
    }

    fn inlines_mut(&mut self) -> impl Iterator<Item = &mut Inline> {
        self.paragraphs.iter_mut().flat_map(|p| p.iter_mut())
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
    ///
    /// This entails removing empty paragraphs and verses
    /// which linger when transposition extensions are applied & removed.
    ///
    /// Distinct from `Book::postprocess()`, this is done by `Parser`.
    pub fn postprocess(&mut self) {
        // Remove paragraphs which contain nothing or linebreaks only
        for verse in self.blocks.iter_mut().filter_map(Block::verse_mut) {
            verse
                .paragraphs
                .retain(|para| para.iter().any(|inline| !inline.is_break()));
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

    pub fn add_songs(&mut self, mut songs: Vec<Song>) {
        self.songs.reserve(songs.len());
        self.songs.append(&mut songs);
    }

    /// Book-level postprocessing.
    ///
    /// Steps taken:
    /// 1. Generation of the songs_sorted vec,
    /// 2. Resolving of image elements (checking path, reading image dimensions).
    pub fn postprocess(&mut self, output_dir: &Path) -> Result<()> {
        self.songs.shrink_to_fit();
        self.songs_sorted = self.songs.iter().enumerate().map(SongRef::new).collect();
        sort_lexical_by(&mut self.songs_sorted, |songref| songref.title.as_ref());

        for image in self.iter_images_mut() {
            image.resolve(output_dir)?;
        }

        Ok(())
    }

    pub fn iter_images(&self) -> impl Iterator<Item = &Image> {
        self.songs
            .iter()
            .flat_map(|s| s.blocks.iter())
            .filter_map(Block::verse)
            .flat_map(|v| v.inlines())
            .filter_map(Inline::image)
    }

    pub fn iter_images_mut(&mut self) -> impl Iterator<Item = &mut Image> {
        self.songs
            .iter_mut()
            .flat_map(|s| s.blocks.iter_mut())
            .filter_map(Block::verse_mut)
            .flat_map(|v| v.inlines_mut())
            .filter_map(Inline::image_mut)
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
