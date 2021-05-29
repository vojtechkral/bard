use std::fmt;
use std::ops;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub type Time = (u32, u32);

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
/// Musical note notation convention
/// Variant naming follows https://en.wikipedia.org/wiki/Musical_note#12-tone_chromatic_scale
pub enum Notation {
    #[serde(alias = "western")]
    #[serde(alias = "dutch")]
    English,
    #[serde(alias = "czech")]
    German,
    Nashville,
    Roman,
}

impl Default for Notation {
    fn default() -> Notation {
        Notation::English
    }
}

impl FromStr for Notation {
    type Err = ();

    fn from_str(s: &str) -> Result<Notation, ()> {
        use self::Notation::*;

        let lower = s.to_ascii_lowercase();
        match lower.as_str() {
            "english" | "western" | "dutch" => Ok(English),
            "german" | "czech" => Ok(German),
            "nashville" => Ok(Nashville),
            "roman" => Ok(Roman),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Notation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Notation::English => "english",
            Notation::German => "german",
            Notation::Nashville => "nashville",
            Notation::Roman => "roman",
        };
        write!(f, "{}", name)
    }
}

/// Represents a half-tone in a 12-tone chromatic scale in equal temperament
/// tuning, starting from C (ie. C = 0, C# = 1, ...)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Chromatic(u8);

macro_rules! impl_from {
    (u $t:ty) => {
        impl From<$t> for Chromatic {
            fn from(u: $t) -> Chromatic {
                Chromatic((u % 12) as u8)
            }
        }
    };
    (i $t:ty) => {
        impl From<$t> for Chromatic {
            fn from(i: $t) -> Chromatic {
                Chromatic(((i % 12 + 12) % 12) as u8)
            }
        }
    };
}

macro_rules! impl_into {
    ($t:ty) => {
        impl From<Chromatic> for $t {
            fn from(c: Chromatic) -> $t {
                c.0 as _
            }
        }
    };
}

impl_from!(u u8);
impl_from!(u u16);
impl_from!(u u32);
impl_from!(u u64);
impl_from!(u usize);
impl_from!(i i8);
impl_from!(i i16);
impl_from!(i i32);
impl_from!(i i64);
impl_from!(i isize);
impl_into!(u8);
impl_into!(u16);
impl_into!(u32);
impl_into!(u64);
impl_into!(usize);
impl_into!(i8);
impl_into!(i16);
impl_into!(i32);
impl_into!(i64);
impl_into!(isize);

impl FromStr for Chromatic {
    type Err = ();

    fn from_str(s: &str) -> Result<Chromatic, ()> {
        match i32::from_str(s) {
            Ok(i) => Ok(i.into()),
            Err(_) => Err(()),
        }
    }
}

impl ops::Add for Chromatic {
    type Output = Chromatic;
    fn add(self, other: Chromatic) -> Chromatic {
        Chromatic((self.0 + other.0) % 12)
    }
}

impl ops::AddAssign for Chromatic {
    fn add_assign(&mut self, other: Chromatic) {
        *self = *self + other;
    }
}

impl ops::Sub for Chromatic {
    type Output = Chromatic;
    fn sub(self, other: Chromatic) -> Chromatic {
        Chromatic((12 + self.0 - other.0) % 12)
    }
}

impl ops::SubAssign for Chromatic {
    fn sub_assign(&mut self, other: Chromatic) {
        *self = *self - other;
    }
}

impl Chromatic {
    pub fn new(i: i32) -> Chromatic {
        i.into()
    }

    pub fn num(&self) -> u8 {
        self.0
    }

    fn parse_halftone(from: &str, base: Chromatic, base_size: usize) -> (Chromatic, usize) {
        let c = from[base_size..].chars().next();
        let (delta, size) = match c {
            Some('b') => (-1, 1),
            Some('♭') => (-1, '♭'.len_utf8()),
            Some('#') => (1, 1),
            Some('♯') => (1, '♯'.len_utf8()),
            Some(_) | None => (0, 0),
        };

        (base + delta.into(), base_size + size)
    }

    fn parse_western(from: &str, german: bool) -> Option<(Chromatic, usize)> {
        let base = match from.chars().next().unwrap() {
            'C' | 'c' => 0,
            'D' | 'd' => 2,
            'E' | 'e' => 4,
            'F' | 'f' => 5,
            'G' | 'g' => 7,
            'A' | 'a' => 9,
            'B' | 'b' if !german => 11,
            'B' | 'b' if german => 10,
            'H' | 'h' if german => 11,
            _ => return None,
        };

        Some(Self::parse_halftone(from, base.into(), 1))
    }

    fn parse_nashvile(from: &str) -> Option<(Chromatic, usize)> {
        let base = match from.chars().next().unwrap() {
            '1' => 0,
            '2' => 2,
            '3' => 4,
            '4' => 5,
            '5' => 7,
            '6' => 9,
            '7' => 11,
            _ => return None,
        };

        Some(Self::parse_halftone(from, base.into(), 1))
    }

    fn parse_roman(from: &str) -> Option<(Chromatic, usize)> {
        let mut chars = from.chars();
        let c1 = chars.next().map(|c| c.to_ascii_uppercase()).unwrap();
        let c2 = chars.next().map(|c| c.to_ascii_uppercase());
        let c3 = chars.next().map(|c| c.to_ascii_uppercase());

        let (base, base_size) = match (c1, c2, c3) {
            ('I', Some('I'), Some('I')) => (4, 3),
            ('V', Some('I'), Some('I')) => (11, 3),
            ('I', Some('I'), _) => (2, 2),
            ('I', Some('V'), _) => (5, 2),
            ('V', Some('I'), _) => (9, 2),
            ('I', _, _) => (0, 1),
            ('V', _, _) => (7, 1),
            _ => return None,
        };

        Some(Self::parse_halftone(from, base.into(), base_size))
    }

    /// Parses a chromatic from a start of a string, which may continue with
    /// arbitrary other characters. Returns the `Chromatic` parsed and
    /// the size of the chromatic part in bytes.
    pub fn parse_span(from: &str, notation: Notation) -> Option<(Chromatic, usize)> {
        use self::Notation::*;

        if from.is_empty() {
            None
        } else {
            match notation {
                English => Self::parse_western(from, false),
                German => Self::parse_western(from, true),
                Nashville => Self::parse_nashvile(from),
                Roman => Self::parse_roman(from),
            }
        }
    }

    pub fn parse(from: &str, notation: Notation) -> Option<Chromatic> {
        Self::parse_span(from, notation).map(|(chromatic, _)| chromatic)
    }

    fn to_string_western(&self, german: bool, uppercase: bool) -> String {
        let res = match self.0 {
            0 => "C",
            1 => "C#",
            2 => "D",
            3 => "Eb",
            4 => "E",
            5 => "F",
            6 => "F#",
            7 => "G",
            8 => "Ab",
            9 => "A",
            10 if german => "B",
            10 if !german => "Bb",
            11 if german => "H",
            11 if !german => "B",
            _ => unreachable!(),
        };

        if uppercase {
            res.into()
        } else {
            res.to_lowercase()
        }
    }

    fn to_string_nashville(&self) -> String {
        match self.0 {
            0 => "1",
            1 => "1#",
            2 => "2",
            3 => "3b",
            4 => "3",
            5 => "4",
            6 => "4#",
            7 => "5",
            8 => "6b",
            9 => "6",
            10 => "7b",
            11 => "7",
            _ => unreachable!(),
        }
        .into()
    }

    fn to_string_roman(&self, uppercase: bool) -> String {
        let res = match self.0 {
            0 => "I",
            1 => "I#",
            2 => "II",
            3 => "IIIb",
            4 => "III",
            5 => "IV",
            6 => "IV#",
            7 => "V",
            8 => "VIb",
            9 => "VI",
            10 => "VIIb",
            11 => "VII",
            _ => unreachable!(),
        };

        if uppercase {
            res.into()
        } else {
            res.to_lowercase()
        }
    }

    pub fn to_string(&self, notation: Notation, uppercase: bool) -> String {
        use self::Notation::*;
        match notation {
            English => self.to_string_western(false, uppercase),
            German => self.to_string_western(true, uppercase),
            Nashville => self.to_string_nashville(),
            Roman => self.to_string_roman(uppercase),
        }
    }

    pub fn transposed<C>(self, by: C) -> Chromatic
    where
        C: Into<Chromatic>,
    {
        self + by.into()
    }
}

impl Default for Chromatic {
    fn default() -> Chromatic {
        Chromatic(0)
    }
}

impl fmt::Display for Chromatic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string(Notation::English, true))
    }
}

#[derive(Clone, Debug)]
pub struct Chord {
    base: Chromatic,
    uppercase: bool,
    suffix: String,
    /// Linked-list of more chords, used for slash chords etc.:
    next: Option<Box<Chord>>,
}

impl Chord {
    fn is_separator(c: char) -> bool {
        match c {
            '/' | ',' | '\\' | '|' => true,
            c if c.is_whitespace() => true,
            _ => false,
        }
    }

    /// Parse one chord from the input, return the `Chord` and its size (in
    /// bytes) til separator position (if any). Returns `None` if the chord
    /// can't be parsed.
    fn parse_one(from: &str, notation: Notation) -> Option<(Chord, usize)> {
        if from.is_empty() {
            return None;
        }

        let (base, base_size) = Chromatic::parse_span(from, notation)?;
        let first_char = from.chars().next().unwrap();
        let uppercase = first_char.is_uppercase() || first_char.is_numeric();

        let mut sep_found = false;
        let len = from.len();
        let end = from[base_size..]
            .char_indices()
            .find(|(_, c)| {
                let is_sep = Self::is_separator(*c);
                if !is_sep && sep_found {
                    return true;
                }
                sep_found = is_sep;
                false
            })
            .map(|(idx, _)| idx + base_size)
            .unwrap_or(from.len());

        let chord = Chord {
            base,
            uppercase,
            suffix: from.get(base_size..end).unwrap_or("").into(),
            next: None,
        };

        Some((chord, end))
    }

    fn append(&mut self, chord: Self) {
        let mut this = self;
        while this.next.is_some() {
            this = this.next.as_mut().unwrap();
        }

        this.next = Some(Box::new(chord));
    }

    pub fn parse(from: &str, notation: Notation) -> Option<Chord> {
        let (mut first, mut index) = Self::parse_one(from, notation)?;

        while index < from.len() {
            let next = Self::parse_one(&from[index..], notation)?;
            index += next.1;
            first.append(next.0);
        }

        Some(first)
    }

    fn to_string_inner(&self, res: &mut String, notation: Notation) {
        let base = self.base.to_string(notation, self.uppercase);
        res.push_str(&base);
        res.push_str(&self.suffix);
    }

    pub fn to_string(&self, notation: Notation) -> String {
        // Try to target the typical case:
        let mut res = String::with_capacity(self.suffix.len() + 5);

        let mut this = self;
        this.to_string_inner(&mut res, notation);
        while let Some(next) = this.next.as_ref() {
            this = next;
            this.to_string_inner(&mut res, notation);
        }

        res
    }

    pub fn transpose<C>(&mut self, by: C)
    where
        C: Into<Chromatic>,
    {
        let by = by.into();

        let mut this = self;
        this.base += by;
        while this.next.is_some() {
            this = this.next.as_mut().unwrap();
            this.base += by;
        }
    }

    pub fn transposed<C>(&self, by: C) -> Chord
    where
        C: Into<Chromatic>,
    {
        let by = by.into();
        let mut res = self.clone();
        res.transpose(by);
        res
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string(Notation::English))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notation_parse() {
        let names = [
            "english",
            "western",
            "dutch",
            "german",
            "nashville",
            "roman",
        ];
        let expected = vec![
            Notation::English,
            Notation::English,
            Notation::English,
            Notation::German,
            Notation::Nashville,
            Notation::Roman,
        ];

        // Test from_str:
        let parsed: Vec<_> = names
            .iter()
            .map(|s| Notation::from_str(*s).unwrap())
            .collect();
        assert_eq!(&parsed, &expected);

        // Test serde deserialization:
        let mut json: String = "[".to_owned();
        for name in &names {
            json.push('"');
            json.push_str(name);
            json.push_str("\",");
        }
        json.pop(); // Remove the last comma
        json.push(']');

        let parsed: Vec<Notation> = serde_json::from_str(&json).unwrap();
        assert_eq!(&parsed, &expected);

        // Test serde serialization:
        let json_expected = r#"["english","english","english","german","nashville","roman"]"#;
        let serialized = serde_json::to_string(&parsed).unwrap();
        assert_eq!(serialized, json_expected);
    }

    #[test]
    fn chromatic_basic() {
        let c = Chromatic::parse("C", Notation::English).unwrap();
        assert_eq!(format!("{}", c), "C");

        assert_eq!(Chromatic::parse("", Notation::English), None);
        assert_eq!(Chromatic::parse("", Notation::German), None);
        assert_eq!(Chromatic::parse("", Notation::Nashville), None);
        assert_eq!(Chromatic::parse("", Notation::Roman), None);
    }

    #[test]
    fn chromatic_transposition() {
        let c: Chromatic = 0.into();
        let transposed = c.transposed(-1);
        assert_eq!(transposed.to_string(Notation::German, false), "h");

        let transposed = c.transposed(3);
        assert_eq!(transposed.to_string(Notation::German, true), "Eb");
    }

    #[test]
    fn chromatic_english() {
        let c: Chromatic = 0.into();
        let fsharp: Chromatic = 6.into();
        assert_eq!(Chromatic::parse("C", Notation::English).unwrap(), c);
        assert_eq!(Chromatic::parse("F#", Notation::English).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("F♯", Notation::English).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("Gb", Notation::English).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("G♭", Notation::English).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("X", Notation::English), None);
    }

    #[test]
    fn chromatic_german() {
        let c: Chromatic = 0.into();
        let fsharp: Chromatic = 6.into();
        assert_eq!(Chromatic::parse("C", Notation::German).unwrap(), c);
        assert_eq!(
            Chromatic::parse("H", Notation::German).unwrap(),
            Chromatic::parse("B", Notation::English).unwrap()
        );
        assert_eq!(
            Chromatic::parse("B", Notation::German).unwrap(),
            Chromatic::parse("Bb", Notation::English).unwrap()
        );
        assert_eq!(Chromatic::parse("F#", Notation::German).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("F♯", Notation::German).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("Gb", Notation::German).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("G♭", Notation::German).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("X", Notation::German), None);
    }

    #[test]
    fn chromatic_nashville() {
        let c: Chromatic = 0.into();
        let fsharp: Chromatic = 6.into();
        assert_eq!(Chromatic::parse("1", Notation::Nashville).unwrap(), c);
        assert_eq!(
            Chromatic::parse("2", Notation::Nashville).unwrap(),
            Chromatic::parse("D", Notation::German).unwrap()
        );
        assert_eq!(Chromatic::parse("4#", Notation::Nashville).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("4♯", Notation::Nashville).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("5b", Notation::Nashville).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("5♭", Notation::Nashville).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("0", Notation::Nashville), None);
        assert_eq!(Chromatic::parse("8", Notation::Nashville), None);
        assert_eq!(Chromatic::parse("X", Notation::Nashville), None);
    }

    #[test]
    fn chromatic_roman() {
        let c: Chromatic = 0.into();
        let fsharp: Chromatic = 6.into();
        assert_eq!(Chromatic::parse("I", Notation::Roman).unwrap(), c);
        assert_eq!(
            Chromatic::parse("II", Notation::Roman).unwrap(),
            Chromatic::parse("D", Notation::German).unwrap()
        );
        assert_eq!(Chromatic::parse("IV#", Notation::Roman).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("IV♯", Notation::Roman).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("Vb", Notation::Roman).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("V♭", Notation::Roman).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("C", Notation::Roman), None);
        assert_eq!(Chromatic::parse("X", Notation::Roman), None);
    }

    #[test]
    fn chromatic_span() {
        assert_eq!(Chromatic::parse_span("A", Notation::English).unwrap().1, 1);
        assert_eq!(Chromatic::parse_span("D#", Notation::English).unwrap().1, 2);
        assert_eq!(Chromatic::parse_span("H#", Notation::German).unwrap().1, 2);
        assert_eq!(
            Chromatic::parse_span("1#", Notation::Nashville).unwrap().1,
            2
        );
    }

    #[test]
    fn chord_basic() {
        let chord = Chord::parse("C", Notation::English).unwrap();
        assert_eq!(format!("{}", chord), "C");
    }

    #[test]
    fn chord_notations() {
        let chord = Chord::parse("bb", Notation::English).unwrap();
        assert_eq!(chord.to_string(Notation::German), "b");

        let chord = Chord::parse("F#mi", Notation::English).unwrap();
        assert_eq!(format!("{}", chord), "F#mi");

        let chord = Chord::parse("1°", Notation::Nashville).unwrap();
        assert_eq!(format!("{}", chord), "C°");
        assert_eq!(chord.to_string(Notation::Nashville), "1°");

        let chord = Chord::parse("ivm", Notation::Roman).unwrap();
        assert_eq!(format!("{}", chord), "fm");
        assert_eq!(chord.to_string(Notation::Roman), "ivm");

        // Slash chords:
        let chord = Chord::parse("B / A", Notation::German).unwrap();
        assert_eq!(chord.to_string(Notation::English), "Bb / A");

        let chord = Chord::parse("I/IV7/V°", Notation::Roman).unwrap();
        assert_eq!(chord.to_string(Notation::English), "C/F7/G°");
    }

    #[test]
    fn chord_transposition() {
        let chord = Chord::parse("F#mi", Notation::English).unwrap();
        let transposed = chord.transposed(1);
        assert_eq!(format!("{}", transposed), "Gmi");

        // Slash chords:
        let chord = Chord::parse("F / C", Notation::English).unwrap();
        let transposed = chord.transposed(1);
        assert_eq!(format!("{}", transposed), "F# / C#");

        let chord = Chord::parse("I,IV7|V°", Notation::Roman).unwrap();
        let transposed = chord.transposed(14);
        assert_eq!(format!("{}", transposed), "D,G7|A°");
    }
}
