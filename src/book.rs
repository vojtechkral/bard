use std::mem;
use std::iter;
use std::path::Path;

use serde::ser::{Serialize, Serializer, SerializeStruct};
use unicode_width::UnicodeWidthStr;

use crate::parser::{MDFile, Event, Transpose, Range, ParsingDebug};
use crate::music::{Time, Notation, Chord};
use crate::cli;
use crate::util::SmallStr;
use crate::error::*;


#[derive(Debug)]
struct Transposition {
    notation: Notation,
    transpose: Transpose,
    transpose_alt: Transpose,
}

impl Transposition {
    fn new(notation: Notation) -> Transposition {
        Transposition {
            notation,
            transpose: Transpose::default(),
            transpose_alt: Transpose::default(),
        }
    }

    fn reset(&mut self) {
        self.transpose = Transpose::default();
        self.transpose_alt = Transpose::default();
    }

    #[inline]
    fn is_some(&self) -> bool {
        self.transpose.is_some() || self.transpose_alt.is_some()
    }
}


#[derive(Clone, Serialize, Debug)]
pub struct Span {
    pub chord: SmallStr,
    pub chord_alt: SmallStr,
    pub lyrics: SmallStr,
}

impl Span {
    fn new(chord: SmallStr, lyrics: SmallStr) -> Span {
        Span {
            chord,
            chord_alt: Default::default(),
            lyrics,
        }
    }

    fn transposed(mut self, tr: &Transposition) -> Result<Span, Span> {
        if self.chord.is_empty() || !tr.is_some() {
            return Ok(self);
        }

        let orig = match Chord::parse(&self.chord, tr.notation) {
            Some(orig) => orig,
            None => return Err(self),
        };

        if tr.transpose.is_some() {
            let notation_to = tr.transpose.notation.unwrap_or(tr.notation);
            self.chord = orig
                .transposed(tr.transpose.delta)
                .to_string(notation_to)
                .into();
        }

        if tr.transpose_alt.is_some() {
            let notation_to = tr.transpose_alt.notation.unwrap_or(tr.notation);
            self.chord_alt = orig
                .transposed(tr.transpose_alt.delta)
                .to_string(notation_to)
                .into();
        }

        Ok(self)
    }

    fn num_chords(&self) -> u32 {
        match (self.chord.is_empty(), self.chord_alt.is_empty()) {
            (true, _) => 0,
            (false, true) => 1,
            (false, false) => 2,
        }
    }
}

impl UnicodeWidthStr for Span {
    fn width(&self) -> usize {
        self.chord
            .width()
            .max(self.chord_alt.width())
            .max(self.lyrics.width())
    }

    fn width_cjk(&self) -> usize {
        self.chord
            .width_cjk()
            .max(self.chord_alt.width_cjk())
            .max(self.lyrics.width_cjk())
    }
}

#[derive(Serialize)]
struct SpanSerialize<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    chord: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chord_alt: Option<&'a str>,
    lyrics: &'a str,
}

impl<'a> SpanSerialize<'a> {
    fn new(span: &'a Span, chord_rows: u32) -> Self {
        let (chord, chord_alt) = match chord_rows {
            0 => (None, None),
            1 => (Some(&*span.chord), None),
            _ => (Some(&*span.chord), Some(&*span.chord_alt)),
        };

        Self {
            chord,
            chord_alt,
            lyrics: &span.lyrics,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LyricLine {
    pub spans: Vec<Span>,
}

impl LyricLine {
    fn new() -> Self {
        Self { spans: vec![] }
    }

    pub fn chord_rows(&self) -> u32 {
        self.spans.iter().map(Span::num_chords).max().unwrap_or(0)
    }
}

struct SpanVecSerialize<'a> {
    spans: &'a Vec<Span>,
    chord_rows: u32,
}

impl<'a> Serialize for SpanVecSerialize<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let map = self
            .spans
            .iter()
            .map(|span| SpanSerialize::new(span, self.chord_rows));
        serializer.collect_seq(map)
    }
}

impl Serialize for LyricLine {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let spans = &self.spans;
        let chord_rows = self.chord_rows();
        let spans_serialize = SpanVecSerialize { spans, chord_rows };

        let mut state = serializer.serialize_struct("LyricLine", 2)?;
        state.serialize_field("spans", &spans_serialize)?;
        state.serialize_field("chord_rows", &chord_rows)?;
        state.end()
    }
}

#[derive(Clone, Serialize, Default, Debug)]
pub struct Verse {
    pub label: SmallStr,
    pub is_chorus: bool,
    pub lines: Vec<LyricLine>,
}

impl Verse {
    fn new(label: SmallStr, is_chorus: bool) -> Self {
        Self {
            label,
            is_chorus,
            lines: vec![],
        }
    }

    fn add_span(&mut self, span: Span, newline: bool) {
        if newline || self.lines.is_empty() {
            self.lines.push(LyricLine::new());
        }

        self.lines.last_mut().unwrap().spans.push(span);
    }
}

#[derive(Clone, Serialize, Default, Debug)]
pub struct List {
    pub items: Vec<SmallStr>,
}

impl List {
    #[inline]
    fn push(&mut self, item: SmallStr) {
        self.items.push(item);
    }
}

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub enum Item {
    Verse(Verse),
    List(List),
    Time { time: Time },
    Rule,
    Pre { text: SmallStr },
}


#[derive(Serialize, Debug)]
pub struct Song {
    pub title: SmallStr,
    pub subtitles: Vec<SmallStr>,
    pub content: Vec<Item>,
}

impl Song {
    fn new(title: SmallStr) -> Song {
        Song {
            title,
            subtitles: vec![],
            content: vec![],
        }
    }
}

#[derive(Debug)]
struct SongBuilder<'a> {
    song: Option<Song>,
    tr: Transposition,
    source_path: &'a Path,
    source: &'a str,
}

impl<'a> SongBuilder<'a> {
    fn new(notation: Notation, source_path: &'a Path, source: &'a str) -> Self {
        Self {
            song: None,
            tr: Transposition::new(notation),
            source_path,
            source,
        }
    }

    fn song_ref(&mut self) -> &mut Song {
        if self.song.is_none() {
            self.song = Some(Song::new(Default::default()));
        }

        self.song.as_mut().unwrap()
    }

    /// starts a new song, returns previous one if any
    fn new_song(&mut self, title: SmallStr) -> Option<Song> {
        self.tr.reset();
        mem::replace(&mut self.song, Some(Song::new(title)))
    }

    fn new_verse(&mut self, label: SmallStr, is_chorus: bool) {
        self.song_ref()
            .content
            .push(Item::Verse(Verse::new(label, is_chorus)));
    }

    fn add_subtitle(&mut self, subtitle: SmallStr) {
        self.song_ref().subtitles.push(subtitle);
    }

    /// Finds a line number and line range base
    /// on an byte offset inside a string.
    /// The returned range is [inclusive; exclusive).
    /// If the line is empty, the range start and end will equal, ie. empty.
    /// The line number is 0-indexed.
    fn find_line(source: &str, mut offset: usize) -> (usize, Range) {
        // This stuff is an amusement park for off-by-one errors.
        // See the test case in the test module below...

        if source.is_empty() {
            return (0, 0..0);
        }

        offset = offset.min(source.len() - 1);

        if offset > 1 && &source[offset - 1..offset + 1] == "\n\r" {
            offset = offset - 1;
        }

        let (line_no, mut line_start) = source[..offset]
            .char_indices()
            .filter(|(_, c)| *c == '\n')
            .map(|(offset, _)| offset + 1)
            .enumerate()
            .map(|(line_no, offset)| (line_no + 1, offset))
            .last()
            .unwrap_or((0, 0));

        if &source[line_start..line_start + 1] == "\r" {
            line_start += 1;
        }

        let line_end = source[line_start..]
            .char_indices()
            .find(|(_, c)| *c == '\n')
            .map(|(offset, _)| offset + line_start)
            .unwrap_or(source.len());

        (line_no, line_start..line_end)
    }

    fn tr_error(&self, span: Span, range: Range) -> anyhow::Error {
        let (line_no, line_range) = Self::find_line(self.source, range.start);
        let line_no = format!("line {}:", line_no + 1);
        let line = &self.source[line_range.clone()];

        let spacing_width = line_no.width() + self.source[line_range.start..range.start].width();
        let chord_width = self.source[range].width();
        let spacing: String = iter::repeat(' ').take(spacing_width).collect();
        let underline: String = iter::repeat('^').take(chord_width).collect();

        anyhow!(
            r#"Could not transpose chord `{}` in file `{}`:

{} {}
{} {}

The chord is not valid or unknown to bard.
Note: Please also check that chord notation setting is correct."#,
            &span.chord,
            self.source_path.display(),
            cli::yellow(&line_no),
            line,
            spacing,
            cli::yellow(&underline),
        )
    }

    fn add_span(&mut self, span: Span, newline: bool, range: Range) -> Result<()> {
        let span = span
            .transposed(&self.tr)
            .map_err(|span| self.tr_error(span, range))?;

        let content = &mut self.song_ref().content;

        // Make sure last item is a verse
        if !matches!(content.last(), Some(Item::Verse(_))) {
            content.push(Item::Verse(Verse::default()));
        }

        match content.last_mut() {
            Some(Item::Verse(verse)) => verse.add_span(span, newline),
            _ => unreachable!(),
        }

        Ok(())
    }

    fn add_bullet(&mut self, text: SmallStr) {
        let content = &mut self.song_ref().content;

        // Make sure last item is a list
        if !matches!(content.last(), Some(Item::List(_))) {
            content.push(Item::List(List::default()));
        }

        match content.last_mut() {
            Some(Item::List(list)) => list.push(text),
            _ => unreachable!(),
        }
    }

    fn add_rule(&mut self) {
        self.song_ref().content.push(Item::Rule);
    }

    fn add_time(&mut self, time: Time) {
        self.song_ref().content.push(Item::Time { time });
    }

    fn add_pre(&mut self, text: SmallStr) {
        self.song_ref().content.push(Item::Pre { text });
    }

    fn set_notation(&mut self, notation: Notation) {
        self.tr.notation = notation;
    }

    fn set_transpose(&mut self, chord_set: u32, transpose: Transpose) {
        match chord_set {
            0 => self.tr.transpose = transpose,
            1 => self.tr.transpose_alt = transpose,
            _ => unreachable!(),
        }
    }

    fn finalize(mut self) -> Option<Song> {
        mem::replace(&mut self.song, None)
    }
}

#[derive(Debug)]
pub struct Book {
    pub songs: Vec<Song>,
    notation: Notation,
    chorus_label: SmallStr,
    pub parsing_debug: Option<ParsingDebug>,
}

impl Book {
    pub fn new(notation: Notation, chorus_label: &str, parsing_debug: bool) -> Book {
        let parsing_debug = if parsing_debug {
            Some(ParsingDebug::default())
        } else {
            None
        };

        Book {
            songs: vec![],
            notation,
            chorus_label: chorus_label.into(),
            parsing_debug,
        }
    }

    fn format_label(&self, label: SmallStr, chorus: bool) -> SmallStr {
        if chorus && !label.is_empty() {
            format!("{} {}", self.chorus_label, label).into()
        } else if chorus {
            self.chorus_label.clone()
        } else {
            label
        }
    }

    fn add_md(&mut self, mdfile: MDFile, path: &Path) -> Result<()> {
        use Event::*;

        let source = mdfile.content();
        let mut builder = SongBuilder::new(self.notation, path, source);

        let mut events = mdfile.parse();
        for event in &mut events {
            match event {
                Song(title) => {
                    if let Some(prev_song) = builder.new_song(title) {
                        self.songs.push(prev_song);
                    }
                }
                Subtitle(subtitle) => builder.add_subtitle(subtitle),

                Clef { time, notation } => {
                    if let Some(time) = time {
                        builder.add_time(time);
                    }
                    if let Some(notation) = notation {
                        builder.set_notation(notation);
                    }
                }
                Transpose {
                    chord_set,
                    transpose,
                } => builder.set_transpose(chord_set, transpose),

                Verse { label, chorus } => {
                    builder.new_verse(self.format_label(label, chorus), chorus)
                }
                Span {
                    chord,
                    lyrics,
                    newline,
                    range,
                } => builder.add_span(self::Span::new(chord, lyrics), newline, range)?,

                Bullet(content) => builder.add_bullet(content),
                Rule => builder.add_rule(),
                Pre(text) => builder.add_pre(text),
            }
        }

        if let Some(song) = builder.finalize() {
            self.songs.push(song);
        }

        if let Some(debug) = self.parsing_debug.as_mut() {
            if let Some(debug_current) = events.take_debug() {
                debug.append(debug_current);
            }
        }

        Ok(())
    }

    pub fn add_md_str(&mut self, s: &str) -> Result<()> {
        static STR_PATH: &'static str = "<buffer>";

        let mdfile = MDFile::from_str(s, self.parsing_debug.is_some());
        self.add_md(mdfile, &Path::new(&STR_PATH))
    }

    pub fn add_md_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        let mdfile = MDFile::new(path, self.parsing_debug.is_some())
            .with_context(|| format!("Error reading markdown file '{}'", path.display()))?;

        self.add_md(mdfile, path)
    }

    pub fn load_files<P: AsRef<Path>>(&mut self, input_paths: &[P]) -> Result<()> {
        for path in input_paths.iter() {
            let path = path.as_ref();
            self.add_md_file(&path)?;
        }

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_line() {
        let input = "abc\ndef\n\r\nghi\n";
        //           012 3456 7 8 9012 3

        assert_eq!(SongBuilder::find_line(input, 1), (0, 0..3));
        assert_eq!(SongBuilder::find_line(input, 1), (0, 0..3));
        assert_eq!(SongBuilder::find_line(input, 1), (0, 0..3));
        assert_eq!(SongBuilder::find_line(input, 1), (0, 0..3));
        assert_eq!(SongBuilder::find_line(input, 3), (0, 0..3));
        assert_eq!(SongBuilder::find_line(input, 4), (1, 4..7));
        assert_eq!(SongBuilder::find_line(input, 7), (1, 4..7));
        assert_eq!(SongBuilder::find_line(input, 8), (1, 4..7));
        assert_eq!(SongBuilder::find_line(input, 9), (2, 9..9));
        assert_eq!(SongBuilder::find_line(input, 10), (3, 10..13));
        assert_eq!(SongBuilder::find_line(input, 9001), (3, 10..13));

        let input = "abc";
        assert_eq!(SongBuilder::find_line(input, 0), (0, 0..3));
        assert_eq!(SongBuilder::find_line(input, 2), (0, 0..3));
        assert_eq!(SongBuilder::find_line(input, 20), (0, 0..3));

        let input = "";
        assert_eq!(SongBuilder::find_line(input, 0), (0, 0..0));
        assert_eq!(SongBuilder::find_line(input, 20), (0, 0..0));
    }

    fn make_book() -> Book {
        Book::new(Notation::default(), "Ch.".into(), true)
    }

    #[test]
    fn settings_between_songs() {
        let mut book = make_book();

        book.add_md_str(
            r#"# Song1

$ 3/4
^ 2

# Song2

`C`lyrics
"#,
        )
        .expect("Failed to parse md");

        println!("book: {:#?}", book);

        assert!(matches!(book.songs[0].content[0], Item::Time {
            time: (3, 4)
        }));

        let song2 = &book.songs[1];
        if let Item::Verse(verse) = &song2.content[0] {
            assert_eq!(verse.lines[0].spans[0].chord, "C".into());
        } else {
            panic!("Unexpected item");
        }
    }
}
