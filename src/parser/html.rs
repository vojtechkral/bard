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

use super::{DiagKind, ParserCtx};
use crate::book::{HtmlTag, Inline};
use crate::util::BStr;

pub static RESERVED_TAGS: &[&str] = &["html", "tex"];

struct Sink<'c> {
    inlines: Vec<HtmlTag>,
    start_line: usize,
    text_buffer: String,
    text_start_line: usize,
    ctx: &'c ParserCtx<'c>,
}

impl<'c> Sink<'c> {
    fn new(start_line: usize, ctx: &'c ParserCtx<'c>) -> Self {
        Self {
            inlines: vec![],
            start_line,
            text_buffer: String::new(),
            text_start_line: 0,
            ctx,
        }
    }

    fn append_tag(&mut self, tag: Tag, line_num: u64) {
        if RESERVED_TAGS.contains(&tag.name.to_ascii_lowercase().as_ref()) {
            self.ctx.report_diag(
                line_num as _,
                DiagKind::HtmlReservedTag {
                    tag: tag.name.to_string().into(),
                },
            );
            return;
        }

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
        self.inlines.push(tag);
    }

    fn append_text(&mut self, text: &str, line_num: usize) {
        // Text within HTML blocks is ignored, but it is accumulated here
        // so that a warning can be emitted.

        let text = text.trim();
        if text.is_empty() {
            return;
        }

        if self.text_buffer.is_empty() {
            self.text_start_line = line_num;
        }
        self.text_buffer.push_str(text);
    }

    fn ignored_text_warn(&mut self) {
        if self.text_buffer.is_empty() {
            return;
        }

        let line = self.start_line + self.text_start_line - 1; // -1 because both are 1-indexed
        self.ctx
            .report_diag(line, DiagKind::html_ignored_text(&self.text_buffer));
        self.text_buffer.clear();
    }

    fn finalize(mut self, target: &mut Vec<Inline>) {
        self.ignored_text_warn();
        target.reserve(self.inlines.len());
        target.extend(self.inlines.drain(..).map(Inline::HtmlTag));
    }
}

impl<'d> TokenSink for Sink<'d> {
    type Handle = ();

    fn process_token(&mut self, token: Token, line_num: u64) -> TokenSinkResult<Self::Handle> {
        if !matches!(&token, Token::CharacterTokens(_)) {
            self.ignored_text_warn();
        }

        match token {
            Token::TagToken(tag) => self.append_tag(tag, line_num),
            Token::CharacterTokens(s) => self.append_text(&s, line_num as _),

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

pub(super) fn parse_html(html: &str, target: &mut Vec<Inline>, start_line: usize, ctx: &ParserCtx) {
    let sink = Sink::new(start_line, ctx);
    let mut tokenizer = Tokenizer::new(sink, TokenizerOpts::default());
    let mut queue = BufferQueue::new();

    queue.push_back(html.into());
    loop {
        if let TokenizerResult::Done = tokenizer.feed(&mut queue) {
            break;
        }
    }

    tokenizer.end();
    tokenizer.sink.finalize(target);
}
