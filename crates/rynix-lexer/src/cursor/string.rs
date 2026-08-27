//! String literal scanning with escape validation.
//!
//! Strings are single-line (spec 2.5): a raw line terminator ends the token
//! with `RYX0002`, and end-of-input ends it with `RYX0006`. Both produce a
//! `StrLit` recovery token so the parser can continue.

use rynix_diag::DiagSink;

use super::{Lexer, utf8_len};
use crate::errors;
use crate::token::{Token, TokenKind};

impl Lexer<'_> {
    /// Entry point: the current byte is `"`.
    pub(super) fn string(&mut self, sink: &mut dyn DiagSink) -> Token {
        let start = self.pos;
        self.pos += 1;
        // Triple-quoted multiline: """ ... """
        if self.pos + 1 < self.src.len() as u32
            && self.src[self.pos as usize] == b'"'
            && self.src[(self.pos + 1) as usize] == b'"'
        {
            self.pos += 2;
            return self.triple_string(start, sink);
        }
        loop {
            let Some(hit) = self.find_string_stop() else {
                self.pos = self.src.len() as u32;
                let span = self.span_from(start);
                sink.emit(errors::eof_in_string(span));
                return Token::new(TokenKind::StrLit, span);
            };
            self.pos = hit;
            match self.src[hit as usize] {
                b'"' => {
                    self.pos += 1;
                    return Token::new(TokenKind::StrLit, self.span_from(start));
                }
                b'\n' | b'\r' => {
                    let span = self.span_from(start);
                    sink.emit(errors::unterminated_string(span));
                    return Token::new(TokenKind::StrLit, span);
                }
                // A backslash: `escape` always consumes at least the
                // backslash, so the loop always makes progress.
                _ => self.escape(sink),
            }
        }
    }

    fn triple_string(&mut self, start: u32, sink: &mut dyn DiagSink) -> Token {
        loop {
            if self.pos as usize >= self.src.len() {
                let span = self.span_from(start);
                sink.emit(errors::eof_in_string(span));
                return Token::new(TokenKind::StrLit, span);
            }
            let b = self.src[self.pos as usize];
            if b == b'"'
                && self.pos + 2 < self.src.len() as u32
                && self.src[(self.pos + 1) as usize] == b'"'
                && self.src[(self.pos + 2) as usize] == b'"'
            {
                self.pos += 3;
                return Token::new(TokenKind::StrLit, self.span_from(start));
            }
            if b == b'\\' {
                self.escape(sink);
            } else {
                self.pos += utf8_len(b);
            }
        }
    }

    /// Finds the next byte that can end or interrupt a string body.
    ///
    /// `memchr3` covers `"`, `\`, and `\n`; a lone `\r` is a line terminator
    /// too (spec 1), so it needs a second scan. That scan is deliberately
    /// bounded by the first hit: searching the whole remaining file for `\r`
    /// on every lookup would make lexing a string-heavy file quadratic.
    fn find_string_stop(&self) -> Option<u32> {
        let from = self.pos as usize;
        let rest = &self.src[from..];
        match memchr::memchr3(b'"', b'\\', b'\n', rest) {
            Some(hit) => {
                let carriage = memchr::memchr(b'\r', &rest[..hit]);
                Some((from + carriage.unwrap_or(hit)) as u32)
            }
            // Nothing can close the string: one final scan for `\r`, then EOF.
            None => memchr::memchr(b'\r', rest).map(|i| (from + i) as u32),
        }
    }

    /// Handles one escape sequence. The current byte is `\`.
    fn escape(&mut self, sink: &mut dyn DiagSink) {
        let backslash = self.pos;
        self.pos += 1;
        let Some(b) = self.peek() else { return };
        match b {
            b'n' | b't' | b'r' | b'0' | b'\\' | b'"' => self.pos += 1,
            b'x' => {
                self.pos += 1;
                self.hex_escape(backslash, sink);
            }
            b'u' => {
                self.pos += 1;
                self.unicode_escape(backslash, sink);
            }
            // Leave the line terminator to the caller: reporting both a bad
            // escape and an unterminated string would be noise.
            b'\n' | b'\r' => {}
            _ => {
                self.pos += utf8_len(b);
                let span = self.span_from(backslash);
                let text = String::from_utf8_lossy(self.slice(backslash, self.pos));
                sink.emit(errors::invalid_escape(
                    span,
                    format!("unknown escape sequence `{text}`"),
                    true,
                ));
            }
        }
    }

    /// `\xHH` — exactly two hex digits.
    fn hex_escape(&mut self, backslash: u32, sink: &mut dyn DiagSink) {
        let digits_start = self.pos;
        while self.pos - digits_start < 2 && self.peek().is_some_and(|b| b.is_ascii_hexdigit()) {
            self.pos += 1;
        }
        if self.pos - digits_start != 2 {
            sink.emit(errors::invalid_escape(
                self.span_from(backslash),
                "`\\x` must be followed by exactly two hex digits",
                false,
            ));
        }
    }

    /// `\u{H...}` — one to six hex digits forming a Unicode scalar value.
    fn unicode_escape(&mut self, backslash: u32, sink: &mut dyn DiagSink) {
        if !self.eat(b'{') {
            sink.emit(errors::invalid_escape(
                self.span_from(backslash),
                "`\\u` must be followed by `{`",
                false,
            ));
            return;
        }
        let digits_start = self.pos;
        while self.peek().is_some_and(|b| b.is_ascii_hexdigit()) {
            self.pos += 1;
        }
        let digits = self.slice(digits_start, self.pos);
        let closed = self.eat(b'}');
        let span = self.span_from(backslash);

        let problem = if digits.is_empty() {
            Some("`\\u{...}` needs at least one hex digit".to_string())
        } else if digits.len() > 6 {
            Some("`\\u{...}` takes at most six hex digits".to_string())
        } else if !closed {
            Some("`\\u{...}` is missing its closing `}`".to_string())
        } else {
            let value = digits.iter().fold(0u32, |acc, &b| {
                acc * 16 + char::from(b).to_digit(16).unwrap_or(0)
            });
            if char::from_u32(value).is_none() {
                Some(format!("`\\u{{{value:x}}}` is not a Unicode scalar value"))
            } else {
                None
            }
        };
        if let Some(message) = problem {
            sink.emit(errors::invalid_escape(span, message, false));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Lexer;
    use crate::token::TokenKind;
    use rynix_diag::VecSink;
    use rynix_span::Span;

    fn lex_one(src: &str) -> (TokenKind, Span, VecSink) {
        let mut sink = VecSink::new();
        let mut lexer = Lexer::new(src, 0);
        let token = lexer.next_token(&mut sink);
        (token.kind, token.span, sink)
    }

    #[test]
    fn plain_strings() {
        for src in [
            r#""""#,
            r#""hello""#,
            r#""with spaces and #hash""#,
            "\"unicode: سلام 漢字 🚀\"",
        ] {
            let (kind, span, sink) = lex_one(src);
            assert_eq!(kind, TokenKind::StrLit, "{src}");
            assert_eq!(span, Span::new(0, src.len() as u32), "{src}");
            assert!(sink.is_empty(), "{src} -> {:?}", sink.diags);
        }
    }

    #[test]
    fn every_valid_escape() {
        let src = r#""\n\t\r\0\\\"\x41\u{1F680}\u{0}""#;
        let (kind, span, sink) = lex_one(src);
        assert_eq!(kind, TokenKind::StrLit);
        assert_eq!(span, Span::new(0, src.len() as u32));
        assert!(sink.is_empty(), "{:?}", sink.diags);
    }

    #[test]
    fn escaped_quote_does_not_end_the_string() {
        let src = r#""a\"b" x"#;
        let (kind, span, _) = lex_one(src);
        assert_eq!(kind, TokenKind::StrLit);
        assert_eq!(span, Span::new(0, 6));
    }

    #[test]
    fn unterminated_at_newline() {
        let (kind, span, sink) = lex_one("\"abc\nrest");
        assert_eq!(kind, TokenKind::StrLit);
        assert_eq!(span, Span::new(0, 4), "the newline is not consumed");
        assert_eq!(sink.diags.len(), 1);
        assert_eq!(sink.diags[0].code.as_str(), "RYX0002");
        let edit = &sink.diags[0].fixes[0].edits[0];
        assert!(edit.span.is_empty());
        assert_eq!(edit.span.lo(), 4);
    }

    #[test]
    fn unterminated_at_carriage_return() {
        let (_, span, sink) = lex_one("\"abc\r\ndef");
        assert_eq!(span, Span::new(0, 4));
        assert_eq!(sink.diags[0].code.as_str(), "RYX0002");
    }

    #[test]
    fn unterminated_at_eof() {
        let (kind, span, sink) = lex_one("\"abc");
        assert_eq!(kind, TokenKind::StrLit);
        assert_eq!(span, Span::new(0, 4));
        assert_eq!(sink.diags.len(), 1);
        assert_eq!(sink.diags[0].code.as_str(), "RYX0006");
    }

    #[test]
    fn trailing_backslash_at_eof_terminates() {
        let (kind, span, sink) = lex_one("\"abc\\");
        assert_eq!(kind, TokenKind::StrLit);
        assert_eq!(span, Span::new(0, 5));
        assert_eq!(sink.diags[0].code.as_str(), "RYX0006");
    }

    #[test]
    fn backslash_before_newline_reports_only_unterminated() {
        let (_, _, sink) = lex_one("\"abc\\\nx");
        assert_eq!(sink.diags.len(), 1);
        assert_eq!(sink.diags[0].code.as_str(), "RYX0002");
    }

    #[test]
    fn unknown_escape_is_removable() {
        let (kind, _, sink) = lex_one(r#""a\qb""#);
        assert_eq!(kind, TokenKind::StrLit, "lexing continues past the escape");
        assert_eq!(sink.diags.len(), 1);
        assert_eq!(sink.diags[0].code.as_str(), "RYX0005");
        assert_eq!(sink.diags[0].message, "unknown escape sequence `\\q`");
        // The fix removes just the backslash.
        assert_eq!(sink.diags[0].fixes[0].edits[0].span, Span::new(2, 3));
        assert_eq!(sink.diags[0].fixes[0].edits[0].replacement, "");
    }

    #[test]
    fn malformed_hex_escapes() {
        for src in [r#""\x""#, r#""\x4""#, r#""\xzz""#] {
            let (_, _, sink) = lex_one(src);
            assert_eq!(sink.diags.len(), 1, "{src}");
            assert_eq!(sink.diags[0].code.as_str(), "RYX0005", "{src}");
        }
    }

    #[test]
    fn malformed_unicode_escapes() {
        let cases = [
            (r#""\u41""#, "must be followed by `{`"),
            (r#""\u{}""#, "at least one hex digit"),
            (r#""\u{1234567}""#, "at most six hex digits"),
            (r#""\u{41""#, "missing its closing"),
            (r#""\u{D800}""#, "not a Unicode scalar value"),
            (r#""\u{110000}""#, "not a Unicode scalar value"),
        ];
        for (src, expected) in cases {
            let (_, _, sink) = lex_one(src);
            assert_eq!(sink.diags.len(), 1, "{src} -> {:?}", sink.diags);
            assert_eq!(sink.diags[0].code.as_str(), "RYX0005", "{src}");
            assert!(
                sink.diags[0].message.contains(expected),
                "{src}: expected {expected:?}, got {:?}",
                sink.diags[0].message
            );
        }
    }

    #[test]
    fn multiple_bad_escapes_all_reported() {
        let (_, _, sink) = lex_one(r#""\q\w""#);
        assert_eq!(sink.diags.len(), 2);
    }
}
