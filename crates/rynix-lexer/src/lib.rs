//! Zero-allocation lexer for Rynix source (ADR-0004).
//!
//! The lexer is *total*: any UTF-8 input produces a token stream that tiles
//! the input exactly, with structured diagnostics instead of failures.

pub struct Placeholder;
