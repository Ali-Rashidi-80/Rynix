//! Zero-allocation lexer for Rynix source (ADR-0004).
//!
//! The lexer is *total*: any UTF-8 input produces a token stream that tiles
//! the input byte-exactly (every byte belongs to exactly one token; `Eof` is
//! the only empty token), with structured diagnostics instead of failures.
//!
//! ```
//! use rynix_diag::VecSink;
//! use rynix_lexer::{Lexer, TokenKind};
//!
//! let mut sink = VecSink::new();
//! let mut lexer = Lexer::new("def main()", 0);
//! assert_eq!(lexer.next_token(&mut sink).kind, TokenKind::Def);
//! assert!(sink.is_empty());
//! ```

mod cursor;
mod errors;
mod token;

pub use cursor::Lexer;
pub use token::{Token, TokenKind};

use rynix_diag::DiagSink;

/// Lexes the whole input into a vector, including the final `Eof` token.
///
/// Convenience for tools, tests, and benchmarks: unlike [`Lexer`] itself,
/// this allocates the output vector.
pub fn lex_all(src: &str, base: u32, sink: &mut dyn DiagSink) -> Vec<Token> {
    let mut lexer = Lexer::new(src, base);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token(sink);
        let done = token.is_eof();
        tokens.push(token);
        if done {
            return tokens;
        }
    }
}
