use rynix_ast::Type;
use rynix_lexer::TokenKind;
use rynix_span::Span;

use crate::Parser;
use crate::errors as parse_errors;

impl<'arena> Parser<'arena, '_, '_> {
    pub(crate) fn parse_type(&mut self) -> &'arena Type<'arena> {
        if self.at(TokenKind::LBracket) {
            let start = self.bump().span;
            let inner = self.parse_type();
            let close = if self.at(TokenKind::RBracket) {
                self.bump().span
            } else {
                self.sink.emit(parse_errors::unclosed_delimiter(start, "]"));
                Span::empty(self.peek().span.lo())
            };
            return self.arena.alloc(Type::Slice(inner, start.to(close)));
        }
        if self.at(TokenKind::Ident) || self.peek().kind.is_reserved() {
            let path = self.parse_path();
            return self.arena.alloc(Type::Path(path));
        }
        let tok = self.peek();
        if tok.kind == TokenKind::Eof {
            self.sink
                .emit(parse_errors::unexpected_eof(tok.span, "a type"));
        } else {
            self.sink.emit(parse_errors::expected_token(
                tok.span, "a type", tok.kind, None,
            ));
            self.bump();
        }
        self.arena.alloc(Type::Error(self.error_node(tok.span)))
    }

    pub(crate) fn parse_path(&mut self) -> &'arena rynix_ast::Path<'arena> {
        self.parse_path_ext(false)
    }

    /// When `allow_reserved`, `tensor`/`signal`/`agent`/`match` may appear as
    /// path segments without emitting `RYX1005` (smart-primitive experiments).
    pub(crate) fn parse_path_ext(&mut self, allow_reserved: bool) -> &'arena rynix_ast::Path<'arena> {
        let first = self.expect_ident_ext("identifier", allow_reserved);
        let start = first.span;
        let mut segs = vec![first];
        while self.eat(TokenKind::ColonColon).is_some() {
            segs.push(self.expect_ident_ext("path segment", allow_reserved));
        }
        let end = segs.last().map_or(start, |s| s.span);
        self.arena.alloc(rynix_ast::Path {
            id: self.arena.next_id(),
            segments: self.arena.alloc_slice(segs),
            span: start.to(end),
        })
    }
}
