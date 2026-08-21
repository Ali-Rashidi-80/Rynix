//! Fuzzes the lexer's totality and tiling invariants.
//!
//! Run on Linux (or WSL2) with a nightly toolchain:
//! `cargo +nightly fuzz run lex_total`

#![no_main]

use libfuzzer_sys::fuzz_target;
use rynix_diag::CountSink;
use rynix_lexer::{Lexer, TokenKind};

fuzz_target!(|data: &str| {
    let mut sink = CountSink::new();
    let mut lexer = Lexer::new(data, 0);
    let mut expected_lo = 0u32;

    loop {
        let token = lexer.next_token(&mut sink);
        assert_eq!(token.span.lo(), expected_lo, "gap or overlap in token stream");
        assert!(
            data.is_char_boundary(token.span.lo() as usize)
                && data.is_char_boundary(token.span.hi() as usize),
            "span splits a UTF-8 character"
        );
        if token.kind == TokenKind::Eof {
            assert!(token.span.is_empty());
            assert_eq!(token.span.hi() as usize, data.len(), "input not consumed");
            break;
        }
        assert!(!token.span.is_empty(), "non-Eof token must consume bytes");
        expected_lo = token.span.hi();
    }
});
