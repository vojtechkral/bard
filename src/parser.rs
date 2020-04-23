use std::mem;
use std::path::Path;
use std::io;
use std::fs;

use pulldown_cmark as md;

use crate::music::{Time, Notation, Chromatic};
use crate::util::SmallStr;


pub type Range = std::ops::Range<usize>;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Transpose {
    pub delta: Chromatic,
    pub notation: Option<Notation>,
}

impl Transpose {
    pub fn new(delta: Chromatic, notation: Option<Notation>) -> Transpose {
        Transpose { delta, notation }
    }

    pub fn is_some(&self) -> bool {
        self.delta != 0.into() || self.notation.is_some()
    }

    pub fn get_notation(&self, default: Notation) -> Notation {
        self.notation.unwrap_or(default)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Event {
    Song(SmallStr),
    Subtitle(SmallStr),

    Clef {
        time: Option<Time>,
        notation: Option<Notation>,
    },
    Transpose {
        chord_set: u32,
        transpose: Transpose,
    },

    Verse {
        label: SmallStr,
        chorus: bool,
    },
    Span {
        chord: SmallStr,
        lyrics: SmallStr,
        newline: bool,
        range: Range,
    },

    Bullet(SmallStr),
    Rule,
    Pre(SmallStr),
}


trait StringExt {
    fn take(&mut self) -> SmallStr;
}

impl StringExt for String {
    fn take(&mut self) -> SmallStr {
        let res = self.as_str().into();
        self.clear();
        res
    }
}


/// Parsing state: What element we're currently parsing
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
enum Element {
    Header(u32),
    TextLyrics,
    Bullet,
    Pre,
}

#[derive(Default, Serialize, Debug)]
pub struct ParsingDebug {
    pub evts_md: Vec<SmallStr>,
    pub evts_bard: Vec<SmallStr>,
}

impl ParsingDebug {
    pub fn append(&mut self, mut other: ParsingDebug) {
        self.evts_md.append(&mut other.evts_md);
        self.evts_bard.append(&mut other.evts_bard);
    }
}

pub struct Events<'a> {
    md: md::OffsetIter<'a>,

    elem: Element,
    text: String,
    chord: Option<SmallStr>,
    chord_range: Range,
    ol_level: u32,
    ol_item: Option<i32>,
    bq_level: u32,
    verse_open: bool,
    newline: bool,
    line_start: bool,

    stashed: Option<Event>,

    debug: Option<ParsingDebug>,
}

impl<'a> Events<'a> {
    fn new(md: md::OffsetIter<'a>, collect_debug: bool) -> Events<'a> {
        let debug = if collect_debug {
            Some(ParsingDebug::default())
        } else {
            None
        };

        Events {
            md,
            elem: Element::TextLyrics,
            text: String::new(),
            chord: None,
            chord_range: 0..0,
            ol_level: 0,
            ol_item: None,
            bq_level: 0,
            verse_open: false,
            newline: true,
            line_start: true,
            stashed: None,
            debug,
        }
    }

    pub fn take_debug(&mut self) -> Option<ParsingDebug> {
        self.debug.take()
    }

    fn start_tag(&mut self, tag: md::Tag) -> Option<Event> {
        use self::md::Tag::*;

        match tag {
            Paragraph => None,
            Heading(num) => {
                self.elem = Element::Header(num.min(3));
                self.text.clear();
                self.verse_end();
                None
            }
            BlockQuote => {
                self.bq_level += 1;
                self.verse_end();
                None
                // Note: A verse start is not dispatched right away,
                // there might be a list following.
            }
            CodeBlock(_s) => {
                self.elem = Element::Pre;
                self.text.clear();
                self.verse_end();
                None
            }
            List(Some(num)) => {
                self.ol_level += 1;
                if self.ol_level == 1 {
                    // Here we subtract one because Item event will add one
                    self.ol_item = Some(num as i32 - 1);
                }
                self.verse_end();
                None
                // Note: A verse start is not dispatched right away,
                // there might be a blockquoute following.
            }
            List(None) => {
                if self.elem == Element::TextLyrics {
                    self.elem = Element::Bullet;
                }
                None
            }
            Item => {
                if let Some(i) = self.ol_item.as_mut() {
                    *i += 1;
                }
                None
            }
            FootnoteDefinition(_s) => None,
            Emphasis => {
                self.line_start = false;
                None
            }
            Strong => {
                self.line_start = false;
                None
            }
            Strikethrough => {
                self.line_start = false;
                None
            }
            Link(_link_type, _url, _title) => {
                self.line_start = false;
                None
            }
            Image(_link_type, _url, _title) => {
                self.line_start = false;
                None
            }

            Table(_) | TableHead | TableRow | TableCell => unreachable!(),
        }
    }

    fn end_tag(&mut self, tag: md::Tag) -> Option<Event> {
        use self::md::Tag::*;

        match tag {
            Paragraph => {
                if self.elem == Element::TextLyrics {
                    self.line_end(true)
                } else {
                    None
                }
            }
            Heading(num) => {
                self.elem = Element::TextLyrics;
                let text = self.text.take();
                Some(match num {
                    1 => Event::Song(text.into()),
                    2 => Event::Subtitle(text.into()),
                    _ => self.verse_event(text, self.bq_level > 0),
                })
            }
            BlockQuote => {
                self.bq_level = self
                    .bq_level
                    .checked_sub(1)
                    .expect("Internal error: Invalid parser state");
                None
            }
            CodeBlock(_s) => {
                self.elem = Element::TextLyrics;
                let text = self.text.take();
                Some(Event::Pre(text))
            }
            List(Some(_num)) => {
                self.ol_level = self
                    .ol_level
                    .checked_sub(1)
                    .expect("Internal error: Invalid parser state");
                if self.ol_level == 0 {
                    self.ol_item = None;
                }
                None
            }
            List(None) => {
                if self.elem == Element::Bullet {
                    self.elem = Element::TextLyrics;
                }
                None
            }
            Item => self.line_end(true),
            FootnoteDefinition(_s) => None,
            Emphasis => None,
            Strong => None,
            Strikethrough => None,
            Link(_link_type, _url, _title) => None,
            Image(_link_type, _url, _title) => None,

            Table(_) | TableHead | TableRow | TableCell => unreachable!(),
        }
    }

    fn get_span(&mut self) -> Option<Event> {
        if !self.text.is_empty() || self.chord.is_some() {
            // We preserve buffer capacity here

            let chord = self.chord.take().unwrap_or(Default::default());
            let lyrics = self.text.as_str().into();
            self.text.clear();

            Some(Event::Span {
                chord,
                lyrics,
                newline: mem::replace(&mut self.newline, false),
                range: self.chord_range.clone(),
            })
        } else {
            None
        }
    }

    fn verse_event(&mut self, label: SmallStr, chorus: bool) -> Event {
        self.verse_open = true;
        Event::Verse { label, chorus }
    }

    /// Ensures that a verse is always open before dispatching a span
    fn verse_or_span(&mut self) -> Option<Event> {
        if let Some(evt) = self.get_span() {
            if !self.verse_open {
                self.stashed = Some(evt);

                let label = self
                    .ol_item
                    .map(|i| format!("{}.", i).into())
                    .unwrap_or(Default::default());

                Some(self.verse_event(label, self.bq_level > 0))
            } else {
                Some(evt)
            }
        } else {
            None
        }
    }

    fn verse_end(&mut self) {
        self.verse_open = false;
        self.newline = true;
    }

    fn parse_dollar(&mut self) -> Option<Event> {
        if !self.line_start || self.chord.is_some() || self.text.get(0..1) != Some("$") {
            return None;
        }

        let mut time: Option<Time> = None;
        let mut notation: Option<Notation> = None;

        for arg in self.text[1..].split_ascii_whitespace() {
            if arg
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                // This looks like a time signtarue argument
                if time.is_some() {
                    return None;
                }

                match arg
                    .find('/')
                    .map(|i| arg.split_at(i))
                    .map(|(a, b)| (a.parse(), b.get(1..).and_then(|b| b.parse().ok())))
                {
                    Some((Ok(a), Some(b))) => time = Some((a, b)),
                    _ => return None,
                }
            } else {
                // This looks like a notation argument
                if notation.is_some() {
                    return None;
                }

                match arg.parse() {
                    Ok(m) => notation = Some(m),
                    Err(_) => return None,
                }
            }
        }

        if time.is_some() || notation.is_some() {
            self.text.clear();
            Some(Event::Clef { time, notation })
        } else {
            None
        }
    }

    fn parse_carret(&mut self) -> Option<Event> {
        if !self.line_start || self.chord.is_some() {
            return None;
        }

        let num_carrets = self.text.chars().take_while(|c| *c == '^').count();

        if num_carrets == 0 || num_carrets > 2 {
            return None;
        }

        let mut delta: Option<Chromatic> = None;
        let mut notation: Option<Notation> = None;

        for arg in self.text[num_carrets..].split_ascii_whitespace() {
            if arg
                .chars()
                .next()
                .map(|c| c == '-' || c == '+' || c.is_ascii_digit())
                .unwrap_or(false)
            {
                // This looks like a numerical pitch delta argument
                if delta.is_some() {
                    return None;
                }

                match arg.parse() {
                    Ok(d) => delta = Some(d),
                    Err(_) => return None,
                }
            } else {
                // This looks like a notation argument
                if notation.is_some() {
                    return None;
                }

                match arg.parse() {
                    Ok(m) => notation = Some(m),
                    Err(_) => return None,
                }
            }
        }

        if delta.is_some() || notation.is_some() {
            self.text.clear();
            let transpose = Transpose::new(delta.unwrap_or(0.into()), notation);
            let chord_set = num_carrets as u32 - 1;
            Some(Event::Transpose {
                chord_set,
                transpose,
            })
        } else {
            None
        }
    }

    fn line_end(&mut self, verse_end: bool) -> Option<Event> {
        use Element::*;

        let res = match self.elem {
            TextLyrics => {
                // Check if this line is a time, notation or transposition instruction
                if let Some(evt) = self.parse_dollar() {
                    Some(evt)
                } else if let Some(evt) = self.parse_carret() {
                    Some(evt)
                } else {
                    // Dispatch span, if any
                    self.verse_or_span()
                }
            }
            Bullet => Some(Event::Bullet(self.text.take())),
            _ => None,
        };

        self.newline = true;
        self.line_start = true;

        if verse_end {
            self.verse_end();
        }

        res
    }

    fn next_inner(&mut self) -> Option<Event> {
        use self::md::Event as M;

        loop {
            if self.stashed.is_some() {
                return self.stashed.take();
            }

            let (md_evt, range) = self.md.next()?;

            if let Some(debug) = self.debug.as_mut() {
                let evt_s = format!("{:?}", md_evt).into();
                debug.evts_md.push(evt_s);
            }

            let evt = match md_evt {
                M::Start(t) => self.start_tag(t),
                M::End(t) => self.end_tag(t),
                M::Text(s) => {
                    self.text.push_str(&s);
                    None
                }
                M::Code(s) => {
                    if self.elem == Element::TextLyrics {
                        // Save the chord in state, dispatch previously collected span, if any
                        let evt = self.verse_or_span();
                        self.chord = Some(s.into());
                        self.chord_range = range;
                        evt
                    } else {
                        None
                    }
                }
                M::Html(_s) => None,
                M::FootnoteReference(_s) => None,
                M::SoftBreak | M::HardBreak => self.line_end(false),
                M::Rule => Some(Event::Rule),
                M::TaskListMarker(_) => None,
            };

            if evt.is_some() {
                return evt;
            }
        }
    }
}

impl<'a> Iterator for Events<'a> {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        self.next_inner().map(|evt| {
            if let Some(debug) = self.debug.as_mut() {
                let evt_s = format!("{:?}", evt).into();
                debug.evts_bard.push(evt_s);
            }
            evt
        })
    }
}

pub struct MDFile {
    content: String,
    collect_debug: bool,
}

impl MDFile {
    pub fn new<P: AsRef<Path>>(path: P, collect_debug: bool) -> io::Result<MDFile> {
        let content = fs::read_to_string(&path)?;

        Ok(MDFile {
            content,
            collect_debug,
        })
    }

    pub fn from_str(s: &str, collect_debug: bool) -> MDFile {
        MDFile {
            content: s.into(),
            collect_debug,
        }
    }

    pub fn content<'a>(&'a self) -> &'a str {
        self.content.as_str()
    }

    pub fn parse<'a>(&'a self) -> Events<'a> {
        // TODO: link callback

        let parser = md::Parser::new(&self.content).into_offset_iter();
        Events::new(parser, self.collect_debug)
    }
}


#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::*;

    /// Event reimplemented with str refs instead of boxed strings for easier
    /// testing
    #[derive(PartialEq, Eq, Debug)]
    pub enum E<'a> {
        Song(&'a str),
        Subtitle(&'a str),

        Clef {
            time: Option<Time>,
            notation: Option<Notation>,
        },
        Transpose {
            chord_set: u32,
            transpose: Transpose,
        },

        Verse {
            label: &'a str,
            chorus: bool,
        },
        Span {
            chord: &'a str,
            lyrics: &'a str,
            newline: bool,
        },

        Bullet(&'a str),
        Rule,
        Pre(&'a str),
    }

    impl<'a> From<&'a Event> for E<'a> {
        fn from(evt: &'a Event) -> Self {
            use Event::*;

            match evt {
                Song(s) => E::Song(s),
                Subtitle(s) => E::Subtitle(&*s),
                Clef { time, notation } => E::Clef {
                    time: *time,
                    notation: *notation,
                },
                Transpose {
                    chord_set,
                    transpose,
                } => E::Transpose {
                    chord_set: *chord_set,
                    transpose: *transpose,
                },
                Verse { label, chorus } => E::Verse {
                    label: &label,
                    chorus: *chorus,
                },
                Span {
                    chord,
                    lyrics,
                    newline,
                    ..
                } => E::Span {
                    chord: &chord,
                    lyrics: &lyrics,
                    newline: *newline,
                },
                Bullet(text) => E::Bullet(&text),
                Rule => E::Rule,
                Pre(text) => E::Pre(&*text),
            }
        }
    }

    impl<'a> PartialEq<&'a Event> for E<'a> {
        fn eq(&self, evt: &&'a Event) -> bool {
            let evt = Self::from(*evt);
            *self == evt
        }
    }

    impl<'a> PartialEq<E<'a>> for &Event {
        fn eq(&self, e: &E<'a>) -> bool {
            let self_e = E::from(*self);
            self_e == *e
        }
    }

    fn parse_one(s: &str) -> Event {
        let md = MDFile::from_str(s, true);
        let mut evts = md.parse();
        let evt = evts.next();
        println!("{:#?}", evts.take_debug());
        evt.unwrap()
    }

    fn parse(s: &str, evts_expected: &[E<'static>]) {
        let md = MDFile::from_str(s, true);
        let mut evts_iter = md.parse();
        let evts: Vec<_> = (&mut evts_iter).collect();
        println!("{:#?}", evts_iter.take_debug());

        for (actual, expected) in evts.iter().zip(evts_expected.iter()) {
            assert_eq!(actual, *expected);
        }

        match evts.len().cmp(&evts_expected.len()) {
            Ordering::Less => panic!("Not all expected events received"),
            Ordering::Equal => { /* evt numbers match */ }
            Ordering::Greater => panic!("Received more events than expected"),
        }
    }

    #[test]
    fn title() {
        assert_eq!(&parse_one("# Title"), E::Song("Title"));
    }

    #[test]
    fn subtitle() {
        assert_eq!(&parse_one("## Subtitle"), E::Subtitle("Subtitle"));
    }

    #[test]
    fn clef() {
        assert_eq!(&parse_one("$ 4/4 western"), E::Clef {
            time: Some((4, 4)),
            notation: Some(Notation::English)
        });
        assert_eq!(&parse_one("$ english 4/4"), E::Clef {
            time: Some((4, 4)),
            notation: Some(Notation::English)
        });
        assert_eq!(&parse_one("$ 4/4"), E::Clef {
            time: Some((4, 4)),
            notation: None
        });
        assert_eq!(&parse_one("$ german"), E::Clef {
            time: None,
            notation: Some(Notation::German)
        });

        assert_eq!(&parse_one("$ blablabla"), E::Verse {
            label: "",
            chorus: false
        });
        assert_eq!(&parse_one("*$ 4/4 western*"), E::Verse {
            label: "",
            chorus: false
        });
        assert_eq!(&parse_one("**$ 4/4 western**"), E::Verse {
            label: "",
            chorus: false
        });
        assert_eq!(&parse_one("~$ 4/4 western~"), E::Verse {
            label: "",
            chorus: false
        });
    }

    // TODO: Transpose

    #[test]
    #[rustfmt::skip]
    fn verse() {
        parse(r#"1. `C`lyrics `Am`lyrics...
2. `C`lyrics

> chorus

3. `C`lyrics
"#,
        &[
            E::Verse { label: "1.", chorus: false },
            E::Span { chord: "C", lyrics: "lyrics ", newline: true },
            E::Span { chord: "Am", lyrics: "lyrics...", newline: false },
            E::Verse { label: "2.", chorus: false },
            E::Span { chord: "C", lyrics: "lyrics", newline: true },
            E::Verse { label: "", chorus: true },
            E::Span { chord: "", lyrics: "chorus", newline: true },
            E::Verse { label: "3.", chorus: false },
            E::Span { chord: "C", lyrics: "lyrics", newline: true },
        ]);

        // Multiple choruses
        parse(r#"> 1. `C`chorus one
> 2. `F`chorus two"#,
        &[
            E::Verse { label: "1.", chorus: true },
            E::Span { chord: "C", lyrics: "chorus one", newline: true },
            E::Verse { label: "2.", chorus: true },
            E::Span { chord: "F", lyrics: "chorus two", newline: true },
        ]);

        // Custom verse label
        parse(r#"### My label
`C`lyrics
###### My label
`C`lyrics
> ### My label
`C`lyrics
"#,
        &[
            E::Verse { label: "My label", chorus: false },
            E::Span { chord: "C", lyrics: "lyrics", newline: true },
            E::Verse { label: "My label", chorus: false },
            E::Span { chord: "C", lyrics: "lyrics", newline: true },
            E::Verse { label: "My label", chorus: true },
            E::Span { chord: "C", lyrics: "lyrics", newline: true },
        ]);

        parse(r#"`D`lyrics

New verse
"#,
        &[
            E::Verse { label: "", chorus: false },
            E::Span { chord: "D", lyrics: "lyrics", newline: true },
            E::Verse { label: "", chorus: false },
            E::Span { chord: "", lyrics: "New verse", newline: true },
        ]);
    }

    #[test]
    #[rustfmt::skip]
    fn bullet() {
        parse(r#"- item one
- item two
"#,
        &[
            E::Bullet("item one"),
            E::Bullet("item two"),
        ]);

        parse(r#"- item one

- item two

- item three
"#,
        &[
            E::Bullet("item one"),
            E::Bullet("item two"),
            E::Bullet("item three"),
        ]);

        parse(r#"1. `C`lyrics
- bullet item

`C` lyrics
"#,
        &[
            E::Verse { label: "1.", chorus: false },
            E::Span { chord: "C", lyrics: "lyrics", newline: true },
            E::Bullet("bullet item"),
            E::Verse { label: "", chorus: false },
            E::Span { chord: "C", lyrics: " lyrics", newline: true },
        ]);

        // This is an oddball case, markdown considers the second line
        // a ul item continuation, it's not feasible to make it start a verse.
        parse(r#"- bullet item
some would-be lyrics
"#,
        &[
            E::Bullet("bullet item"),
            E::Bullet("some would-be lyrics"),
        ]);
    }

    #[test]
    #[rustfmt::skip]
    fn pre() {
        parse(r#"```
pre line 1
pre line 2
```"#,
        &[E::Pre("pre line 1\npre line 2\n")]);

        parse(r#"verse
```
pre line 1
pre line 2
```
verse"#,
        &[
            E::Verse { label: "", chorus: false },
            E::Span { chord: "", lyrics: "verse", newline: true },
            E::Pre("pre line 1\npre line 2\n"),
            E::Verse { label: "", chorus: false },
            E::Span { chord: "", lyrics: "verse", newline: true },
        ]);
    }
}
