//! A parser for HTML fragments embdedded in MD based on `html5ever`.
//!
//! We're actually using the `Tokenizer` from `html5ever` rather than the
//! parser per se, since we really just need to extract the raw start/end
//! tags and which map directly to Hbs inline calls in the template.
//!
//! The HTML isn't validate at all, not even matching of tags,
//! the tags are really just a way to call inlines.

use html5ever::buffer_queue::BufferQueue;
use html5ever::tokenizer::{
    Tag, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts, TokenizerResult,
};

use super::utf8;
use crate::book::{HtmlTag, Inline};
use crate::util::BStr;

/// An intermediate inline, like `Inline`,
/// but only subset that can be contained in HTML blocks
/// and growable (for appending from tokenizer).
enum SemiInline {
    Text(String),
    Break,
    HtmlTag(HtmlTag),
}

impl From<SemiInline> for Inline {
    fn from(semi: SemiInline) -> Self {
        match semi {
            SemiInline::Text(s) => Inline::text(s),
            SemiInline::Break => Inline::Break,
            SemiInline::HtmlTag(t) => Inline::HtmlTag(t),
        }
    }
}

struct Sink {
    inlines: Vec<SemiInline>,
}

impl Sink {
    fn new() -> Self {
        Self { inlines: vec![] }
    }

    fn append_tag(&mut self, tag: Tag) {
        let name: BStr = match (tag.kind, tag.self_closing) {
            (TagKind::StartTag, false) => tag.name.to_string(),
            (TagKind::StartTag, true) => format!("{}/", tag.name),
            (TagKind::EndTag, _) => format!("/{}", tag.name),
        }
        .into();

        let attrs = tag
            .attrs
            .iter()
            .map(|attr| {
                let name: BStr = attr.name.local.to_string().into();
                let value: BStr = attr.value.to_string().into();
                (name, value)
            })
            .collect();

        let tag = HtmlTag { name, attrs };
        self.inlines.push(SemiInline::HtmlTag(tag));
    }

    fn append_text(&mut self, text: &str) {
        // Nb. The HTML parse doesn't emit \r (even if in input).
        // This is covered by a parser test.

        let mut split = text.split('\n');

        let first = split.next().unwrap(); // There will always be at least the prefix
        if !first.is_empty() {
            if let Some(SemiInline::Text(s)) = self.inlines.last_mut() {
                s.push_str(first);
            } else {
                self.inlines.push(SemiInline::Text(first.to_string()));
            }
        }

        for s in split {
            self.inlines.push(SemiInline::Break);
            if !s.is_empty() {
                self.inlines.push(SemiInline::Text(s.to_string()));
            }
        }
    }

    fn finalize(self, target: &mut Vec<Inline>) {
        let Self { mut inlines } = self;

        // There's typically a trailing break returned by the parse,
        // remove that:
        if let Some(SemiInline::Break) = inlines.last() {
            inlines.pop();
        }

        for semi in inlines.drain(..) {
            target.push(semi.into())
        }
    }
}

impl TokenSink for Sink {
    type Handle = ();

    fn process_token(&mut self, token: Token, _line_num: u64) -> TokenSinkResult<Self::Handle> {
        match token {
            Token::TagToken(tag) => self.append_tag(tag),
            Token::CharacterTokens(s) => self.append_text(&*s),

            Token::NullCharacterToken => {
                panic!("Control characters should not have been left in input.")
            }

            // These are simply ignored:
            Token::CommentToken(_)
            | Token::DoctypeToken(_)
            | Token::EOFToken
            | Token::ParseError(_) => {}
        }

        TokenSinkResult::Continue
    }
}

pub fn parse_html(html: &[u8], target: &mut Vec<Inline>) {
    let sink = Sink::new();
    let mut tokenizer = Tokenizer::new(sink, TokenizerOpts::default());
    let mut queue = BufferQueue::new();

    queue.push_back(utf8(html).into());
    loop {
        if let TokenizerResult::Done = tokenizer.feed(&mut queue) {
            break;
        }
    }

    tokenizer.end();
    tokenizer.sink.finalize(target);
}
