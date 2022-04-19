use std::mem;
use std::str;

use comrak::nodes::{AstNode, ListType, NodeCode, NodeValue};
use comrak::{ComrakExtensionOptions, ComrakOptions, ComrakParseOptions, ComrakRenderOptions};
use lazy_static::lazy_static;
use regex::{Captures, Regex};

use crate::book::*;
use crate::error::*;
use crate::music::{self, Notation};
use crate::util::{BStr, ByteSliceExt};

type AstRef<'a> = &'a AstNode<'a>;
type Arena<'a> = comrak::Arena<AstNode<'a>>;

const FALLBACK_TITLE: &str = "[Untitled]";

lazy_static! {
    static ref EXTENSION: Regex = Regex::new(r"(^|\s)(!+)(\S+)").unwrap();
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
    fn ends_chord(&self) -> bool {
        self.is_break() || self.is_img()
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
                let utf8 = String::from_utf8_lossy(&bytes[..]);
                res.push_str(&utf8);
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
                .find(|(_, c)| c.is_code() || c.is_break() || c.is_img())
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
}

#[derive(Debug)]
struct ChordBuilder {
    chord: BStr,
    alt_chord: Option<BStr>,
    backticks: usize,
    inlines: Vec<Inline>,
    line: u32,
}

impl ChordBuilder {
    fn new(code: &NodeCode, line: u32) -> Self {
        Self {
            chord: code.literal.as_bstr(),
            alt_chord: None,
            backticks: code.num_backticks,
            inlines: vec![],
            line,
        }
    }

    fn inlines_mut(&mut self) -> &mut Vec<Inline> {
        &mut self.inlines
    }

    fn transpose(&mut self, xp: &Transposition) -> Result<()> {
        if xp.disabled {
            return Ok(());
        }

        let src_nt = xp.src_notation;
        let chord = music::Chord::parse(&self.chord, src_nt)
            .with_context(|| format!("Unknown chord `{}` on line {}", self.chord, self.line))?;

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
        let chord = Chord::new(self.chord, self.alt_chord, self.backticks);
        let chord = Inline::Chord(Inlines {
            data: chord,
            inlines: self.inlines.into(),
        });
        inlines.push(chord);
    }
}

#[derive(Debug)]
struct VerseBuilder {
    label: VerseLabel,
    paragraphs: Vec<Paragraph>,
    xp: Transposition,
}

impl VerseBuilder {
    fn new(label: VerseLabel, xp: Transposition) -> Self {
        Self {
            label,
            paragraphs: vec![],
            xp,
        }
    }

    fn with_p_nodes<'n, 'a, I>(label: VerseLabel, xp: Transposition, mut nodes: I) -> Result<Self>
    where
        I: Iterator<Item = AstRef<'a>>,
    {
        nodes.try_fold(Self::new(label, xp), |mut this, node| {
            this.add_p_node(node)?;
            Ok(this)
        })
    }

    /// Parse a text node. It may parse into a series of `Inline`s
    /// since extension parsing is handled here.
    fn parse_text(&mut self, node: AstRef, target: &mut Vec<Inline>) {
        let data = node.data.borrow();
        let text = match &data.value {
            NodeValue::Text(text) => text,
            other => unreachable!("Unexpected element: {:?}", other),
        };

        let text = String::from_utf8_lossy(&*text);
        let mut pos = 0;

        for caps in EXTENSION.captures_iter(&*text) {
            let hit = caps.get(0).unwrap();

            // Try parsing an extension
            let ext = Extension::from(caps);
            if let Some(inline) = ext.try_parse() {
                // First see if there's regular text preceding the extension
                let preceding = &text[pos..hit.start()];
                if !preceding.is_empty() {
                    let preceding = Inline::Text {
                        text: preceding.into(),
                    };
                    target.push(preceding);
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
            let rest = Inline::Text { text: rest.into() };
            target.push(rest);
        }
    }

    fn collect_inlines(&mut self, node: AstRef) -> Result<Box<[Inline]>> {
        node.children()
            .try_fold(vec![], |mut vec, node| {
                self.make_inlines(node, &mut vec)?;
                Ok(vec)
            })
            .map(Into::into)
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
            NodeValue::HtmlInline(..) => return Ok(()),
            NodeValue::Emph => Inline::Emph(Inlines::new(self.collect_inlines(node)?)),
            NodeValue::Strong => Inline::Strong(Inlines::new(self.collect_inlines(node)?)),
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

                let mut new_cb = ChordBuilder::new(code, c_data.start_line);
                if self.xp.is_some() {
                    new_cb.transpose(&self.xp).with_context(|| {
                        format!(
                            "Failed to transpose: Uknown chord `{}` on line {}",
                            new_cb.chord, new_cb.line
                        )
                    })?;
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
    verse: Option<VerseBuilder>,
    blocks: Vec<Block>,
    xp: Transposition,
    verse_num: u32,
}

impl<'a> SongBuilder<'a> {
    fn new(nodes: &'a [AstRef<'a>], config: &ParserConfig) -> Self {
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
        }
    }

    fn next_verse_num(&mut self) -> u32 {
        self.verse_num += 1;
        self.verse_num
    }

    fn verse_mut(&mut self) -> &mut VerseBuilder {
        if self.verse.is_none() {
            self.verse = Some(VerseBuilder::new(VerseLabel::None {}, self.xp.clone()));
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
                    let verse = VerseBuilder::new(label, self.xp.clone());
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
                        let verse =
                            VerseBuilder::with_p_nodes(label, self.xp.clone(), item.children())?;
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
                    self.verse = Some(VerseBuilder::new(label, self.xp.clone()));
                }

                NodeValue::ThematicBreak => {
                    self.blocks.push(Block::HorizontalLine);
                }

                NodeValue::CodeBlock(cb) => self.blocks.push(Block::Pre {
                    text: cb.literal.as_bstr(),
                }),

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
    config: ParserConfig,
}

impl<'i> Parser<'i> {
    pub fn new(input: &'i str, config: ParserConfig) -> Self {
        Self { input, config }
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

    pub fn parse<'s>(&mut self, songs: &'s mut Vec<Song>) -> Result<&'s mut [Song]> {
        let arena = Arena::new();
        let root = comrak::parse_document(&arena, self.input, &Self::comrak_config());
        let root_elems: Vec<_> = root.children().collect();

        // Parsing is done in four steps:
        //
        // 1. Split the source AST in individual songs (they are separated by H1s),
        //    this is done by SongIter.
        //    For each song:
        // 2. Preprocess the AST for easier parsing, this mainly involves bringing up
        //    Code inlines and line breaks to the top level (out of arbitrary nested levels).
        //    This is done by methods in NodeExt
        // 3. Parse the song content, this is done in SongBuilder, VerseBuilder et al.,
        //    as well as helper methods in NodeExt.
        //    Parsing bard MD extensions, incl. transposition, is also done here.
        //    Transposition is applied right away, which is the sole reason
        //    why parsing is fallible (chords may fail to be understood by the music module).
        // 4. Postprocess the song data and convert into the final Song AST,
        //    as of now this is just removing of empty paragraphs/verses,
        //    this is actually implemented on the Song AST type (in book).
        //
        // The Result is one or more Song structures which are appended to the songs vec passed in.
        // See the book module where the bard AST is defined.

        let orig_len = songs.len();
        for song_nodes in SongsIter::new(&root_elems) {
            song_nodes.iter().for_each(|node| node.preprocess(&arena));

            let mut song = SongBuilder::new(song_nodes, &self.config);
            song.parse()?;
            songs.push(song.finalize());
        }

        Ok(&mut songs[orig_len..])
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn parse(input: &str, disable_xpose: bool) -> Vec<Song> {
        let mut songs = vec![];
        let mut parser = Parser::new(input, ParserConfig::default());
        parser.set_xp_disabled(disable_xpose);
        parser.parse(&mut songs).unwrap();
        songs
    }

    fn parse_one(input: &str) -> Song {
        let mut songs = parse(input, false);
        assert_eq!(songs.len(), 1);
        let song = songs.drain(..).next().unwrap();
        song
    }

    fn parse_one_para(input: &str) -> Paragraph {
        let blocks = parse_one(input).blocks;
        let block = Vec::from(blocks).drain(..).next().unwrap();
        match block {
            Block::Verse(v) => Vec::from(v.paragraphs).drain(..).next().unwrap(),
            _ => panic!("First block in this Song isn't a Verse"),
        }
    }

    fn get_verse(song: &Song, block_num: usize) -> &Verse {
        match &song.blocks[block_num] {
            Block::Verse(verse) => verse,
            b => panic!("Unexpected block type: {:?}", b),
        }
    }

    #[test]
    fn songs_split() {
        let input = r#"
No-heading lyrics
# Song 1
Lyrics lyrics...
# Song 2
Lyrics lyrics...
        "#;

        let songs = parse(&input, false);

        assert_eq!(songs.len(), 3);
        assert_eq!(&*songs[0].title, FALLBACK_TITLE);
        assert_eq!(&*songs[1].title, "Song 1");
        assert_eq!(&*songs[2].title, "Song 2");
    }

    #[test]
    fn ast_split_at() {
        let input = r#"_text **strong** `C`text2 **strong2**_"#;

        let arena = Arena::new();
        let options = ComrakOptions::default();
        let root = comrak::parse_document(&arena, input, &options);

        let para = root.children().next().unwrap();
        let em = para.children().next().unwrap();
        let code = em.split_at(3, &arena);
        let em2 = code.split_at(1, &arena);

        assert_eq!(em.children().count(), 3);
        assert_eq!(em.as_plaintext(), "text strong ");
        assert_eq!(code.children().count(), 1);
        assert_eq!(code.as_plaintext(), "C");
        assert_eq!(em2.children().count(), 2);
        assert_eq!(em2.as_plaintext(), "text2 strong2");
    }

    #[test]
    fn ast_preprocess() {
        let input = r#"
Lyrics _em **strong `C` strong**
em_ lyrics
        "#;

        let arena = Arena::new();
        let options = ComrakOptions::default();
        let root = comrak::parse_document(&arena, input, &options);

        let para = root.children().next().unwrap();
        para.preprocess(&arena);

        assert_eq!(para.children().count(), 7);
        let code = para
            .children()
            .find(|c| c.is_code())
            .unwrap()
            .as_plaintext();
        assert_eq!(code, "C");
        para.children().find(|c| c.is_break()).unwrap();
    }

    #[test]
    fn parse_verses_basic() {
        let input = r#"
# Song
1. First verse.

Second paragraph of the first verse.

2. Second verse.

Second paragraph of the second verse.

3. Third verse.
4. Fourth verse.
> Chorus.
"#;

        parse_one(input).assert_eq(json!({
            "title": "Song",
            "subtitles": [],
            "notation": "english",
            "blocks": [
                {
                    "type": "b-verse",
                    "label": { "verse": 1 },
                    "paragraphs": [
                        [{ "type": "i-text", "text": "First verse." }],
                        [{ "type": "i-text", "text": "Second paragraph of the first verse." }],
                    ],
                },
                {
                    "type": "b-verse",
                    "label": { "verse": 2 },
                    "paragraphs": [
                        [{ "type": "i-text", "text": "Second verse." }],
                        [{ "type": "i-text", "text": "Second paragraph of the second verse." }],
                    ],
                },
                {
                    "type": "b-verse",
                    "label": { "verse": 3 },
                    "paragraphs": [[{ "type": "i-text", "text": "Third verse." }]],
                },
                {
                    "type": "b-verse",
                    "label": { "verse": 4 },
                    "paragraphs": [[{ "type": "i-text", "text": "Fourth verse." }]],
                },
                {
                    "type": "b-verse",
                    "label": { "chorus": null },
                    "paragraphs": [[{ "type": "i-text", "text": "Chorus." }]],
                },
            ],
        }));
    }

    #[test]
    fn parse_verses_corners() {
        let input = r#"
# Song

Verse without any label.

Next paragraph of that verse.

### Custom label

Lyrics Lyrics lyrics.

> Chorus 1.
>> Chorus 2.
>
> Chorus 1 again.
>
> More lyrics.

Yet more lyrics (these should go to the chorus as well actually).

>>> Chorus 3.

More lyrics to the chorus 3.

"#;

        parse_one(input).assert_eq(json!({
            "title": "Song",
            "subtitles": [],
            "notation": "english",
            "blocks": [
                {
                    "type": "b-verse",
                    "label": { "none": {} },
                    "paragraphs": [
                        [{ "type": "i-text", "text": "Verse without any label." }],
                        [{ "type": "i-text", "text": "Next paragraph of that verse." }],
                    ],
                },
                {
                    "type": "b-verse",
                    "label": { "custom": "Custom label" },
                    "paragraphs": [
                        [{ "type": "i-text", "text": "Lyrics Lyrics lyrics." }],
                    ],
                },
                {
                    "type": "b-verse",
                    "label": { "chorus": 1 },
                    "paragraphs": [
                        [{ "type": "i-text", "text": "Chorus 1." }],
                    ],
                },
                {
                    "type": "b-verse",
                    "label": { "chorus": 2 },
                    "paragraphs": [
                        [{ "type": "i-text", "text": "Chorus 2." }],
                    ],
                },
                {
                    "type": "b-verse",
                    "label": { "chorus": 1 },
                    "paragraphs": [
                        [{ "type": "i-text", "text": "Chorus 1 again." }],
                        [{ "type": "i-text", "text": "More lyrics." }],
                        [{ "type": "i-text", "text": "Yet more lyrics (these should go to the chorus as well actually)." }],
                    ],
                },
                {
                    "type": "b-verse",
                    "label": { "chorus": 3 },
                    "paragraphs": [
                        [{ "type": "i-text", "text": "Chorus 3." }],
                        [{ "type": "i-text", "text": "More lyrics to the chorus 3." }],
                    ],
                },
            ],
        }));
    }

    #[test]
    fn parse_subtitles() {
        let input = r#"
# Song
## Subtitle 1
## Subtitle 2

Some lyrics.

## This one should be ignored
"#;

        let song = parse_one(input);
        assert_eq!(
            &*song.subtitles,
            &["Subtitle 1".into(), "Subtitle 2".into(),]
        );
    }

    #[test]
    fn parse_chords() {
        let input = r#"
# Song
1. Sailing round `G`the ocean,
Sailing round the ```D```sea.
"#;
        parse_one_para(input).assert_eq(json!([
            { "type": "i-text", "text": "Sailing round " },
            {
                "type": "i-chord",
                "chord": "G",
                "alt_chord": null,
                "backticks": 1,
                "inlines": [{ "type": "i-text", "text": "the ocean," }],
            },
            { "type": "i-break" },
            { "type": "i-text", "text": "Sailing round the " },
            {
                "type": "i-chord",
                "chord": "D",
                "alt_chord": null,
                "backticks": 3,
                "inlines": [{ "type": "i-text", "text": "sea." }],
            },
        ]));
    }

    #[test]
    fn parse_inlines() {
        let input = r#"
# Song
1. Sailing **round `G`the _ocean,
Sailing_ round the `D`sea.**
"#;
        parse_one_para(input).assert_eq(json!([
            { "type": "i-text", "text": "Sailing " },
            { "type": "i-strong", "inlines": [{ "type": "i-text", "text": "round " }] },
            {
                "type": "i-chord",
                "chord": "G",
                "alt_chord": null,
                "backticks": 1,
                "inlines": [{
                    "type": "i-strong",
                    "inlines": [
                        { "type": "i-text", "text": "the "  },
                        { "type": "i-emph", "inlines": [{ "type": "i-text", "text": "ocean," }] },
                    ]
                }],
            },
            { "type": "i-break" },
            {
                "type": "i-strong",
                "inlines": [
                    { "type": "i-emph", "inlines": [{ "type": "i-text", "text": "Sailing" }] },
                    { "type": "i-text", "text": " round the " },
                ],
            },
            {
                "type": "i-chord",
                "chord": "D",
                "alt_chord": null,
                "backticks": 1,
                "inlines": [{
                    "type": "i-strong",
                    "inlines": [{ "type": "i-text", "text": "sea."  }]
                }],
            },
        ]));
    }

    #[test]
    fn parse_extensions() {
        let input = r#"
# Song

!+5
!!czech

> Chorus.

1. Lyrics !!> !!!english !+0
!+2 More lyrics !>

# Song two

> Chorus.

>> Chorus two.

1. Reference both: !> !>>
!> First on the line.
Mixed !>> in text.

"#;

        let songs = parse(input, true);

        songs[0].blocks.assert_eq(json!([
            {
              "type": "b-verse",
              "label": { "none": {} },
              "paragraphs": [
                [
                  { "type": "i-transpose", "t-transpose": 5 },
                  { "type": "i-break" },
                  { "type": "i-transpose", "t-alt-notation": "german" },
                ]
              ],
            },
            {
              "type": "b-verse",
              "label": { "chorus": null },
              "paragraphs": [
                [
                  { "type": "i-text", "text": "Chorus." },
                ]
              ]
            },
            {
              "type": "b-verse",
              "label": { "verse": 1 },
              "paragraphs": [
                [
                    { "type": "i-text", "text": "Lyrics !!> !!!english" },
                    { "type": "i-transpose", "t-transpose": 0 },
                    { "type": "i-break" },
                    { "type": "i-transpose", "t-transpose": 2 },
                    { "type": "i-text", "text": " More lyrics" },
                    { "type": "i-chorus-ref", "num": null, "prefix_space": " " },
                ]
              ]
            }
          ]
        ));

        songs[1].blocks.assert_eq(json!([
            {
              "type": "b-verse",
              "label": { "chorus": 1 },
              "paragraphs": [
                [
                  { "type": "i-text", "text": "Chorus." },
                ]
              ]
            },
            {
              "type": "b-verse",
              "label": { "chorus": 2 },
              "paragraphs": [
                [
                  { "type": "i-text", "text": "Chorus two." },
                ]
              ]
            },
            {
              "type": "b-verse",
              "label": { "verse": 1 },
              "paragraphs": [
                [
                    { "type": "i-text", "text": "Reference both:" },
                    { "type": "i-chorus-ref", "num": 1, "prefix_space": " "},
                    { "type": "i-chorus-ref", "num": 2, "prefix_space": " "},
                    { "type": "i-break" },
                    { "type": "i-chorus-ref", "num": 1, "prefix_space": ""},
                    { "type": "i-text", "text": " First on the line." },
                    { "type": "i-break" },
                    { "type": "i-text", "text": "Mixed" },
                    { "type": "i-chorus-ref", "num": 2, "prefix_space": " "},
                    { "type": "i-text", "text": " in text." },
                ]
              ]
            }
          ]
        ));
    }

    #[test]
    fn transposition() {
        let input = r#"
# Song

!+5
!!czech

> 1. `Bm`Yippie yea `D`oh! !+0
!+0 Yippie yea `Bm`yay!

"#;

        let song = parse_one(input);
        song.blocks.assert_eq(json!([
            {
              "type": "b-verse",
              "label": { "chorus": null },
              "paragraphs": [
                [
                  {
                    "type": "i-chord",
                    "chord": "Em",
                    "alt_chord": "Hm",
                    "backticks": 1,
                    "inlines": [ { "type": "i-text", "text": "Yippie yea " } ],
                  },
                  {
                    "type": "i-chord",
                    "chord": "G",
                    "alt_chord": "D",
                    "backticks": 1,
                    "inlines": [ { "type": "i-text", "text": "oh!" } ],
                  },
                  { "type": "i-break" },
                  { "type": "i-text", "text": "Yippie yea " },
                  {
                    "type": "i-chord",
                    "chord": "Bm",
                    "alt_chord": "Hm",
                    "backticks": 1,
                    "inlines": [ { "type": "i-text", "text": "yay!" } ],
                  },
                ]
              ]
            }
        ]));
    }

    #[test]
    fn parse_verse_numbering() {
        let input = r#"
# Song 1

1. Verse 1.
> Chorus 1.
1. Verse 2.
>> Chorus 2.
1. Verse 3.

# Song 2

1. Verse 1.
2. Verse 2.
> Chorus.
>> Chorus two.
3. Verse 3.
3. Verse 3.
"#;

        let songs = parse(input, true);

        assert_eq!(get_verse(&songs[0], 0).label, VerseLabel::Verse(1));
        assert_eq!(get_verse(&songs[0], 2).label, VerseLabel::Verse(2));
        assert_eq!(get_verse(&songs[0], 4).label, VerseLabel::Verse(3));

        assert_eq!(get_verse(&songs[1], 0).label, VerseLabel::Verse(1));
        assert_eq!(get_verse(&songs[1], 1).label, VerseLabel::Verse(2));
        assert_eq!(get_verse(&songs[1], 4).label, VerseLabel::Verse(3));
        assert_eq!(get_verse(&songs[1], 5).label, VerseLabel::Verse(4));
    }
}
