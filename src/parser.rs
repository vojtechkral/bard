//! The bard Markdown parser module.
//!
//! Here the bard's Markdown subset is parsed using `comrak`, `tl`,
//! and code for parsing of `!` extensions.
//!
//! The API is provided by the `Parser` type, it's `parse()` method is the entry point.

use std::mem;
use std::str;

use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use comrak::nodes::{AstNode, ListType, NodeCode, NodeValue};
use comrak::{ComrakExtensionOptions, ComrakOptions, ComrakParseOptions, ComrakRenderOptions};
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use thiserror::Error;

use crate::book::*;
use crate::music::{self, Notation};
use crate::util::{BStr, ByteSliceExt};

mod html;

type AstRef<'a> = &'a AstNode<'a>;
type Arena<'a> = comrak::Arena<AstNode<'a>>;

const FALLBACK_TITLE: &str = "[Untitled]";

lazy_static! {
    static ref EXTENSION: Regex = Regex::new(r"(^|\s)(!+)(\S+)").unwrap();
}

#[derive(Error, PartialEq, Eq, Clone, Debug)]
enum ErrorKind {
    #[error("Control character not allowed: 0x{char:x}")]
    ControlChar { char: u32 },
    #[error("Unrecognized chord: {chord}")]
    Transposition { chord: BStr },
}

/// Parser error type.
///
/// Reports filename, line number and kind of error occured.
/// The line number is 1-indexed.
#[derive(Error, PartialEq, Eq, Clone, Debug)]
#[error("{file}:{line}: {kind}")]
pub struct Error {
    file: PathBuf,
    line: u32,
    kind: ErrorKind,
}

impl Error {
    pub fn control_char(file: &Path, line: u32, char: u32) -> Self {
        Self {
            file: file.to_owned(),
            line,
            kind: ErrorKind::ControlChar { char },
        }
    }

    pub fn transposition(file: &Path, mut node: AstRef<'_>, chord: BStr) -> Self {
        // Comrak actually doesn't set the start_line for some elements (code),
        // so we try to find the line number by looking at parent nodes.
        // FIXME: This is a hack, report or fix at comrak.
        let mut line = node.data.borrow().start_line;
        while line == 0 {
            node = match node.parent() {
                Some(n) => n,
                None => break,
            };
            line = node.data.borrow().start_line;
        }

        Self {
            file: file.into(),
            line: line + 1, // make the line number 1-indexed
            kind: ErrorKind::Transposition { chord },
        }
    }
}

type Result<T, E = Error> = std::result::Result<T, E>;

// Since parser takes an UTF-8 string as input, we don't have to error-check
// when converting bytes to strings.
fn utf8<'a>(bytes: &'a [u8]) -> &'a str {
    str::from_utf8(bytes).unwrap()
}

/// Parser for a candidate bard MD extension
#[derive(Debug)]
struct Extension {
    num_excls: u32,
    content: String,
    /// `true` if there was a space char in front of the ext,
    /// used to preserve proper spacing when chorus refs are mixed in text.
    prefix_space: bool,
}

impl<'a> From<Captures<'a>> for Extension {
    fn from(caps: Captures<'a>) -> Self {
        let prefix_space = caps.get(1).unwrap().as_str().chars().next().is_some();
        let num_excls = caps.get(2).unwrap().as_str().len() as _;
        let content = caps.get(3).unwrap().as_str().to_owned();
        Self {
            num_excls,
            content,
            prefix_space,
        }
    }
}

impl Extension {
    fn try_parse_xpose(&self) -> Option<Transpose> {
        if self.content.starts_with(&['+', '-'][..]) {
            if let Ok(delta) = self.content.parse::<i32>() {
                match self.num_excls {
                    1 => return Some(Transpose::Transpose(delta)),
                    2 => return Some(Transpose::AltTranspose(delta)),
                    _ => {}
                }
            }
        }

        if let Ok(notation) = self.content.parse::<Notation>() {
            match self.num_excls {
                1 => return Some(Transpose::Notation(notation)),
                2 => return Some(Transpose::AltNotation(notation)),
                _ => {}
            }
        }

        None
    }

    fn try_parse_chorus_ref(&self) -> Option<ChorusRef> {
        if self.num_excls == 1 && self.content.chars().all(|c| c == '>') {
            let num = self.content.len() as _;
            Some(ChorusRef::new(Some(num), self.prefix_space))
        } else {
            None
        }
    }

    fn try_parse(&self) -> Option<Inline> {
        if let Some(xpose) = self.try_parse_xpose() {
            // Transposition extension recognized
            Some(Inline::Transpose(xpose))
        } else {
            // Try parsing chorus reference,
            // push as regular text if not recognized
            self.try_parse_chorus_ref().map(Inline::ChorusRef)
        }
    }
}

/// Parser transposition state
#[derive(Clone, Default, Debug)]
pub struct Transposition {
    /// Source notation of the song
    src_notation: Notation,
    /// Transposition of chords
    xpose: Option<i32>,
    /// Notation conversion of chords
    notation: Option<Notation>,
    /// Transposition of alt chords (2nd row)
    alt_xpose: Option<i32>,
    /// Notation conversion of alt chords (2nd row)
    alt_notation: Option<Notation>,

    /// Option to disable transposition for unit testing,
    /// ie. leave `Inline::Transpose` in the AST so they can be checked.
    disabled: bool,
}

impl Transposition {
    fn new(src_notation: Notation, disabled: bool) -> Self {
        Self {
            src_notation,
            disabled,
            ..Default::default()
        }
    }

    fn update(&mut self, xpose: Transpose) {
        if self.disabled {
            return;
        }

        match xpose {
            Transpose::Transpose(d) => self.xpose = Some(d),
            Transpose::Notation(nt) => self.notation = Some(nt),
            Transpose::AltTranspose(d) => self.alt_xpose = Some(d),
            Transpose::AltNotation(nt) => self.alt_notation = Some(nt),
        }
    }

    fn is_some(&self) -> bool {
        self.xpose.is_some()
            || self.notation.is_some()
            || self.alt_xpose.is_some()
            || self.alt_notation.is_some()
    }
}

/// Custom operations on Comrak AST nodes
trait NodeExt<'a> {
    fn is_block(&self) -> bool;
    fn is_text(&self) -> bool;
    fn is_h(&self, level: u32) -> bool;
    fn is_p(&self) -> bool;
    fn is_code(&self) -> bool;
    fn is_break(&self) -> bool;
    fn is_link(&self) -> bool;
    fn is_item(&self) -> bool;
    fn is_bq(&self) -> bool;
    fn is_img(&self) -> bool;
    fn is_inline_html(&self) -> bool;

    /// Elements that shouldn't go into chord child inlines,
    /// ie. line break or and image
    fn ends_chord(&self) -> bool;

    /// Recursively concatenate all text fields, ie. remove
    /// formatting and just return the text.
    fn as_plaintext(&'a self) -> String;

    /// Split the current node at the specified child index.
    /// This effectively: 1. duplicates the current node, the copy is
    /// added as originals next sibling, 2. moves children starting from index
    /// `at_child` (inclusive) from the original to the copy.
    fn split_at(&'a self, at_child: usize, arena: &'a Arena<'a>) -> AstRef<'a>;

    /// Preprocesses the AST, performing the following operations:
    /// - Link child nodes are converted to plaintext
    /// - Inline `Code`, `LineBreak`s, `SoftBreak`s and `Image`s 'bubble up' to the top level
    ///   of the current block element. That is, if a `Code` inline is nested
    ///   within another inline element, the element is split and the code is brought up.
    ///   This happens recursively. This is done so that inline code spans can be easily
    ///   collected into Chord spans with the content that follows until the next inline code
    ///   or linebreak.
    fn preprocess(&'a self, arena: &'a Arena<'a>);

    /// Parse the html snippet using a 3rd party HTML parser,
    /// convert HTML elements into `Inline::HtmlTag`s,
    /// interleaved plain text to `Inline::Text`s and append to `target`.
    fn parse_html(&self, target: &mut Vec<Inline>);
}

impl<'a> NodeExt<'a> for AstNode<'a> {
    #[inline]
    fn is_block(&self) -> bool {
        self.data.borrow().value.block()
    }

    #[inline]
    fn is_text(&self) -> bool {
        self.data.borrow().value.text().is_some()
    }

    #[inline]
    fn is_h(&self, level: u32) -> bool {
        matches!(self.data.borrow().value,
            NodeValue::Heading(h) if h.level == level
        )
    }

    #[inline]
    fn is_p(&self) -> bool {
        matches!(self.data.borrow().value, NodeValue::Paragraph)
    }

    #[inline]
    fn is_code(&self) -> bool {
        matches!(self.data.borrow().value, NodeValue::Code(..))
    }

    #[inline]
    fn is_break(&self) -> bool {
        matches!(
            self.data.borrow().value,
            NodeValue::LineBreak | NodeValue::SoftBreak
        )
    }

    #[inline]
    fn is_link(&self) -> bool {
        matches!(self.data.borrow().value, NodeValue::Link(..))
    }

    #[inline]
    fn is_item(&self) -> bool {
        matches!(self.data.borrow().value, NodeValue::Item(..))
    }

    #[inline]
    fn is_bq(&self) -> bool {
        matches!(self.data.borrow().value, NodeValue::BlockQuote)
    }

    #[inline]
    fn is_img(&self) -> bool {
        matches!(self.data.borrow().value, NodeValue::Image(..))
    }

    #[inline]
    fn is_inline_html(&self) -> bool {
        matches!(self.data.borrow().value, NodeValue::HtmlInline(..))
    }

    #[inline]
    fn ends_chord(&self) -> bool {
        self.is_break() || self.is_img() || self.is_inline_html()
    }

    fn as_plaintext(&'a self) -> String {
        fn recurse<'a>(this: &'a AstNode<'a>, res: &mut String) {
            let value = this.data.borrow();
            let text_b = match &value.value {
                NodeValue::Text(literal) | NodeValue::Code(NodeCode { literal, .. }) => {
                    Some(literal)
                }
                _ => None,
            };

            if let Some(bytes) = text_b {
                res.push_str(utf8(&bytes[..]));
            } else {
                for c in this.children() {
                    recurse(c, res);
                }
            }
        }

        let mut res = String::new();
        recurse(self, &mut res);
        res
    }

    fn split_at(&'a self, at_child: usize, arena: &'a Arena<'a>) -> AstRef<'a> {
        // Clone the data and alloc a new node in the arena:
        let data2 = self.data.clone();
        let node2 = arena.alloc(AstNode::new(data2));

        // Append as the next sibling
        self.insert_after(node2);

        // Move [i, len) children to node2
        for child in self.children().skip(at_child) {
            node2.append(child); // Yes, this will detach the child from self
        }

        node2
    }

    fn preprocess(&'a self, arena: &'a Arena<'a>) {
        // First make sure children are already preprocessed
        // (We're doing a DFS descent basically.)
        self.children().for_each(|c| c.preprocess(arena));

        // The preprocessing is only applicable to inlines
        if self.is_block() {
            return;
        }

        if self.is_link() {
            if self.children().count() == 1
                && self.children().next().map_or(false, NodeExt::is_text)
            {
                // This is a plaintext link, nothing needs to be done
            } else {
                // Convert link to plaintext
                let plain = self.as_plaintext().into_bytes();
                for c in self.children() {
                    c.detach();
                }
                let textnode = arena.alloc(AstNode::from(NodeValue::Text(plain)));
                self.append(textnode);
            }

            return;
        }

        let mut start_node = Some(self);
        while let Some(node) = start_node.take() {
            if let Some((i, child)) = node
                .children()
                .enumerate()
                .find(|(_, c)| c.is_code() || c.is_break() || c.is_img() || c.is_inline_html())
            {
                // We want to take this child and append as a sibling to self,
                // but first self needs to be duplicated with the already-processed nodes
                // removed. The processing then should go on to the duplicated node...
                child.detach();
                let node2 = node.split_at(i, arena);
                node.insert_after(child);
                start_node = Some(node2);
            }
        }
    }

    fn parse_html(&self, target: &mut Vec<Inline>) {
        let this = self.data.borrow();
        let html = match &this.value {
            NodeValue::HtmlBlock(b) => b.literal.as_slice(),
            NodeValue::HtmlInline(b) => b.as_slice(),

            _ => panic!("HTML can only be parsed from HTML nodes."),
        };

        html::parse_html(html, target);
    }
}

#[derive(Debug)]
struct ChordBuilder {
    chord: BStr,
    alt_chord: Option<BStr>,
    backticks: usize,
    inlines: Vec<Inline>,
}

impl ChordBuilder {
    fn new(code: &NodeCode) -> Self {
        Self {
            chord: code.literal.as_bstr(),
            alt_chord: None,
            backticks: code.num_backticks,
            inlines: vec![],
        }
    }

    fn inlines_mut(&mut self) -> &mut Vec<Inline> {
        &mut self.inlines
    }

    fn transpose(&mut self, xp: &Transposition) -> Result<(), BStr> {
        if xp.disabled {
            return Ok(());
        }

        let src_nt = xp.src_notation;
        let chord = music::Chord::parse(&self.chord, src_nt).ok_or_else(|| self.chord.clone())?;

        if xp.xpose.is_some() || xp.notation.is_some() {
            let delta = xp.xpose.unwrap_or(0);
            let notation = xp.notation.unwrap_or(src_nt);
            self.chord = chord.transposed(delta).as_string(notation).into();
        }

        if xp.alt_xpose.is_some() || xp.alt_notation.is_some() {
            let delta = xp.alt_xpose.unwrap_or(0);
            let notation = xp.alt_notation.unwrap_or(src_nt);
            self.alt_chord = Some(chord.transposed(delta).as_string(notation).into());
        }

        Ok(())
    }

    fn finalize(self, inlines: &mut Vec<Inline>) {
        let chord = Chord::new(self.chord, self.alt_chord, self.backticks, self.inlines);
        inlines.push(Inline::Chord(chord));
    }
}

#[derive(Debug)]
struct VerseBuilder<'a> {
    label: VerseLabel,
    paragraphs: Vec<Paragraph>,
    xp: Transposition,
    src_file: &'a Path,
}

impl<'a> VerseBuilder<'a> {
    fn new(label: VerseLabel, xp: Transposition, src_file: &'a Path) -> Self {
        Self {
            label,
            paragraphs: vec![],
            xp,
            src_file,
        }
    }

    fn with_p_nodes<I>(
        label: VerseLabel,
        xp: Transposition,
        src_file: &'a Path,
        mut nodes: I,
    ) -> Result<Self>
    where
        I: Iterator<Item = AstRef<'a>>,
    {
        nodes.try_fold(Self::new(label, xp, src_file), |mut this, node| {
            this.add_p_node(node)?;
            Ok(this)
        })
    }

    /// Parse a text node. It may parse into a series of `Inline`s
    /// since extension parsing is handled here.
    fn parse_text(&mut self, node: AstRef, target: &mut Vec<Inline>) {
        let data = node.data.borrow();
        let text = match &data.value {
            NodeValue::Text(text) => utf8(text),
            other => unreachable!("Unexpected element: {:?}", other),
        };

        let mut pos = 0;
        for caps in EXTENSION.captures_iter(&*text) {
            let hit = caps.get(0).unwrap();

            // Try parsing an extension
            let ext = Extension::from(caps);
            if let Some(inline) = ext.try_parse() {
                // First see if there's regular text preceding the extension
                let preceding = &text[pos..hit.start()];
                if !preceding.is_empty() {
                    target.push(Inline::text(preceding));
                }

                if inline.is_xpose() && !self.xp.disabled {
                    // Update transposition state and throw the inline away,
                    // we're normally not keeping them in the AST
                    self.xp.update(inline.unwrap_xpose());

                    // If the extension is first on the line (ie. no leading ws)
                    // then we should consume the following whitespace char
                    // (there must be either whitespace or EOL).
                    if !ext.prefix_space && hit.end() < text.len() {
                        pos = hit.end() + 1;
                    } else {
                        pos = hit.end();
                    }
                } else {
                    // inline not xpose or xp disabled
                    target.push(inline);
                    pos = hit.end();
                }
            }
        }

        // Also add text past the last extension (if any)
        let rest = &text[pos..];
        if !rest.is_empty() {
            target.push(Inline::text(rest));
        }
    }

    fn collect_inlines(&mut self, node: AstRef) -> Result<Vec<Inline>> {
        node.children().try_fold(vec![], |mut vec, node| {
            self.make_inlines(node, &mut vec)?;
            Ok(vec)
        })
    }

    /// Generate `Inline`s out of this inline node.
    /// Also recursively applies to children when applicable.
    fn make_inlines(&mut self, node: AstRef, target: &mut Vec<Inline>) -> Result<()> {
        assert!(!node.is_block());

        let single = match &node.data.borrow().value {
            NodeValue::Text(..) => {
                self.parse_text(node, target);
                return Ok(());
            }
            NodeValue::SoftBreak | NodeValue::LineBreak => Inline::Break,
            NodeValue::HtmlInline(..) => {
                node.parse_html(target);
                return Ok(());
            }
            NodeValue::Emph => Inline::Emph(self.collect_inlines(node)?.into()),
            NodeValue::Strong => Inline::Strong(self.collect_inlines(node)?.into()),
            NodeValue::Link(link) => {
                let mut children = node.children();
                let text = children.next().unwrap();
                assert!(children.next().is_none());
                assert!(text.is_text());
                let text = text.as_plaintext().into();

                let link = Link::new(link.url.as_bstr(), link.title.as_bstr(), text);
                Inline::Link(link)
            }
            NodeValue::Image(link) => {
                let img = Image::new(
                    link.url.as_bstr(),
                    node.as_plaintext().into(),
                    link.title.as_bstr(),
                );
                Inline::Image(img)
            }
            NodeValue::FootnoteReference(..) => return Ok(()),

            // TODO: Ensure extensions are not enabled through a test
            other => {
                unreachable!("Unexpected element: {:?}", other);
            }
        };

        target.push(single);
        Ok(())
    }

    fn add_p_inner(&mut self, node: AstRef) -> Result<()> {
        assert!(node.is_p());

        let mut para: Vec<Inline> = vec![];
        let mut cb = None::<ChordBuilder>;
        for c in node.children() {
            let c_data = c.data.borrow();
            if let NodeValue::Code(code) = &c_data.value {
                if let Some(cb) = cb.take() {
                    cb.finalize(&mut para);
                }

                let mut new_cb = ChordBuilder::new(code);
                if self.xp.is_some() {
                    new_cb
                        .transpose(&self.xp)
                        .map_err(|chord| Error::transposition(self.src_file, c, chord))?;
                }
                cb = Some(new_cb);
            } else if c.ends_chord() {
                if let Some(cb) = cb.take() {
                    cb.finalize(&mut para);
                }

                self.make_inlines(c, &mut para)?;
            } else {
                // c must be another inline element.
                // See if a chord is currently open
                if let Some(cb) = cb.as_mut() {
                    // Add the inlines to the current chord
                    self.make_inlines(c, cb.inlines_mut())?;
                } else {
                    // Otherwise just push as a standalone inline
                    self.make_inlines(c, &mut para)?;
                }
            }
        }

        if let Some(cb) = cb.take() {
            cb.finalize(&mut para);
        }

        if !para.is_empty() {
            self.paragraphs.push(para.into());
        }

        Ok(())
    }

    /// Add node containing a paragraph (or multiple ones in case of nested lists)
    fn add_p_node(&mut self, node: AstRef) -> Result<()> {
        // This is called from SongBuilder, ie. if we come across a List
        // or a BlockQuote here, that means it must be a nested one,
        // as top-level ones are handled in SongBuilder.
        // These nested lists/bqs are undefined by bard MD,
        // ATM we just ignore them as such, but parse the paragraphs within.
        match &node.data.borrow().value {
            NodeValue::Paragraph => self.add_p_inner(node),
            NodeValue::BlockQuote | NodeValue::List(..) | NodeValue::Item(..) => {
                node.children().try_for_each(|c| self.add_p_node(c))
            }

            NodeValue::HtmlBlock(..) => {
                let mut inlines = vec![];
                node.parse_html(&mut inlines);
                if !inlines.is_empty() {
                    self.paragraphs.push(inlines.into());
                }
                Ok(())
            }

            _ => Ok(()), // ignored
        }
    }

    fn finalize(self) -> (Verse, Transposition) {
        let verse = Verse::new(self.label, self.paragraphs);
        (verse, self.xp)
    }
}

#[derive(Debug)]
struct SongBuilder<'a> {
    nodes: &'a [AstRef<'a>],
    title: String,
    subtitles: Vec<BStr>,
    verse: Option<VerseBuilder<'a>>,
    blocks: Vec<Block>,
    xp: Transposition,
    verse_num: u32,
    src_file: &'a Path,
}

impl<'a> SongBuilder<'a> {
    fn new(nodes: &'a [AstRef<'a>], config: &ParserConfig, src_file: &'a Path) -> Self {
        // Read song title or use fallback
        let (title, nodes) = match nodes.first() {
            Some(n) if n.is_h(1) => (n.as_plaintext(), &nodes[1..]),
            _ => (config.fallback_title.clone(), nodes),
        };

        // Collect subtitles - H2s following the title (if any)
        let subtitles: Vec<_> = nodes
            .iter()
            .take_while(|node| node.is_h(2))
            .map(|node| node.as_plaintext().into())
            .collect();

        // Shift nodes to the song content
        let nodes = &nodes[subtitles.len()..];

        Self {
            nodes,
            title,
            subtitles,
            verse: None,
            blocks: vec![],
            xp: Transposition::new(config.notation, config.xp_disabled),
            verse_num: 0,
            src_file,
        }
    }

    fn next_verse_num(&mut self) -> u32 {
        self.verse_num += 1;
        self.verse_num
    }

    fn verse_mut(&mut self) -> &mut VerseBuilder<'a> {
        if self.verse.is_none() {
            self.verse = Some(VerseBuilder::new(
                VerseLabel::None {},
                self.xp.clone(),
                self.src_file,
            ));
        }

        self.verse.as_mut().unwrap()
    }

    fn verse_finalize(&mut self) {
        if let Some(verse) = self.verse.take() {
            let (verse, xp) = verse.finalize();
            self.blocks.push(Block::Verse(verse));
            self.xp = xp;
        }
    }

    fn parse_bq(&mut self, bq: AstRef, level: u32) -> Result<()> {
        assert!(bq.is_bq());

        let mut prev_bq = false;
        for c in bq.children() {
            if c.is_bq() {
                self.verse_finalize();
                self.parse_bq(c, level + 1)?;
                prev_bq = true;
            } else {
                if prev_bq {
                    self.verse_finalize();
                    prev_bq = false;
                }

                if self.verse.is_none() {
                    let label = VerseLabel::Chorus(Some(level));
                    let verse = VerseBuilder::new(label, self.xp.clone(), self.src_file);
                    self.verse = Some(verse);
                }

                self.verse_mut().add_p_node(c)?;
            }
        }

        Ok(())
    }

    fn parse(&mut self) -> Result<()> {
        for node in self.nodes.iter() {
            if !node.is_p() {
                self.verse_finalize();
            }

            match &node.data.borrow().value {
                NodeValue::Paragraph => self.verse_mut().add_p_node(node)?,

                NodeValue::List(list) if matches!(list.list_type, ListType::Ordered) => {
                    for item in node.children() {
                        assert!(item.is_item());
                        self.verse_finalize();

                        let label = VerseLabel::Verse(self.next_verse_num());
                        let verse = VerseBuilder::with_p_nodes(
                            label,
                            self.xp.clone(),
                            self.src_file,
                            item.children(),
                        )?;
                        self.verse = Some(verse);
                    }
                }

                NodeValue::List(..) => {
                    let items: Vec<BStr> = node
                        .children()
                        .map(|item| item.as_plaintext().into())
                        .collect();
                    let list = BulletList {
                        items: items.into(),
                    };
                    self.blocks.push(Block::BulletList(list));
                }

                NodeValue::BlockQuote => self.parse_bq(node, 1)?,

                NodeValue::Heading(h) if h.level >= 3 => {
                    let label = VerseLabel::Custom(node.as_plaintext().into());
                    self.verse = Some(VerseBuilder::new(label, self.xp.clone(), self.src_file));
                }

                NodeValue::ThematicBreak => {
                    self.blocks.push(Block::HorizontalLine);
                }

                NodeValue::CodeBlock(cb) => self.blocks.push(Block::Pre {
                    text: cb.literal.as_bstr(),
                }),

                NodeValue::HtmlBlock(..) => {
                    let mut inlines = vec![];
                    node.parse_html(&mut inlines);
                    if !inlines.is_empty() {
                        self.blocks.push(Block::HtmlBlock(inlines.into()));
                    }
                }

                _ => {}
            }
        }

        Ok(())
    }

    fn finalize(mut self) -> Song {
        self.verse_finalize();

        // Chorus labels and chorus references carry a number
        // identifying the chorus. However, if there's just one chorus
        // in the song, we set the number to None, the number would be useless/distracting.
        let max_chorus = self
            .blocks
            .iter()
            .map(|b| b.chorus_num().unwrap_or(0))
            .max()
            .unwrap_or(0);
        if max_chorus < 2 {
            self.blocks.iter_mut().for_each(Block::remove_chorus_num);
        }

        let mut song = Song {
            title: self.title.into(),
            subtitles: self.subtitles.into(),
            blocks: self.blocks,
            notation: self.xp.src_notation,
        };

        song.postprocess();
        song
    }
}

struct SongsIter<'s, 'a> {
    slice: &'s [AstRef<'a>],
}

impl<'s, 'a> SongsIter<'s, 'a> {
    fn new(slice: &'s [AstRef<'a>]) -> Self {
        Self { slice }
    }

    fn find_next_h1(&self) -> Option<usize> {
        self.slice[1..]
            .iter()
            .enumerate()
            .find_map(|(i, node)| if node.is_h(1) { Some(i + 1) } else { None })
    }
}

impl<'s, 'a> Iterator for SongsIter<'s, 'a> {
    type Item = &'s [AstRef<'a>];

    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.is_empty() {
            return None;
        }

        if let Some(next_h1) = self.find_next_h1() {
            let (ret, next_slice) = self.slice.split_at(next_h1);
            self.slice = next_slice;
            Some(ret)
        } else {
            // Return the whole remaining slice
            Some(mem::take(&mut self.slice))
        }
    }
}

#[derive(Debug)]
pub struct ParserConfig {
    pub notation: Notation,
    pub fallback_title: String,
    pub xp_disabled: bool,
}

impl ParserConfig {
    pub fn new(notation: Notation) -> Self {
        Self {
            notation,
            fallback_title: FALLBACK_TITLE.into(),
            xp_disabled: false,
        }
    }
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            notation: Notation::default(),
            fallback_title: FALLBACK_TITLE.into(),
            xp_disabled: false,
        }
    }
}

#[derive(Debug)]
pub struct Parser<'i> {
    input: &'i str,
    src_file: &'i Path,
    config: ParserConfig,
}

impl<'i> Parser<'i> {
    pub fn new(input: &'i str, src_file: &'i Path, config: ParserConfig) -> Self {
        Self {
            input,
            src_file,
            config,
        }
    }

    #[cfg(test)]
    fn set_xp_disabled(&mut self, disabled: bool) {
        self.config.xp_disabled = disabled;
    }

    fn comrak_config() -> ComrakOptions {
        ComrakOptions {
            extension: ComrakExtensionOptions {
                strikethrough: false,
                tagfilter: false,
                table: false,
                autolink: false,
                tasklist: false,
                superscript: false,
                header_ids: None,
                footnotes: false,
                description_lists: false,
                front_matter_delimiter: None,
            },
            parse: ComrakParseOptions {
                smart: false,
                default_info_string: None,
            },
            render: ComrakRenderOptions {
                hardbreaks: false,
                github_pre_lang: false,
                width: 0,
                unsafe_: false,
                escape: false,
            },
        }
    }

    /// Verify input doesn't contain disallowed control chars,
    /// which are all of them except LF, TAB, and CR.
    fn check_control_chars(&self) -> Result<()> {
        for (num, line) in self.input.lines().enumerate() {
            for c in line.chars() {
                // The Lines iterator already takes care of \n and \r,
                // only need to check for \t here:
                if c.is_control() && c != '\t' {
                    return Err(Error::control_char(self.src_file, num as u32 + 1, c as u32));
                }
            }
        }

        Ok(())
    }

    /// Parsing is done in four steps:
    ///
    /// 1. Split the source AST in individual songs (they are separated by H1s),
    ///    this is done by `SongIter`.
    ///
    ///    For each song:
    ///
    /// 2. Preprocess the AST for easier parsing, this mainly involves bringing up
    ///    Code inlines and line breaks to the top level (out of arbitrary nested levels).
    ///    This is done by methods in `NodeExt`.
    ///
    /// 3. Parse the song content, this is done in `SongBuilder`, `VerseBuilder` et al.,
    ///    as well as helper methods in `NodeExt`.
    ///    Parsing bard MD extensions, incl. transposition, is also done here (`Extension`).
    ///    Transposition is applied right away.
    ///
    /// 4. Postprocess the song data and convert into the final `Song` AST,
    ///    as of now this is just removing of empty paragraphs/verses,
    ///    this is actually implemented on the `Song` AST type in `book`.
    ///
    /// The `Result` is one or more `Song` structures which are appended to the `songs` vec passed in.
    /// See the `book` module where the bard AST is defined.
    pub fn parse<'s>(&mut self, songs: &'s mut Vec<Song>) -> Result<&'s mut [Song]> {
        self.check_control_chars()?;

        let arena = Arena::new();
        let root = comrak::parse_document(&arena, self.input, &Self::comrak_config());
        let root_elems: Vec<_> = root.children().collect();

        let orig_len = songs.len();
        for song_nodes in SongsIter::new(&root_elems) {
            song_nodes.iter().for_each(|node| node.preprocess(&arena));

            let mut song = SongBuilder::new(song_nodes, &self.config, self.src_file);
            song.parse()?;
            songs.push(song.finalize());
        }

        Ok(&mut songs[orig_len..])
    }
}

#[cfg(test)]
mod tests;
