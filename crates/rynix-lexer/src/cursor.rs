//! The zero-allocation lexer core (ADR-0004).
//!
//! [`Lexer`] walks the raw bytes of a source file and yields [`Token`]s on
//! demand. It never allocates and never fails: any input produces a token
//! stream that tiles the input byte-exactly, with structured diagnostics
//! attached to recovery tokens.

mod number;
mod string;

use rynix_diag::DiagSink;
use rynix_span::{SourceFile, Span};

use crate::errors;
use crate::token::{Token, TokenKind, keyword_kind};

/// First-byte classification. Dispatching through a 256-entry table keeps
/// the hot loop to one indexed load plus one jump.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Class {
    /// Not the start of any token (`$`, `@`, `;`, `&`, control bytes, ...).
    Unknown,
    /// Space or tab.
    Space,
    /// `\n` or `\r`.
    Newline,
    /// ASCII letter or `_`.
    Ident,
    /// ASCII digit.
    Digit,
    /// `"`.
    Quote,
    /// `#`.
    Hash,
    /// A byte that starts real punctuation (including `!` for `!=`).
    Punct,
    /// Any byte >= 0x80: a UTF-8 lead or continuation byte.
    NonAscii,
}

const CLASS: [Class; 256] = build_class_table();

const fn build_class_table() -> [Class; 256] {
    let mut t = [Class::Unknown; 256];
    let mut b = 0usize;
    while b < 256 {
        let byte = b as u8;
        t[b] = if byte >= 0x80 {
            Class::NonAscii
        } else if byte.is_ascii_alphabetic() || byte == b'_' {
            Class::Ident
        } else if byte.is_ascii_digit() {
            Class::Digit
        } else {
            match byte {
                b' ' | b'\t' => Class::Space,
                b'\n' | b'\r' => Class::Newline,
                b'"' => Class::Quote,
                b'#' => Class::Hash,
                b'(' | b')' | b'[' | b']' | b'{' | b'}' | b',' | b'.' | b':' | b'=' | b'<'
                | b'>' | b'+' | b'-' | b'*' | b'/' | b'%' | b'!' => Class::Punct,
                _ => Class::Unknown,
            }
        };
        b += 1;
    }
    t
}

#[inline]
const fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[inline]
const fn is_digit_radix(b: u8, radix: u32) -> bool {
    match radix {
        2 => b == b'0' || b == b'1',
        8 => b.is_ascii_digit() && b <= b'7',
        16 => b.is_ascii_hexdigit(),
        _ => b.is_ascii_digit(),
    }
}

/// Byte length of the UTF-8 character starting with `b`.
#[inline]
const fn utf8_len(b: u8) -> u32 {
    match b {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xFF => 4,
        // ASCII, or a continuation byte: the latter cannot occur at a
        // character boundary in valid UTF-8, but stay total and advance one.
        _ => 1,
    }
}

/// A lazy, allocation-free lexer over one source file.
pub struct Lexer<'src> {
    src: &'src [u8],
    /// Global offset of `src[0]` (see `SourceMap` / ADR-0003).
    base: u32,
    /// Current file-local offset.
    pos: u32,
}

impl<'src> Lexer<'src> {
    /// Creates a lexer over `src`, whose first byte lives at global offset
    /// `base`.
    pub fn new(src: &'src str, base: u32) -> Self {
        Lexer {
            src: src.as_bytes(),
            base,
            pos: 0,
        }
    }

    /// Creates a lexer over a file loaded into a
    /// [`SourceMap`](rynix_span::SourceMap).
    pub fn from_file(file: &'src SourceFile) -> Self {
        Lexer::new(file.text(), file.start_pos())
    }

    /// Whether the whole input has been consumed.
    #[inline]
    pub fn is_at_end(&self) -> bool {
        self.pos as usize >= self.src.len()
    }

    /// Global offset of the next byte to be lexed.
    #[inline]
    pub fn offset(&self) -> u32 {
        self.base + self.pos
    }

    /// Produces the next token, reporting any problems to `sink`.
    ///
    /// Once the input is exhausted this returns an empty
    /// [`Eof`](TokenKind::Eof) token forever.
    pub fn next_token(&mut self, sink: &mut dyn DiagSink) -> Token {
        let Some(b) = self.peek() else {
            return Token::new(TokenKind::Eof, Span::empty(self.base + self.pos));
        };
        match CLASS[b as usize] {
            Class::Space => self.whitespace(),
            Class::Newline => self.newline(),
            Class::Ident => self.ident(sink),
            Class::Digit => self.number(sink),
            Class::Quote => self.string(sink),
            Class::Hash => self.comment(),
            Class::Punct => self.punct(sink),
            Class::NonAscii => self.non_ascii(sink),
            Class::Unknown => self.unknown(sink),
        }
    }

    // --- primitives -------------------------------------------------------

    #[inline]
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos as usize).copied()
    }

    #[inline]
    fn peek_at(&self, offset: u32) -> Option<u8> {
        self.src.get((self.pos + offset) as usize).copied()
    }

    /// Consumes `b` if it is next.
    #[inline]
    fn eat(&mut self, b: u8) -> bool {
        if self.peek() == Some(b) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// The global span from local `start` to the current position.
    #[inline]
    fn span_from(&self, start: u32) -> Span {
        Span::new(self.base + start, self.base + self.pos)
    }

    /// The global span of the single byte at local offset `at`.
    #[inline]
    fn byte_span(&self, at: u32) -> Span {
        Span::new(self.base + at, self.base + at + 1)
    }

    #[inline]
    fn slice(&self, start: u32, end: u32) -> &'src [u8] {
        &self.src[start as usize..end as usize]
    }

    // --- token scanners ---------------------------------------------------

    fn whitespace(&mut self) -> Token {
        let start = self.pos;
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.pos += 1;
        }
        Token::new(TokenKind::Whitespace, self.span_from(start))
    }

    fn newline(&mut self) -> Token {
        let start = self.pos;
        if self.src[start as usize] == b'\r' && self.peek_at(1) == Some(b'\n') {
            self.pos += 2;
        } else {
            self.pos += 1;
        }
        Token::new(TokenKind::Newline, self.span_from(start))
    }

    fn comment(&mut self) -> Token {
        let start = self.pos;
        self.pos += 1;
        let is_doc = self.eat(b'#');
        let rest = &self.src[self.pos as usize..];
        let end =
            memchr::memchr2(b'\n', b'\r', rest).map_or(self.src.len(), |i| self.pos as usize + i);
        self.pos = end as u32;
        let kind = if is_doc {
            TokenKind::DocComment
        } else {
            TokenKind::Comment
        };
        Token::new(kind, self.span_from(start))
    }

    fn ident(&mut self, sink: &mut dyn DiagSink) -> Token {
        let start = self.pos;
        while self.peek().is_some_and(is_ident_continue) {
            self.pos += 1;
        }
        if self.peek().is_some_and(|b| b >= 0x80) {
            // ASCII-only identifiers (ADR-0002): consume the mixed run so the
            // parser sees one broken identifier rather than a token storm.
            self.eat_non_ascii_run();
            let span = self.span_from(start);
            sink.emit(errors::non_ascii_ident(span, true));
            return Token::new(TokenKind::Ident, span);
        }
        let kind = keyword_kind(self.slice(start, self.pos)).unwrap_or(TokenKind::Ident);
        Token::new(kind, self.span_from(start))
    }

    fn non_ascii(&mut self, sink: &mut dyn DiagSink) -> Token {
        let start = self.pos;
        self.eat_non_ascii_run();
        let span = self.span_from(start);
        sink.emit(errors::non_ascii_ident(span, false));
        Token::new(TokenKind::Unknown, span)
    }

    fn eat_non_ascii_run(&mut self) {
        while self
            .peek()
            .is_some_and(|b| b >= 0x80 || is_ident_continue(b))
        {
            self.pos += 1;
        }
    }

    fn unknown(&mut self, sink: &mut dyn DiagSink) -> Token {
        let start = self.pos;
        self.pos += 1;
        let b = self.src[start as usize];
        // Consume `&&` / `||` as one token so the fix can replace the pair.
        if (b == b'&' || b == b'|') && self.peek() == Some(b) {
            self.pos += 1;
        }
        let span = self.span_from(start);
        sink.emit(errors::unknown_char(span, self.slice(start, self.pos)));
        Token::new(TokenKind::Unknown, span)
    }

    fn punct(&mut self, sink: &mut dyn DiagSink) -> Token {
        use TokenKind::{
            Arrow, BangEq, Colon, ColonColon, Comma, Dot, DotDot, DotDotEq, Eq, EqEq, Gt, GtEq,
            LBrace, LBracket, LParen, Lt, LtEq, Minus, MinusEq, Percent, PercentEq, Plus, PlusEq,
            RBrace, RBracket, RParen, Slash, SlashEq, Star, StarEq,
        };
        let start = self.pos;
        let b = self.src[start as usize];
        self.pos += 1;
        let kind = match b {
            b'(' => LParen,
            b')' => RParen,
            b'[' => LBracket,
            b']' => RBracket,
            b'{' => LBrace,
            b'}' => RBrace,
            b',' => Comma,
            b'.' => {
                if self.eat(b'.') {
                    if self.eat(b'=') { DotDotEq } else { DotDot }
                } else {
                    Dot
                }
            }
            b':' => {
                if self.eat(b':') {
                    ColonColon
                } else {
                    Colon
                }
            }
            b'=' => {
                if self.eat(b'=') {
                    EqEq
                } else {
                    Eq
                }
            }
            b'<' => {
                if self.eat(b'=') {
                    LtEq
                } else {
                    Lt
                }
            }
            b'>' => {
                if self.eat(b'=') {
                    GtEq
                } else {
                    Gt
                }
            }
            b'+' => {
                if self.eat(b'=') {
                    PlusEq
                } else {
                    Plus
                }
            }
            b'-' => {
                if self.eat(b'>') {
                    Arrow
                } else if self.eat(b'=') {
                    MinusEq
                } else {
                    Minus
                }
            }
            b'*' => {
                if self.eat(b'=') {
                    StarEq
                } else {
                    Star
                }
            }
            b'/' => {
                if self.eat(b'=') {
                    SlashEq
                } else {
                    Slash
                }
            }
            b'%' => {
                if self.eat(b'=') {
                    PercentEq
                } else {
                    Percent
                }
            }
            // `!` only exists as part of `!=`; a lone `!` is `not` in Rynix.
            b'!' => {
                if self.eat(b'=') {
                    BangEq
                } else {
                    let span = self.span_from(start);
                    sink.emit(errors::unknown_char(span, self.slice(start, self.pos)));
                    return Token::new(TokenKind::Unknown, span);
                }
            }
            _ => unreachable!("byte {b:#x} is not classified as punctuation"),
        };
        Token::new(kind, self.span_from(start))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rynix_diag::VecSink;

    fn lex(src: &str) -> (Vec<Token>, VecSink) {
        let mut sink = VecSink::new();
        let mut lexer = Lexer::new(src, 0);
        let mut tokens = Vec::new();
        loop {
            let t = lexer.next_token(&mut sink);
            let stop = t.is_eof();
            tokens.push(t);
            if stop {
                break;
            }
        }
        (tokens, sink)
    }

    fn kinds(src: &str) -> Vec<TokenKind> {
        lex(src).0.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn empty_input_is_just_eof() {
        let (tokens, sink) = lex("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
        assert_eq!(tokens[0].span, Span::empty(0));
        assert!(sink.is_empty());
    }

    #[test]
    fn newline_variants_each_produce_one_token() {
        let (tokens, _) = lex("a\nb\r\nc\rd");
        let spans: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Newline)
            .map(|t| (t.span.lo(), t.span.hi()))
            .collect();
        assert_eq!(spans, vec![(1, 2), (3, 5), (6, 7)]);
    }

    #[test]
    fn whitespace_runs_collapse_into_one_token() {
        let (tokens, _) = lex("a \t  b");
        assert_eq!(tokens[1].kind, TokenKind::Whitespace);
        assert_eq!(tokens[1].span, Span::new(1, 5));
    }

    #[test]
    fn comments_and_doc_comments() {
        assert_eq!(
            kinds("# note\n## doc\n"),
            vec![
                TokenKind::Comment,
                TokenKind::Newline,
                TokenKind::DocComment,
                TokenKind::Newline,
                TokenKind::Eof
            ]
        );
        // A comment at EOF without a trailing newline still terminates.
        let (tokens, _) = lex("x # tail");
        assert_eq!(tokens[2].kind, TokenKind::Comment);
        assert_eq!(tokens[2].span, Span::new(2, 8));
    }

    #[test]
    fn keywords_and_identifiers() {
        assert_eq!(
            kinds("def end_of x1 _y"),
            vec![
                TokenKind::Def,
                TokenKind::Whitespace,
                TokenKind::Ident, // end_of is not `end`
                TokenKind::Whitespace,
                TokenKind::Ident,
                TokenKind::Whitespace,
                TokenKind::Ident,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn every_punctuation_form() {
        let cases: &[(&str, TokenKind)] = &[
            ("(", TokenKind::LParen),
            (")", TokenKind::RParen),
            ("[", TokenKind::LBracket),
            ("]", TokenKind::RBracket),
            ("{", TokenKind::LBrace),
            ("}", TokenKind::RBrace),
            (",", TokenKind::Comma),
            (".", TokenKind::Dot),
            ("..", TokenKind::DotDot),
            ("..=", TokenKind::DotDotEq),
            (":", TokenKind::Colon),
            ("::", TokenKind::ColonColon),
            ("->", TokenKind::Arrow),
            ("=", TokenKind::Eq),
            ("==", TokenKind::EqEq),
            ("!=", TokenKind::BangEq),
            ("<", TokenKind::Lt),
            ("<=", TokenKind::LtEq),
            (">", TokenKind::Gt),
            (">=", TokenKind::GtEq),
            ("+", TokenKind::Plus),
            ("-", TokenKind::Minus),
            ("*", TokenKind::Star),
            ("/", TokenKind::Slash),
            ("%", TokenKind::Percent),
            ("+=", TokenKind::PlusEq),
            ("-=", TokenKind::MinusEq),
            ("*=", TokenKind::StarEq),
            ("/=", TokenKind::SlashEq),
            ("%=", TokenKind::PercentEq),
        ];
        for (src, expected) in cases {
            let (tokens, sink) = lex(src);
            assert_eq!(tokens[0].kind, *expected, "lexing {src:?}");
            assert_eq!(tokens[0].span, Span::new(0, src.len() as u32), "{src:?}");
            assert!(sink.is_empty(), "{src:?} produced diagnostics");
        }
    }

    #[test]
    fn maximal_munch_boundaries() {
        // `1..2` must not turn `.` into a float; `..=` beats `..`.
        assert_eq!(
            kinds("1..2"),
            vec![
                TokenKind::IntLit,
                TokenKind::DotDot,
                TokenKind::IntLit,
                TokenKind::Eof
            ]
        );
        assert_eq!(
            kinds("a..=b"),
            vec![
                TokenKind::Ident,
                TokenKind::DotDotEq,
                TokenKind::Ident,
                TokenKind::Eof
            ]
        );
        // `->` wins over `-`, and `-=` over `-`.
        assert_eq!(kinds("->"), vec![TokenKind::Arrow, TokenKind::Eof]);
        assert_eq!(
            kinds("- >"),
            vec![
                TokenKind::Minus,
                TokenKind::Whitespace,
                TokenKind::Gt,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn lone_bang_is_unknown_with_not_fix() {
        let (tokens, sink) = lex("!x");
        assert_eq!(tokens[0].kind, TokenKind::Unknown);
        assert_eq!(tokens[0].span, Span::new(0, 1));
        assert_eq!(tokens[1].kind, TokenKind::Ident);
        assert_eq!(sink.diags.len(), 1);
        assert_eq!(sink.diags[0].code.as_str(), "RYX0001");
        assert_eq!(sink.diags[0].fixes[0].edits[0].replacement, "not ");
    }

    #[test]
    fn double_ampersand_and_pipe_are_single_tokens() {
        let (tokens, sink) = lex("a && b || c");
        let unknowns: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Unknown)
            .map(|t| (t.span.lo(), t.span.hi()))
            .collect();
        assert_eq!(unknowns, vec![(2, 4), (7, 9)]);
        assert_eq!(sink.diags.len(), 2);
        assert_eq!(sink.diags[0].fixes[0].edits[0].replacement, "and");
        assert_eq!(sink.diags[1].fixes[0].edits[0].replacement, "or");
    }

    #[test]
    fn semicolon_reports_removal_fix() {
        let (_, sink) = lex("let x = 1;");
        assert_eq!(sink.diags.len(), 1);
        assert_eq!(sink.diags[0].code.as_str(), "RYX0001");
        assert_eq!(sink.diags[0].fixes[0].edits[0].replacement, "");
        assert!(sink.diags[0].fixes[0].confidence >= 0.9);
    }

    #[test]
    fn non_ascii_identifier_is_reported_but_recovered() {
        let (tokens, sink) = lex("let café = 1");
        assert_eq!(sink.diags.len(), 1);
        assert_eq!(sink.diags[0].code.as_str(), "RYX0003");
        // `café` stays one Ident token (5 bytes: c a f é).
        let ident = tokens
            .iter()
            .find(|t| t.kind == TokenKind::Ident)
            .expect("ident");
        assert_eq!(ident.span, Span::new(4, 9));
    }

    #[test]
    fn leading_non_ascii_is_one_unknown_token() {
        let (tokens, sink) = lex("λx = 1");
        assert_eq!(tokens[0].kind, TokenKind::Unknown);
        assert_eq!(tokens[0].span, Span::new(0, 3), "λ is 2 bytes plus `x`");
        assert_eq!(sink.diags.len(), 1);
        assert_eq!(sink.diags[0].code.as_str(), "RYX0003");
    }

    #[test]
    fn eof_is_idempotent() {
        let mut sink = VecSink::new();
        let mut lexer = Lexer::new("x", 0);
        assert_eq!(lexer.next_token(&mut sink).kind, TokenKind::Ident);
        for _ in 0..3 {
            let t = lexer.next_token(&mut sink);
            assert_eq!(t.kind, TokenKind::Eof);
            assert_eq!(t.span, Span::empty(1));
        }
        assert!(lexer.is_at_end());
    }

    #[test]
    fn base_offset_is_applied_to_every_span() {
        let mut sink = VecSink::new();
        let mut lexer = Lexer::new("def", 1000);
        let t = lexer.next_token(&mut sink);
        assert_eq!(t.span, Span::new(1000, 1003));
        assert_eq!(lexer.next_token(&mut sink).span, Span::empty(1003));
    }
}
