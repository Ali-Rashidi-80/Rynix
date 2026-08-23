//! Property tests for the lexer's structural invariants.
//!
//! These hold for *every* UTF-8 input, valid Rynix or not. They are what
//! makes the lexer safe to run on untrusted or AI-generated source.

use proptest::prelude::*;
use rynix_diag::VecSink;
use rynix_lexer::{Token, TokenKind, lex_all};

/// Random Rynix-ish source: fragments joined by random separators. Much more
/// likely to hit interesting lexer states than fully random text.
#[rustfmt::skip] // the fragment table reads better as a dense grid
fn source_soup() -> impl Strategy<Value = String> {
    let fragment = prop::sample::select(vec![
        "def", "end", "let", "mut", "if", "elif", "else", "loop", "for", "in", "return", "struct",
        "enum", "type", "import", "pub", "spawn", "region", "true", "false", "nil", "and", "or", "not", "as",
        "match", "agent", "signal", "tensor", "ident", "_x9", "0", "42", "1_000", "0xFF", "0X1",
        "0o7", "0b1", "1.5", "1e9", "1E9", "1.", "1e", "123abc", "1__0", "\"str\"", "\"a\\nb\"",
        "\"\\q\"", "\"\\u{1F600}\"", "\"\\u{D800}\"", "\"open", "(", ")", "[", "]", "{", "}", ",",
        ".", "..", "..=", ":", "::", "->", "=", "==", "!=", "<", "<=", ">", ">=", "+", "-", "*",
        "/", "%", "+=", "-=", "*=", "/=", "%=", "!", "&", "&&", "|", "||", ";", "$", "@", "'",
        "#c", "##d", "\n", "\r\n", "\r", " ", "\t", "café", "λ", "漢字", "\0", "\u{7}",
    ]);
    prop::collection::vec(fragment, 0..40).prop_map(|parts| parts.concat())
}

fn lex(src: &str) -> (Vec<Token>, VecSink) {
    let mut sink = VecSink::new();
    let tokens = lex_all(src, 0, &mut sink);
    (tokens, sink)
}

/// The core invariant: tokens tile the input byte-exactly, in order, with no
/// gaps, no overlaps, and no loss.
fn assert_tiles(src: &str, tokens: &[Token]) {
    let last = tokens.last().expect("at least Eof");
    assert_eq!(last.kind, TokenKind::Eof, "stream must end with Eof");
    assert!(last.span.is_empty(), "Eof must be empty");

    let mut expected_lo = 0u32;
    let mut reassembled = String::with_capacity(src.len());
    for token in tokens {
        assert_eq!(token.span.lo(), expected_lo, "gap or overlap in {src:?}");
        if token.kind == TokenKind::Eof {
            assert_eq!(token.span.hi(), src.len() as u32);
        } else {
            assert!(
                !token.span.is_empty(),
                "non-Eof token {:?} is empty in {src:?}",
                token.kind
            );
        }
        assert!(
            src.is_char_boundary(token.span.lo() as usize)
                && src.is_char_boundary(token.span.hi() as usize),
            "span {:?} splits a character in {src:?}",
            token.span
        );
        reassembled.push_str(&src[token.span.lo() as usize..token.span.hi() as usize]);
        expected_lo = token.span.hi();
    }
    assert_eq!(expected_lo, src.len() as u32, "input not fully consumed");
    assert_eq!(reassembled, src, "token text does not reproduce the input");
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    /// Totality plus perfect tiling on structured input.
    #[test]
    fn tiles_structured_input(src in source_soup()) {
        let (tokens, _) = lex(&src);
        assert_tiles(&src, &tokens);
    }

    /// Totality plus perfect tiling on arbitrary Unicode text.
    #[test]
    fn tiles_arbitrary_unicode(src in ".{0,200}") {
        let (tokens, _) = lex(&src);
        assert_tiles(&src, &tokens);
    }

    /// Lexing is deterministic.
    #[test]
    fn deterministic(src in source_soup()) {
        let (a, sink_a) = lex(&src);
        let (b, sink_b) = lex(&src);
        prop_assert_eq!(a, b);
        prop_assert_eq!(sink_a.diags.len(), sink_b.diags.len());
    }

    /// Every span and every diagnostic span shifts uniformly with the base
    /// offset, so multi-file compilation cannot alias positions.
    #[test]
    fn base_offset_is_a_pure_shift(src in source_soup(), base in 0u32..100_000) {
        let (zero_based, mut sink_zero) = lex(&src);
        let mut sink_shifted = VecSink::new();
        let shifted = lex_all(&src, base, &mut sink_shifted);

        prop_assert_eq!(zero_based.len(), shifted.len());
        for (a, b) in zero_based.iter().zip(&shifted) {
            prop_assert_eq!(a.kind, b.kind);
            prop_assert_eq!(a.span.lo() + base, b.span.lo());
            prop_assert_eq!(a.span.hi() + base, b.span.hi());
        }
        prop_assert_eq!(sink_zero.diags.len(), sink_shifted.diags.len());
        for (a, b) in sink_zero.diags.drain(..).zip(&sink_shifted.diags) {
            prop_assert_eq!(a.primary.span.lo() + base, b.primary.span.lo());
        }
    }

    /// Diagnostics always point inside the file and are well-formed, which
    /// the JSON renderer and any agent applying fixes rely on.
    #[test]
    fn diagnostics_are_well_formed(src in source_soup()) {
        let (_, sink) = lex(&src);
        let len = src.len() as u32;
        for diag in &sink.diags {
            prop_assert!(diag.primary.span.hi() <= len);
            for label in &diag.secondary {
                prop_assert!(label.span.hi() <= len);
            }
            for fix in &diag.fixes {
                prop_assert!((0.0..=1.0).contains(&fix.confidence));
                prop_assert!(!fix.edits.is_empty());
                for edit in &fix.edits {
                    prop_assert!(edit.span.hi() <= len);
                    prop_assert!(src.is_char_boundary(edit.span.lo() as usize));
                    prop_assert!(src.is_char_boundary(edit.span.hi() as usize));
                }
            }
        }
    }

    /// Keywords are only ever produced for exactly their own spelling.
    #[test]
    fn keyword_spellings_are_exact(src in source_soup()) {
        let (tokens, _) = lex(&src);
        for token in &tokens {
            if token.kind.is_keyword() {
                let text = &src[token.span.lo() as usize..token.span.hi() as usize];
                prop_assert_eq!(Some(text), token.kind.spelling());
            }
        }
    }

    /// Applying a single high-confidence fix never makes things worse: the
    /// patched source still tiles perfectly and has no more diagnostics than
    /// before, neither in total nor for the code that was fixed.
    ///
    /// Note this is deliberately *not* "strictly fewer": the lexer reports at
    /// most one problem per literal, so fixing `0X1end` reveals the invalid
    /// digit that was hidden behind the uppercase prefix.
    #[test]
    fn high_confidence_fixes_never_make_things_worse(src in source_soup()) {
        let (_, sink) = lex(&src);
        let Some(diag) = sink
            .diags
            .iter()
            .find(|d| d.fixes.iter().any(|f| f.confidence >= 0.9 && f.edits.len() == 1))
        else {
            return Ok(());
        };
        let fix = diag
            .fixes
            .iter()
            .find(|f| f.confidence >= 0.9 && f.edits.len() == 1)
            .expect("checked above");
        let edit = &fix.edits[0];
        let mut patched = String::with_capacity(src.len() + edit.replacement.len());
        patched.push_str(&src[..edit.span.lo() as usize]);
        patched.push_str(&edit.replacement);
        patched.push_str(&src[edit.span.hi() as usize..]);

        let (patched_tokens, patched_sink) = lex(&patched);
        assert_tiles(&patched, &patched_tokens);

        let same_code = |sink: &VecSink| sink.diags.iter().filter(|d| d.code == diag.code).count();
        prop_assert!(
            same_code(&patched_sink) <= same_code(&sink),
            "fix increased {} occurrences\nsrc: {src:?}\npatched: {patched:?}",
            diag.code
        );
        prop_assert!(
            patched_sink.diags.len() <= sink.diags.len(),
            "fix increased the total diagnostic count from {} to {}\nsrc: {src:?}\npatched: {patched:?}",
            sink.diags.len(),
            patched_sink.diags.len()
        );
    }
}
