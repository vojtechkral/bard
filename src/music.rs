use std::fmt;
use std::ops;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
/// Musical note notation convention
/// Variant naming follows <https://en.wikipedia.org/wiki/Musical_note#12-tone_chromatic_scale>
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
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
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

    fn as_str_western(&self, german: bool, uppercase: bool) -> &'static str {
        const TONES_UPPER: &[&str] = &[
            "C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B",
        ];
        const TONES_UPPER_G: &[&str] = &[
            "C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "B", "H",
        ];
        const TONES_LOWER: &[&str] = &[
            "c", "c#", "d", "eb", "e", "f", "f#", "g", "ab", "a", "bb", "b",
        ];
        const TONES_LOWER_G: &[&str] = &[
            "c", "c#", "d", "eb", "e", "f", "f#", "g", "ab", "a", "b", "h",
        ];

        let i = self.0 as usize;
        match (german, uppercase) {
            (false, true) => TONES_UPPER[i],
            (true, true) => TONES_UPPER_G[i],
            (false, false) => TONES_LOWER[i],
            (true, false) => TONES_LOWER_G[i],
        }
    }

    fn as_str_nashville(&self) -> &'static str {
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
    }

    fn as_str_roman(&self, uppercase: bool) -> &'static str {
        const TONES_UPPER: &[&str] = &[
            "I", "I#", "II", "IIIb", "III", "IV", "IV#", "V", "VIb", "VI", "VIIb", "VII",
        ];
        const TONES_LOWER: &[&str] = &[
            "i", "i#", "ii", "iiib", "iii", "iv", "iv#", "v", "vib", "vi", "viib", "vii",
        ];

        let i = self.0 as usize;
        if uppercase {
            TONES_UPPER[i]
        } else {
            TONES_LOWER[i]
        }
    }

    fn as_str(&self, notation: Notation, uppercase: bool) -> &'static str {
        use self::Notation::*;
        match notation {
            English => self.as_str_western(false, uppercase),
            German => self.as_str_western(true, uppercase),
            Nashville => self.as_str_nashville(),
            Roman => self.as_str_roman(uppercase),
        }
    }

    pub fn transposed<C>(self, by: C) -> Chromatic
    where
        C: Into<Chromatic>,
    {
        self + by.into()
    }
}

impl fmt::Display for Chromatic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str(Notation::English, true))
    }
}

#[derive(Debug)]
struct Chord<'s> {
    base: Chromatic,
    uppercase: bool,
    suffix: &'s str,
}

impl<'s> Chord<'s> {
    fn parse(src: &'s str, notation: Notation) -> Result<Self, &'s str> {
        let (base, base_size) = Chromatic::parse_span(src, notation).ok_or(src)?;

        Ok(Self {
            base,
            uppercase: src.chars().next().unwrap().is_uppercase(),
            suffix: &src[base_size..],
        })
    }

    fn transposed(self, by: impl Into<Chromatic>) -> Self {
        Self {
            base: self.base.transposed(by),
            uppercase: self.uppercase,
            suffix: self.suffix,
        }
    }

    fn str_len(&self, notation: Notation) -> usize {
        self.base.as_str(notation, self.uppercase).len() + self.suffix.len()
    }

    fn write_string(&self, mut to: String, notation: Notation) -> String {
        let base = self.base.as_str(notation, self.uppercase);
        to.push_str(base);
        to.push_str(self.suffix);
        to
    }
}

fn is_chord_separator(c: char) -> bool {
    match c {
        '/' | ',' | '\\' | '|' => true,
        c if c.is_whitespace() => true,
        _ => false,
    }
}

#[derive(Debug)]
struct ChordIter<'s> {
    rest: &'s str,
    notation: Notation,
}

impl<'s> ChordIter<'s> {
    fn new(src: &'s str, src_notation: Notation) -> Self {
        Self {
            rest: src,
            notation: src_notation,
        }
    }
}

impl<'s> Iterator for ChordIter<'s> {
    type Item = Result<Chord<'s>, &'s str>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }

        let mut split_found = false;
        // Find split such that multiple consecutive split chars are all
        // added as suffix to its preceiding chord.
        let split = self
            .rest
            .find(|c| {
                if !split_found {
                    split_found = is_chord_separator(c);
                    false
                } else {
                    !is_chord_separator(c)
                }
            })
            .unwrap_or(self.rest.len());

        let (next, rest) = self.rest.split_at(split);
        self.rest = rest;

        Some(Chord::parse(next, self.notation))
    }
}

pub fn transpose(
    chord_set: &str,
    by: impl Into<Chromatic>,
    src_notation: Notation,
    to_notation: Notation,
) -> Result<String, &str> {
    let by = by.into();

    // Split the leading prefix, if any, from the chord set
    let prefix_at = chord_set
        .find(|c: char| !is_chord_separator(c))
        .unwrap_or(0);
    let (prefix, rest) = chord_set.split_at(prefix_at);

    // Compute the resulting string's length
    let mut transposed_len = prefix.len();
    for chord in ChordIter::new(rest, src_notation) {
        transposed_len += chord?.transposed(by).str_len(to_notation);
    }

    // Render the resulting string
    let mut res = String::with_capacity(transposed_len);
    res.push_str(prefix);
    Ok(ChordIter::new(rest, src_notation).fold(res, |res, chord| {
        chord.unwrap().transposed(by).write_string(res, to_notation)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    use Notation::*;

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
        let expected = vec![English, English, English, German, Nashville, Roman];

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
        let c = Chromatic::parse("C", English).unwrap();
        assert_eq!(format!("{}", c), "C");

        assert_eq!(Chromatic::parse("", English), None);
        assert_eq!(Chromatic::parse("", German), None);
        assert_eq!(Chromatic::parse("", Nashville), None);
        assert_eq!(Chromatic::parse("", Roman), None);
    }

    #[test]
    fn chromatic_transposition() {
        let c: Chromatic = 0.into();
        let transposed = c.transposed(-1);
        assert_eq!(transposed.as_str(German, false), "h");

        let transposed = c.transposed(3);
        assert_eq!(transposed.as_str(German, true), "Eb");
    }

    #[test]
    fn chromatic_english() {
        let c: Chromatic = 0.into();
        let fsharp: Chromatic = 6.into();
        assert_eq!(Chromatic::parse("C", English).unwrap(), c);
        assert_eq!(Chromatic::parse("F#", English).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("F♯", English).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("Gb", English).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("G♭", English).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("X", English), None);
    }

    #[test]
    fn chromatic_german() {
        let c: Chromatic = 0.into();
        let fsharp: Chromatic = 6.into();
        assert_eq!(Chromatic::parse("C", German).unwrap(), c);
        assert_eq!(
            Chromatic::parse("H", German).unwrap(),
            Chromatic::parse("B", English).unwrap()
        );
        assert_eq!(
            Chromatic::parse("B", German).unwrap(),
            Chromatic::parse("Bb", English).unwrap()
        );
        assert_eq!(Chromatic::parse("F#", German).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("F♯", German).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("Gb", German).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("G♭", German).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("X", German), None);
    }

    #[test]
    fn chromatic_nashville() {
        let c: Chromatic = 0.into();
        let fsharp: Chromatic = 6.into();
        assert_eq!(Chromatic::parse("1", Nashville).unwrap(), c);
        assert_eq!(
            Chromatic::parse("2", Nashville).unwrap(),
            Chromatic::parse("D", German).unwrap()
        );
        assert_eq!(Chromatic::parse("4#", Nashville).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("4♯", Nashville).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("5b", Nashville).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("5♭", Nashville).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("0", Nashville), None);
        assert_eq!(Chromatic::parse("8", Nashville), None);
        assert_eq!(Chromatic::parse("X", Nashville), None);
    }

    #[test]
    fn chromatic_roman() {
        let c: Chromatic = 0.into();
        let fsharp: Chromatic = 6.into();
        assert_eq!(Chromatic::parse("I", Roman).unwrap(), c);
        assert_eq!(
            Chromatic::parse("II", Roman).unwrap(),
            Chromatic::parse("D", German).unwrap()
        );
        assert_eq!(Chromatic::parse("IV#", Roman).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("IV♯", Roman).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("Vb", Roman).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("V♭", Roman).unwrap(), fsharp);
        assert_eq!(Chromatic::parse("C", Roman), None);
        assert_eq!(Chromatic::parse("X", Roman), None);
    }

    #[test]
    fn chromatic_span() {
        assert_eq!(Chromatic::parse_span("A", English).unwrap().1, 1);
        assert_eq!(Chromatic::parse_span("D#", English).unwrap().1, 2);
        assert_eq!(Chromatic::parse_span("H#", German).unwrap().1, 2);
        assert_eq!(Chromatic::parse_span("1#", Nashville).unwrap().1, 2);
    }

    #[test]
    fn transpose_basic() {
        let t = transpose("C", 2, English, English).unwrap();
        assert_eq!(t, "D");
    }

    #[test]
    fn transpose_multiple() {
        let t = transpose("C/D,E", 2, English, English).unwrap();
        assert_eq!(t, "D/E,F#");

        let t = transpose("C / D , E", 2, English, English).unwrap();
        assert_eq!(t, "D / E , F#");
    }

    #[test]
    fn transpose_suffixes() {
        let t = transpose("Cm/D°,Emaj7", 2, English, English).unwrap();
        assert_eq!(t, "Dm/E°,F#maj7");
    }

    #[test]
    fn transpose_multiple_separators() {
        let t = transpose("C/|\\/D,,   ,,E,,,", 2, English, English).unwrap();
        assert_eq!(t, "D/|\\/E,,   ,,F#,,,");
    }

    #[test]
    fn transpose_leading_separators() {
        let t = transpose(",C", 2, English, English).unwrap();
        assert_eq!(t, ",D");
    }

    #[test]
    fn transpose_whitespace() {
        let t = transpose("   /C  ", 2, English, English).unwrap();
        assert_eq!(t, "   /D  ");
    }

    #[test]
    fn transpose_german() {
        let t = transpose("H/B", 0, German, English).unwrap();
        assert_eq!(t, "B/Bb");
    }

    #[test]
    fn transpose_roman() {
        let t = transpose("C/D,E", 5, English, Roman).unwrap();
        assert_eq!(t, "IV/V,VI");

        let t = transpose("C/D,E", 5, English, Roman).unwrap();
        assert_eq!(t, "IV/V,VI");
    }

    #[test]
    fn transpose_nashville() {
        let t = transpose("I/II,III", 0, Roman, Nashville).unwrap();
        assert_eq!(t, "1/2,3");
    }

    #[test]
    fn transpose_lowercase() {
        let t = transpose("c", 2, English, Roman).unwrap();
        assert_eq!(t, "ii");

        let t = transpose("c,d,e,", 2, English, Roman).unwrap();
        assert_eq!(t, "ii,iii,iv#,");
    }
}
