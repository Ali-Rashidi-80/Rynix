use rynix_ast::{
    EnumDef, Field, FnDef, Ident, Import, Item, Param, StructDef, TypeAlias, Variant, Visibility,
};
use rynix_lexer::TokenKind;
use rynix_span::Span;

use crate::Parser;
use crate::errors as parse_errors;

impl<'arena> Parser<'arena, '_, '_> {
    pub(crate) fn parse_item(&mut self) -> Item<'arena> {
        let doc = self.eat_doc();
        let visibility = if self.eat(TokenKind::Pub).is_some() {
            Visibility::Pub
        } else {
            Visibility::Private
        };

        match self.peek().kind {
            TokenKind::Def => Item::Fn(self.parse_fn(visibility, doc)),
            TokenKind::Struct => Item::Struct(self.parse_struct(visibility, doc)),
            TokenKind::Enum => Item::Enum(self.parse_enum(visibility, doc)),
            TokenKind::Type => {
                if visibility == Visibility::Pub {
                    self.sink.emit(parse_errors::unexpected_token(
                        self.peek().span,
                        TokenKind::Pub,
                    ));
                }
                Item::TypeAlias(self.parse_type_alias(doc))
            }
            TokenKind::Import => {
                if visibility == Visibility::Pub {
                    self.sink.emit(parse_errors::unexpected_token(
                        self.peek().span,
                        TokenKind::Pub,
                    ));
                }
                Item::Import(self.parse_import())
            }
            _ => {
                let tok = self.bump();
                self.sink
                    .emit(parse_errors::unexpected_token(tok.span, tok.kind));
                self.sync_item();
                Item::Error(self.error_node(tok.span))
            }
        }
    }

    fn eat_doc(&mut self) -> Option<Span> {
        let mut doc: Option<Span> = None;
        while self.at(TokenKind::DocComment) {
            let tok = self.bump();
            doc = Some(match doc {
                Some(prev) => prev.to(tok.span),
                None => tok.span,
            });
            self.skip_newlines();
        }
        doc
    }

    fn parse_fn(&mut self, visibility: Visibility, doc: Option<Span>) -> FnDef<'arena> {
        let start = self.bump().span; // `def`
        let name = self.expect_ident("function name");
        self.expect(TokenKind::LParen, "`(`");
        let params = self.parse_params();
        self.expect(TokenKind::RParen, "`)`");
        let ret = if self.eat(TokenKind::Arrow).is_some() {
            Some(self.parse_type())
        } else {
            None
        };
        self.expect_newline_or_end_header();
        let body = self.parse_block_until_end(start);
        let end = self.expect_end(start);
        FnDef {
            id: self.arena.next_id(),
            visibility,
            name,
            params,
            ret,
            body,
            doc,
            span: start.to(end),
        }
    }

    fn parse_params(&mut self) -> &'arena [Param<'arena>] {
        let mut params = Vec::new();
        if self.at(TokenKind::RParen) {
            return self.arena.alloc_slice(params);
        }
        loop {
            let name = self.expect_ident("parameter name");
            self.expect(TokenKind::Colon, "`:`");
            let ty = self.parse_type();
            params.push(Param {
                span: name.span.to(ty.span()),
                name,
                ty,
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
            if self.at(TokenKind::RParen) {
                break;
            }
        }
        self.arena.alloc_slice(params)
    }

    fn parse_struct(&mut self, visibility: Visibility, doc: Option<Span>) -> StructDef<'arena> {
        let start = self.bump().span; // `struct`
        let name = self.expect_ident("struct name");
        self.expect_newline_or_end_header();
        let mut fields = Vec::new();
        while !self.at_any(&[TokenKind::End, TokenKind::Eof]) {
            if self.at(TokenKind::Newline) {
                self.bump();
                continue;
            }
            let field_name = self.expect_ident("field name");
            self.expect(TokenKind::Colon, "`:`");
            let ty = self.parse_type();
            fields.push(Field {
                span: field_name.span.to(ty.span()),
                name: field_name,
                ty,
            });
            self.expect_newline_soft();
        }
        let end = self.expect_end(start);
        StructDef {
            id: self.arena.next_id(),
            visibility,
            name,
            fields: self.arena.alloc_slice(fields),
            doc,
            span: start.to(end),
        }
    }

    fn parse_enum(&mut self, visibility: Visibility, doc: Option<Span>) -> EnumDef<'arena> {
        let start = self.bump().span; // `enum`
        let name = self.expect_ident("enum name");
        self.expect_newline_or_end_header();
        let mut variants = Vec::new();
        while !self.at_any(&[TokenKind::End, TokenKind::Eof]) {
            if self.at(TokenKind::Newline) {
                self.bump();
                continue;
            }
            let vname = self.expect_ident("variant name");
            let mut span = vname.span;
            let payload = if self.eat(TokenKind::LParen).is_some() {
                let ty = self.parse_type();
                let close = self.expect(TokenKind::RParen, "`)`");
                span = span.to(close.span);
                Some(ty)
            } else {
                None
            };
            variants.push(Variant {
                name: vname,
                payload,
                span,
            });
            self.expect_newline_soft();
        }
        let end = self.expect_end(start);
        EnumDef {
            id: self.arena.next_id(),
            visibility,
            name,
            variants: self.arena.alloc_slice(variants),
            doc,
            span: start.to(end),
        }
    }

    fn parse_type_alias(&mut self, doc: Option<Span>) -> TypeAlias<'arena> {
        let start = self.bump().span; // `type`
        let name = self.expect_ident("type name");
        self.expect(TokenKind::Eq, "`=`");
        let ty = self.parse_type();
        let end = ty.span();
        self.expect_newline_soft();
        TypeAlias {
            id: self.arena.next_id(),
            name,
            ty,
            doc,
            span: start.to(end),
        }
    }

    fn parse_import(&mut self) -> Import<'arena> {
        let start = self.bump().span; // `import`
        let path = self.parse_path();
        let end = path.span;
        self.expect_newline_soft();
        Import {
            id: self.arena.next_id(),
            path,
            span: start.to(end),
        }
    }

    pub(crate) fn expect_ident(&mut self, label: &str) -> Ident {
        self.expect_ident_ext(label, false)
    }

    pub(crate) fn expect_ident_ext(&mut self, label: &str, allow_reserved: bool) -> Ident {
        if self.at(TokenKind::Ident) {
            let tok = self.bump();
            let text = self.text(tok.span).to_string();
            return Ident {
                name: self.interner.intern(&text),
                span: tok.span,
            };
        }
        if self.peek().kind.is_reserved() {
            let tok = self.bump();
            let word = self.text(tok.span).to_string();
            if !allow_reserved {
                self.sink
                    .emit(parse_errors::reserved_keyword(tok.span, &word));
            }
            return Ident {
                name: self.interner.intern(&word),
                span: tok.span,
            };
        }
        // Keywords used as names: still recover with the spelling.
        if self.peek().kind.is_keyword() {
            let tok = self.bump();
            let text = self.text(tok.span).to_string();
            self.sink.emit(parse_errors::expected_token(
                tok.span, label, tok.kind, None,
            ));
            return Ident {
                name: self.interner.intern(&text),
                span: tok.span,
            };
        }
        let found = self.peek();
        self.sink.emit(parse_errors::expected_token(
            found.span, label, found.kind, None,
        ));
        Ident {
            name: self.interner.intern(""),
            span: Span::empty(found.span.lo()),
        }
    }

    pub(crate) fn expect_end(&mut self, open: Span) -> Span {
        if let Some(tok) = self.eat(TokenKind::End) {
            return tok.span;
        }
        let here = self.peek().span;
        self.sink.emit(parse_errors::missing_end(open, here));
        here
    }

    pub(crate) fn expect_newline_or_end_header(&mut self) {
        if self.at(TokenKind::Newline) {
            self.bump();
            return;
        }
        // Allow `def f() end` on one line — unusual but total.
        if self.at(TokenKind::End) {
            return;
        }
        let found = self.peek();
        self.sink.emit(parse_errors::expected_token(
            found.span,
            "newline after header",
            found.kind,
            Some("\n"),
        ));
    }

    fn expect_newline_soft(&mut self) {
        if self.at(TokenKind::Newline) {
            self.bump();
        } else if !self.at_any(&[
            TokenKind::End,
            TokenKind::Eof,
            TokenKind::Elif,
            TokenKind::Else,
        ]) {
            // tolerate missing newline before `end`
        }
    }
}
