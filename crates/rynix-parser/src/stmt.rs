use rynix_ast::{
    AssignOp, AssignStmt, BreakStmt, ContinueStmt, ExprStmt, ForStmt, IfArm, IfStmt, LetStmt,
    LoopStmt, MatchArm, MatchPat, MatchStmt, ReturnStmt, Stmt,
};
use rynix_lexer::TokenKind;
use rynix_span::Span;

use crate::Parser;
use crate::errors as parse_errors;

impl<'arena> Parser<'arena, '_, '_> {
    /// Parses statements until `end` / `elif` / `else` / EOF.
    pub(crate) fn parse_block_until_end(&mut self, _open: Span) -> &'arena [Stmt<'arena>] {
        let mut stmts = Vec::new();
        while !self.at_any(&[
            TokenKind::End,
            TokenKind::Elif,
            TokenKind::Else,
            TokenKind::Eof,
        ]) {
            if self.at(TokenKind::Newline) {
                self.bump();
                continue;
            }
            // A sibling item starting mid-block: stop and let the outer
            // recovery report the missing `end`.
            if self.at_any(&[
                TokenKind::Def,
                TokenKind::Struct,
                TokenKind::Enum,
                TokenKind::Type,
                TokenKind::Import,
                TokenKind::Pub,
                TokenKind::DocComment,
            ]) {
                break;
            }
            stmts.push(self.parse_stmt());
        }
        self.arena.alloc_slice(stmts)
    }

    pub(crate) fn parse_stmt(&mut self) -> Stmt<'arena> {
        match self.peek().kind {
            TokenKind::Let => Stmt::Let(self.parse_let()),
            TokenKind::Return => Stmt::Return(self.parse_return()),
            TokenKind::Break => {
                let tok = self.bump();
                self.expect_newline_soft_stmt();
                Stmt::Break(BreakStmt {
                    id: self.arena.next_id(),
                    span: tok.span,
                })
            }
            TokenKind::Continue => {
                let tok = self.bump();
                self.expect_newline_soft_stmt();
                Stmt::Continue(ContinueStmt {
                    id: self.arena.next_id(),
                    span: tok.span,
                })
            }
            TokenKind::Loop => Stmt::Loop(self.parse_loop()),
            TokenKind::For => Stmt::For(self.parse_for()),
            TokenKind::If => Stmt::If(self.parse_if()),
            TokenKind::Match => Stmt::Match(self.parse_match()),
            _ => self.parse_expr_or_assign(),
        }
    }

    fn parse_let(&mut self) -> LetStmt<'arena> {
        let start = self.bump().span; // `let`
        let mutable = self.eat(TokenKind::Mut).is_some();
        let name = self.expect_ident("binding name");
        let ty = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type())
        } else {
            None
        };
        self.expect(TokenKind::Eq, "`=`");
        let init = self.parse_expr();
        let end = init.span();
        self.expect_newline_soft_stmt();
        LetStmt {
            id: self.arena.next_id(),
            mutable,
            name,
            ty,
            init,
            span: start.to(end),
        }
    }

    fn parse_return(&mut self) -> ReturnStmt<'arena> {
        let start = self.bump().span;
        let value = if self.at_any(&[TokenKind::Newline, TokenKind::End, TokenKind::Eof]) {
            None
        } else {
            Some(self.parse_expr())
        };
        let end = value.map_or(start, rynix_ast::Expr::span);
        self.expect_newline_soft_stmt();
        ReturnStmt {
            id: self.arena.next_id(),
            value,
            span: start.to(end),
        }
    }

    fn parse_loop(&mut self) -> LoopStmt<'arena> {
        let start = self.bump().span;
        self.expect_newline_or_end_header();
        let body = self.parse_block_until_end(start);
        let end = self.expect_end(start);
        LoopStmt {
            id: self.arena.next_id(),
            body,
            span: start.to(end),
        }
    }

    fn parse_for(&mut self) -> ForStmt<'arena> {
        let start = self.bump().span;
        let binder = self.expect_ident("loop binder");
        self.expect(TokenKind::In, "`in`");
        let iter = self.parse_expr();
        self.expect_newline_or_end_header();
        let body = self.parse_block_until_end(start);
        let end = self.expect_end(start);
        ForStmt {
            id: self.arena.next_id(),
            binder,
            iter,
            body,
            span: start.to(end),
        }
    }

    fn parse_if(&mut self) -> IfStmt<'arena> {
        let start = self.bump().span; // `if`
        let mut arms = Vec::new();
        let cond = self.parse_expr();
        self.expect_newline_or_end_header();
        let body = self.parse_block_until_end(start);
        arms.push(IfArm { cond, body });

        while self.at(TokenKind::Elif) {
            self.bump();
            let cond = self.parse_expr();
            self.expect_newline_or_end_header();
            let body = self.parse_block_until_end(start);
            arms.push(IfArm { cond, body });
        }

        let else_body = if self.at(TokenKind::Else) {
            self.bump();
            self.expect_newline_or_end_header();
            Some(self.parse_block_until_end(start))
        } else {
            None
        };

        let end = self.expect_end(start);
        IfStmt {
            id: self.arena.next_id(),
            arms: self.arena.alloc_slice(arms),
            else_body,
            span: start.to(end),
        }
    }

    fn parse_match(&mut self) -> MatchStmt<'arena> {
        let start = self.bump().span; // `match`
        let scrutinee = self.parse_expr();
        self.expect_newline_or_end_header();
        let mut arms = Vec::new();
        let mut else_body = None;
        loop {
            while self.at(TokenKind::Newline) {
                self.bump();
            }
            if self.at(TokenKind::End) || self.at(TokenKind::Eof) {
                break;
            }
            if self.at(TokenKind::Else) {
                self.bump();
                self.expect_newline_or_end_header();
                else_body = Some(self.parse_block_until_end(start));
                break;
            }
            if self.at_match_pattern() {
                let pattern = self.parse_match_pattern();
                self.expect_newline_or_end_header();
                let body = self.parse_block_until_match_boundary(start);
                arms.push(MatchArm { pattern, body });
                continue;
            }
            let found = self.peek();
            self.sink
                .emit(parse_errors::unexpected_token(found.span, found.kind));
            self.sync_stmt();
            break;
        }
        let end = self.expect_end(start);
        MatchStmt {
            id: self.arena.next_id(),
            scrutinee,
            arms: self.arena.alloc_slice(arms),
            else_body,
            span: start.to(end),
        }
    }

    fn at_match_pattern(&mut self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::IntLit | TokenKind::True | TokenKind::False
        ) || (self.at(TokenKind::Ident) && self.text(self.peek().span) == "_")
    }

    fn parse_match_pattern(&mut self) -> MatchPat<'arena> {
        if self.at(TokenKind::Ident) && self.text(self.peek().span) == "_" {
            let tok = self.bump();
            return MatchPat::Wildcard(tok.span);
        }
        // Literals only (primary) — not full expressions.
        let expr = self.parse_primary();
        MatchPat::Literal(expr)
    }

    /// Block body for a match arm — stops before the next pattern / else / end.
    fn parse_block_until_match_boundary(&mut self, open: Span) -> &'arena [Stmt<'arena>] {
        let mut stmts = Vec::new();
        while !self.at_any(&[TokenKind::End, TokenKind::Else, TokenKind::Eof]) {
            if self.at(TokenKind::Newline) {
                self.bump();
                // After newline, a pattern starts a sibling arm.
                if self.at_match_pattern() {
                    break;
                }
                continue;
            }
            if self.at_match_pattern() {
                break;
            }
            if self.at_any(&[
                TokenKind::Def,
                TokenKind::Struct,
                TokenKind::Enum,
                TokenKind::Type,
                TokenKind::Import,
                TokenKind::Pub,
                TokenKind::DocComment,
            ]) {
                break;
            }
            let _ = open;
            stmts.push(self.parse_stmt());
        }
        self.arena.alloc_slice(stmts)
    }

    fn parse_expr_or_assign(&mut self) -> Stmt<'arena> {
        let expr = self.parse_expr();
        if let Some(op) = self.assign_op() {
            self.bump();
            let value = self.parse_expr();
            let span = expr.span().to(value.span());
            self.expect_newline_soft_stmt();
            return Stmt::Assign(AssignStmt {
                id: self.arena.next_id(),
                target: expr,
                op,
                value,
                span,
            });
        }
        let span = expr.span();
        // Expression statements must end at a newline (or block closer).
        if !self.at_any(&[
            TokenKind::Newline,
            TokenKind::End,
            TokenKind::Elif,
            TokenKind::Else,
            TokenKind::Eof,
        ]) {
            let found = self.peek();
            self.sink
                .emit(parse_errors::unexpected_token(found.span, found.kind));
            self.sync_stmt();
        } else if self.at(TokenKind::Newline) {
            self.bump();
        }
        Stmt::Expr(ExprStmt {
            id: self.arena.next_id(),
            expr,
            span,
        })
    }

    fn assign_op(&self) -> Option<AssignOp> {
        Some(match self.peek().kind {
            TokenKind::Eq => AssignOp::Eq,
            TokenKind::PlusEq => AssignOp::PlusEq,
            TokenKind::MinusEq => AssignOp::MinusEq,
            TokenKind::StarEq => AssignOp::StarEq,
            TokenKind::SlashEq => AssignOp::SlashEq,
            TokenKind::PercentEq => AssignOp::PercentEq,
            _ => return None,
        })
    }

    fn expect_newline_soft_stmt(&mut self) {
        if self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    // Re-export for stmt module: item.rs defines expect_newline_or_end_header
    // as private; call through a thin wrapper if needed.
}
