//! Numeric literal scanning: decimal, hex, octal, binary, and floats.
//!
//! At most one diagnostic is emitted per literal (the first problem found),
//! so a typo never turns into a diagnostic storm.

use rynix_diag::DiagSink;
use rynix_span::Span;

use super::{is_digit_radix, is_ident_continue, Lexer};
use crate::errors;
use crate::token::{Token, TokenKind};

impl<'src> Lexer<'src> {
    /// Entry point: the current byte is an ASCII digit.
    pub(super) fn number(&mut self, sink: &mut dyn DiagSink) -> Token {
        let start = self.pos;
        if self.src[start as usize] == b'0' {
            if let Some(marker) = self.peek_at(1) {
                let radix = match marker {
                    b'x' | b'X' => Some(16),
                    b'o' | b'O' => Some(8),
                    b'b' | b'B' => Some(2),
                    _ => None,
                };
                if let Some(radix) = radix {
                    return self.radix_number(start, radix, marker, sink);
                }
            }
        }
        self.decimal_number(start, sink)
    }

    fn radix_number(
        &mut self,
        start: u32,
        radix: u32,
        marker: u8,
        sink: &mut dyn DiagSink,
    ) -> Token {
        self.pos += 2; // the `0` and the base marker
        let digits_start = self.pos;
        // Consume the whole identifier-ish run so invalid digits and bogus
        // suffixes are diagnosed as part of this literal.
        while self.peek().is_some_and(is_ident_continue) {
            self.pos += 1;
        }
        let whole = self.span_from(start);

        if marker.is_ascii_uppercase() {
            let lowered = marker.to_ascii_lowercase() as char;
            sink.emit(errors::wrong_case_in_number(
                self.byte_span(start + 1),
                whole,
                format!("base prefix `0{marker}` must be lowercase", marker = marker as char),
                &lowered.to_string(),
                0.95,
            ));
        } else if digits_start == self.pos {
            sink.emit(errors::malformed_number(
                whole,
                whole,
                format!(
                    "missing digits after base prefix `0{}`",
                    marker.to_ascii_lowercase() as char
                ),
            ));
        } else {
            self.validate_digit_run(
                self.slice(digits_start, self.pos),
                digits_start,
                radix,
                whole,
                sink,
            );
        }
        Token::new(TokenKind::IntLit, whole)
    }

    fn decimal_number(&mut self, start: u32, sink: &mut dyn DiagSink) -> Token {
        let int_run = self.eat_digit_run();
        let mut kind = TokenKind::IntLit;
        let mut frac_run = None;
        let mut exp_run = None;
        let mut uppercase_e = None;

        // A `.` only starts a fraction when a digit follows, so `1..2` stays
        // a range and `1.` stays an integer followed by `.`.
        if self.peek() == Some(b'.') && self.peek_at(1).is_some_and(|b| b.is_ascii_digit()) {
            self.pos += 1;
            frac_run = Some(self.eat_digit_run());
            kind = TokenKind::FloatLit;
        }

        if let Some(marker @ (b'e' | b'E')) = self.peek() {
            if self.exponent_digits_follow() {
                if marker == b'E' {
                    uppercase_e = Some(self.pos);
                }
                self.pos += 1;
                if matches!(self.peek(), Some(b'+' | b'-')) {
                    self.pos += 1;
                }
                exp_run = Some(self.eat_digit_run());
                kind = TokenKind::FloatLit;
            }
        }

        // Anything identifier-like left over is either a suffix (`123abc`) or
        // a truncated exponent (`1e`): both are errors in v0.1.
        let suffix_start = self.pos;
        while self.peek().is_some_and(is_ident_continue) {
            self.pos += 1;
        }
        let whole = self.span_from(start);

        let mut ok = true;
        for (run_start, run_end) in [Some(int_run), frac_run, exp_run].into_iter().flatten() {
            if !ok {
                break;
            }
            ok = self.validate_digit_run(
                self.slice(run_start, run_end),
                run_start,
                10,
                whole,
                sink,
            );
        }
        if ok {
            if let Some(at) = uppercase_e {
                sink.emit(errors::wrong_case_in_number(
                    self.byte_span(at),
                    whole,
                    "exponent marker must be the lowercase `e`",
                    "e",
                    0.90,
                ));
                ok = false;
            }
        }
        if ok && suffix_start != self.pos {
            self.report_suffix(suffix_start, whole, sink);
        }
        Token::new(kind, whole)
    }

    fn report_suffix(&self, suffix_start: u32, whole: Span, sink: &mut dyn DiagSink) {
        let suffix = self.slice(suffix_start, self.pos);
        let truncated_exponent = matches!(suffix, b"e" | b"E");
        let message = if truncated_exponent {
            "missing digits in exponent".to_string()
        } else {
            format!(
                "invalid suffix `{}`: Rynix has no numeric suffixes, use `as` to convert",
                String::from_utf8_lossy(suffix)
            )
        };
        sink.emit(errors::malformed_number(
            Span::new(self.base + suffix_start, self.base + self.pos),
            whole,
            message,
        ));
    }

    /// Consumes a run of `[0-9_]` and returns its file-local bounds.
    fn eat_digit_run(&mut self) -> (u32, u32) {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|b| b.is_ascii_digit() || b == b'_')
        {
            self.pos += 1;
        }
        (start, self.pos)
    }

    /// Whether the `e`/`E` at the current position is followed by a valid
    /// exponent (optional sign, then at least one digit).
    fn exponent_digits_follow(&self) -> bool {
        let mut offset = 1;
        if matches!(self.peek_at(offset), Some(b'+' | b'-')) {
            offset += 1;
        }
        self.peek_at(offset).is_some_and(|b| b.is_ascii_digit())
    }

    /// Validates digit/underscore placement. Returns `false` (after emitting
    /// one diagnostic) on the first problem.
    fn validate_digit_run(
        &self,
        digits: &[u8],
        offset: u32,
        radix: u32,
        whole: Span,
        sink: &mut dyn DiagSink,
    ) -> bool {
        for (i, &b) in digits.iter().enumerate() {
            let at = offset + i as u32;
            if b == b'_' {
                let prev_ok = i > 0 && is_digit_radix(digits[i - 1], radix);
                let next_ok = digits
                    .get(i + 1)
                    .copied()
                    .is_some_and(|n| is_digit_radix(n, radix));
                if !prev_ok || !next_ok {
                    sink.emit(errors::malformed_number(
                        self.byte_span(at),
                        whole,
                        "`_` in a number literal must separate two digits",
                    ));
                    return false;
                }
            } else if !is_digit_radix(b, radix) {
                let message = if radix == 10 {
                    "invalid digit in decimal literal".to_string()
                } else {
                    format!(
                        "invalid digit `{}` for a base-{radix} literal",
                        b as char
                    )
                };
                sink.emit(errors::malformed_number(
                    self.byte_span(at),
                    whole,
                    message,
                ));
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::token::TokenKind;
    use crate::Lexer;
    use rynix_diag::VecSink;
    use rynix_span::Span;

    fn lex_one(src: &str) -> (TokenKind, Span, VecSink) {
        let mut sink = VecSink::new();
        let mut lexer = Lexer::new(src, 0);
        let token = lexer.next_token(&mut sink);
        (token.kind, token.span, sink)
    }

    #[test]
    fn valid_integers() {
        for src in ["0", "7", "42", "1_000", "1_000_000", "0x1F", "0xdead_beef", "0o755", "0b1010_0101"] {
            let (kind, span, sink) = lex_one(src);
            assert_eq!(kind, TokenKind::IntLit, "{src}");
            assert_eq!(span, Span::new(0, src.len() as u32), "{src}");
            assert!(sink.is_empty(), "{src} -> {:?}", sink.diags);
        }
    }

    #[test]
    fn valid_floats() {
        for src in ["1.5", "0.0", "3.141_592", "1e10", "1e+10", "1e-10", "2.5e-3"] {
            let (kind, span, sink) = lex_one(src);
            assert_eq!(kind, TokenKind::FloatLit, "{src}");
            assert_eq!(span, Span::new(0, src.len() as u32), "{src}");
            assert!(sink.is_empty(), "{src} -> {:?}", sink.diags);
        }
    }

    #[test]
    fn dot_without_digits_is_not_a_float() {
        let (kind, span, sink) = lex_one("1.");
        assert_eq!(kind, TokenKind::IntLit);
        assert_eq!(span, Span::new(0, 1));
        assert!(sink.is_empty());
    }

    #[test]
    fn uppercase_prefix_gets_lowercase_fix() {
        let (kind, _, sink) = lex_one("0XFF");
        assert_eq!(kind, TokenKind::IntLit);
        assert_eq!(sink.diags.len(), 1);
        let d = &sink.diags[0];
        assert_eq!(d.code.as_str(), "RYX0004");
        assert_eq!(d.primary.span, Span::new(1, 2));
        assert_eq!(d.fixes[0].edits[0].replacement, "x");
        assert!(d.fixes[0].confidence >= 0.95);
    }

    #[test]
    fn uppercase_exponent_gets_lowercase_fix() {
        let (kind, _, sink) = lex_one("1E5");
        assert_eq!(kind, TokenKind::FloatLit);
        assert_eq!(sink.diags.len(), 1);
        assert_eq!(sink.diags[0].fixes[0].edits[0].replacement, "e");
    }

    #[test]
    fn missing_digits_after_prefix() {
        let (_, span, sink) = lex_one("0x");
        assert_eq!(span, Span::new(0, 2));
        assert_eq!(sink.diags.len(), 1);
        assert!(sink.diags[0].message.contains("missing digits"), "{:?}", sink.diags[0].message);
    }

    #[test]
    fn invalid_digit_for_radix() {
        for (src, bad_at) in [("0b12", 3), ("0o18", 3), ("0xzz", 2)] {
            let (_, _, sink) = lex_one(src);
            assert_eq!(sink.diags.len(), 1, "{src}");
            assert_eq!(sink.diags[0].code.as_str(), "RYX0004", "{src}");
            assert_eq!(sink.diags[0].primary.span, Span::new(bad_at, bad_at + 1), "{src}");
        }
    }

    #[test]
    fn misplaced_underscores() {
        for src in ["1__0", "1_", "0x_1", "1_.5", "1.5_", "1e1_"] {
            let (_, _, sink) = lex_one(src);
            assert!(!sink.is_empty(), "{src} should be rejected");
            assert_eq!(sink.diags[0].code.as_str(), "RYX0004", "{src}");
        }
    }

    #[test]
    fn underscore_after_dot_is_a_field_access_not_a_float() {
        // `1._5` is canonically IntLit `.` Ident: a fraction needs a digit.
        let mut sink = VecSink::new();
        let mut lexer = Lexer::new("1._5", 0);
        assert_eq!(lexer.next_token(&mut sink).kind, TokenKind::IntLit);
        assert_eq!(lexer.next_token(&mut sink).kind, TokenKind::Dot);
        assert_eq!(lexer.next_token(&mut sink).kind, TokenKind::Ident);
        assert!(sink.is_empty());
    }

    #[test]
    fn numeric_suffixes_are_rejected() {
        let (_, span, sink) = lex_one("123abc");
        assert_eq!(span, Span::new(0, 6), "the suffix is part of the literal");
        assert_eq!(sink.diags.len(), 1);
        assert!(sink.diags[0].message.contains("invalid suffix `abc`"), "{:?}", sink.diags[0].message);
    }

    #[test]
    fn truncated_exponent() {
        let (_, _, sink) = lex_one("1e");
        assert_eq!(sink.diags.len(), 1);
        assert_eq!(sink.diags[0].message, "missing digits in exponent");
    }

    #[test]
    fn one_diagnostic_per_literal() {
        // Uppercase prefix *and* an invalid digit: only the prefix is reported.
        let (_, _, sink) = lex_one("0Bxyz");
        assert_eq!(sink.diags.len(), 1);
    }
}
