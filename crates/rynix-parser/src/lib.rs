//! Recursive-descent + Pratt parser for Rynix.
//!
//! The parser is total: every input yields a [`Module`], with [`Error`](rynix_ast::ErrorNode)
//! nodes filling holes and synchronisation at `{Newline, end, def, struct, enum, type, import}`.

mod errors;
mod expr;
mod item;
mod stmt;
mod ty;

use rynix_ast::{AstArena, Module};
use rynix_diag::DiagSink;
use rynix_lexer::{Lexer, Token, TokenKind};
use rynix_span::{Interner, Span};

use crate::errors as parse_errors;

/// Parse `src` (global offsets start at `base`) into an arena-allocated module.
pub fn parse<'arena>(
    arena: &'arena AstArena,
    interner: &mut Interner,
    src: &str,
    base: u32,
    sink: &mut dyn DiagSink,
) -> &'arena Module<'arena> {
    let mut parser = Parser::new(arena, interner, src, base, sink);
    parser.parse_module()
}

struct Parser<'arena, 'parse, 'src> {
    arena: &'arena AstArena,
    interner: &'parse mut Interner,
    lexer: Lexer<'src>,
    /// Lookahead token with trivia already skipped.
    current: Token,
    /// Soft-ignored newlines inside `(...)`, `[...]`, `{...}`.
    delim_depth: u32,
    sink: &'parse mut dyn DiagSink,
    src: &'src str,
    base: u32,
}

impl<'arena, 'parse, 'src> Parser<'arena, 'parse, 'src> {
    fn new(
        arena: &'arena AstArena,
        interner: &'parse mut Interner,
        src: &'src str,
        base: u32,
        sink: &'parse mut dyn DiagSink,
    ) -> Self {
        let mut lexer = Lexer::new(src, base);
        let current = Self::next_significant(&mut lexer, sink, 0);
        Self {
            arena,
            interner,
            lexer,
            current,
            delim_depth: 0,
            sink,
            src,
            base,
        }
    }

    fn next_significant(
        lexer: &mut Lexer<'src>,
        sink: &mut dyn DiagSink,
        delim_depth: u32,
    ) -> Token {
        loop {
            let tok = lexer.next_token(sink);
            let skip = matches!(tok.kind, TokenKind::Whitespace | TokenKind::Comment)
                || (tok.kind == TokenKind::Newline && delim_depth > 0);
            if !skip {
                return tok;
            }
        }
    }

    fn bump(&mut self) -> Token {
        let prev = self.current;
        match prev.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                self.delim_depth += 1;
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                self.delim_depth = self.delim_depth.saturating_sub(1);
            }
            _ => {}
        }
        self.current = Self::next_significant(&mut self.lexer, self.sink, self.delim_depth);
        prev
    }

    fn peek(&self) -> Token {
        self.current
    }

    /// Next significant token after `current` (does not mutate parser state).
    fn peek_next(&self) -> Token {
        struct Discard;
        impl DiagSink for Discard {
            fn emit(&mut self, _diag: rynix_diag::Diagnostic) {}
        }
        let mut lexer = self.lexer.clone();
        let mut sink = Discard;
        Self::next_significant(&mut lexer, &mut sink, self.delim_depth)
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current.kind == kind
    }

    fn at_any(&self, kinds: &[TokenKind]) -> bool {
        kinds.contains(&self.current.kind)
    }

    fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump())
        } else {
            None
        }
    }

    fn expect(&mut self, kind: TokenKind, label: &str) -> Token {
        if self.at(kind) {
            return self.bump();
        }
        let found = self.current;
        let insert = kind.spelling().map(str::to_string);
        self.sink.emit(parse_errors::expected_token(
            found.span,
            label,
            found.kind,
            insert.as_deref(),
        ));
        Token::new(kind, Span::empty(found.span.lo()))
    }

    fn text(&self, span: Span) -> &str {
        let lo = (span.lo() - self.base) as usize;
        let hi = (span.hi() - self.base) as usize;
        &self.src[lo..hi]
    }

    fn error_node(&self, span: Span) -> rynix_ast::ErrorNode {
        rynix_ast::ErrorNode {
            id: self.arena.next_id(),
            span,
        }
    }

    fn skip_newlines(&mut self) {
        while self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    /// Panic-mode sync: skip until a statement/item boundary.
    fn sync_stmt(&mut self) {
        while !self.at_any(&[
            TokenKind::Newline,
            TokenKind::End,
            TokenKind::Def,
            TokenKind::Struct,
            TokenKind::Enum,
            TokenKind::Type,
            TokenKind::Import,
            TokenKind::Eof,
            TokenKind::Elif,
            TokenKind::Else,
        ]) {
            self.bump();
        }
        if self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    fn sync_item(&mut self) {
        while !self.at_any(&[
            TokenKind::Def,
            TokenKind::Struct,
            TokenKind::Enum,
            TokenKind::Type,
            TokenKind::Import,
            TokenKind::Pub,
            TokenKind::Eof,
            TokenKind::DocComment,
        ]) {
            if self.at(TokenKind::Newline) {
                self.bump();
                if self.at_any(&[
                    TokenKind::Def,
                    TokenKind::Struct,
                    TokenKind::Enum,
                    TokenKind::Type,
                    TokenKind::Import,
                    TokenKind::Pub,
                    TokenKind::DocComment,
                    TokenKind::Eof,
                ]) {
                    break;
                }
            } else {
                self.bump();
            }
        }
    }

    fn parse_module(&mut self) -> &'arena Module<'arena> {
        let start = self.base;
        self.skip_newlines();
        let mut items = Vec::new();
        while !self.at(TokenKind::Eof) {
            if self.at(TokenKind::Newline) {
                self.bump();
                continue;
            }
            items.push(self.parse_item());
            self.skip_newlines();
        }
        let end = self.current.span.hi();
        self.arena.alloc(Module {
            items: self.arena.alloc_slice(items),
            span: Span::new(start, end),
        })
    }
}
