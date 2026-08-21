//! Recursive-descent + Pratt parser for Rynix.
//!
//! Implemented in Phase 2. Like the lexer, the parser is total: it always
//! produces a tree, inserting `Error` nodes and synchronizing at statement
//! boundaries (`Newline`, `end`, `def`, `struct`).
