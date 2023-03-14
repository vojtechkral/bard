//! Defines how AST serializes into XML, see `RXml` for the rest of XML output definition.

use crate::util::xml_support::*;
use crate::xml_write;

use super::*;

xml_write!(struct Chord {
    chord,
    alt_chord,
    backticks,
    baseline,
    inlines,
} -> |w| {
    w.tag("chord")
        .attr(chord)
        .attr_opt("alt-chord", alt_chord.unwrap())
        .attr(backticks)
        .attr(baseline)
        .content()?
        .many(inlines)?
});

xml_write!(struct Link {
    url,
    title,
    text,
} -> |w| {
    w.tag("link")
        .attr(url)
        .attr(title)
        .content()?
        .text(text)?
});

xml_write!(struct Image {
    path,
    title,
    class,
    width,
    height,
    full_path,
} -> |w| {
    let _ = full_path;
    w.tag("image",)
        .attr(path)
        .attr(title)
        .attr(class)
        .attr(width)
        .attr(height)
});

xml_write!(struct ChorusRef {
    num,
    prefix_space,
} -> |w| {
    w.tag("chorus-ref")
        .attr_opt("num", &num.unwrap().map(|n| format!("{}", n)))
        .attr(prefix_space)
});

xml_write!(struct HtmlTag {
    name,
    attrs,
} -> |w| {
    let tag = w.tag("tag").attr(name);
    let attrs = attrs.unwrap();
    if attrs.is_empty() {
        return tag.finish();
    } else {
        tag.content()?.value(attrs)?
    }
});

xml_write!(enum Inline |w| {
    Text { text } => { w.write_text(text)?; },
    Chord(c) => { w.write_value(c)?; },
    Break => { w.tag("br").finish()?; },
    Emph(i) => { w.tag("emph").content()?.many(i)?.finish()?; },
    Strong(i) => { w.tag("strong").content()?.many(i)?.finish()?; },
    Link(l) => { w.write_value(l)?; },
    Image(i) => { w.write_value(i)?; },
    ChorusRef(cr) => { w.write_value(cr)?; },
    HtmlTag(tag) => { w.write_value(tag)?; },

    Transpose(..) => { unreachable!() },
});

xml_write!(struct Verse {
    label,
    paragraphs,
} -> |w| {
    use VerseLabel::*;
    let label = label.unwrap();
    let label_type = match label {
        Verse(..) => "verse",
        Chorus(..) => "chorus",
        Custom(..) => "custom",
        None {} => "none",
    };

    let label = match label {
        Verse(n) | Chorus(Some(n)) => Some(format!("{}", n)),
        Custom(s) => Some(s.to_string()),
        _ => Option::None,
    };

    w.tag("verse")
        .attr(("label-type", label_type))
        .attr_opt("label", &label)
        .content()?
        .many_tags("p", paragraphs)?
});

xml_write!(struct BulletList { items, } -> |w| {
    w.tag("bullet-list").content()?.many_tags("item", items)?
});

xml_write!(enum Block |w| {
    Verse(verse) => { w.write_value(verse)?; },
    BulletList(l) => { w.write_value(l)?; },
    HorizontalLine => { w.tag("hr").finish()?; },
    Pre { text } => { w.tag("pre").content()?.text(text)?.finish()?; },
    HtmlBlock(i) => { w.tag("html-block").content()?.many(i)?.finish()?; },
});

xml_write!(struct Song {
    title,
    subtitles,
    blocks,
    notation,
} -> |w| {
    w.tag("song")
        .attr(title)
        .attr(notation)
        .content()?
        .many_tags("subtitle", subtitles)?
        .many(blocks)?
});

xml_write!(struct SongRef {
    title,
    idx,
} -> |w| {
    w.tag("song-ref")
        .attr(title)
        .attr(idx)
});
