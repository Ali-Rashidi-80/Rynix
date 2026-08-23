//! Pratt expression parser.
//!
//! Precedence (weakest → strongest), matching SPEC §3:
//! `or < and < not < cmp < range < add < mul < unary - < as < postfix`.

use rynix_ast::{
    ArrayExpr, BinaryExpr, BinaryOp, CallExpr, CastExpr, Expr, FieldExpr, IndexExpr, LiteralExpr,
    LiteralKind, MethodCallExpr, SpawnExpr, UnaryExpr, UnaryOp,
};
use rynix_lexer::TokenKind;
use rynix_span::Span;

use crate::Parser;
use crate::errors as parse_errors;

#[derive(Clone, Copy)]
struct BindingPower {
    left: u8,
    right: u8,
}

impl<'arena> Parser<'arena, '_, '_> {
    pub(crate) fn parse_expr(&mut self) -> &'arena Expr<'arena> {
        self.parse_bp(0)
    }

    fn parse_bp(&mut self, min_bp: u8) -> &'arena Expr<'arena> {
        let mut lhs = self.parse_prefix();

        loop {
            // `as` cast: binding power above unary, below postfix... SPEC says
            // as is 9, postfix is 10. So as is infix-ish with right bp.
            if self.at(TokenKind::As) {
                let (l_bp, r_bp) = (18, 19); // between unary(16) and postfix(20)
                if l_bp < min_bp {
                    break;
                }
                self.bump();
                let ty = self.parse_type();
                let span = lhs.span().to(ty.span());
                lhs = self.arena.alloc(Expr::Cast(CastExpr {
                    id: self.arena.next_id(),
                    expr: lhs,
                    ty,
                    span,
                }));
                let _ = r_bp;
                continue;
            }

            if let Some((op, bp)) = self.infix_op() {
                if bp.left < min_bp {
                    break;
                }
                if op.is_comparison() && matches!(lhs, Expr::Binary(b) if b.op.is_comparison()) {
                    let tok = self.peek();
                    self.sink
                        .emit(parse_errors::chained_comparison(lhs.span().to(tok.span)));
                }
                self.bump();
                let rhs = self.parse_bp(bp.right);
                let span = lhs.span().to(rhs.span());
                lhs = self.arena.alloc(Expr::Binary(BinaryExpr {
                    id: self.arena.next_id(),
                    op,
                    lhs,
                    rhs,
                    span,
                }));
                continue;
            }

            // Postfix: call, index, field / method.
            if self.at_any(&[TokenKind::LParen, TokenKind::LBracket, TokenKind::Dot]) {
                const POSTFIX_BP: u8 = 20;
                if POSTFIX_BP < min_bp {
                    break;
                }
                lhs = self.parse_postfix(lhs);
                continue;
            }

            break;
        }
        lhs
    }

    fn parse_prefix(&mut self) -> &'arena Expr<'arena> {
        match self.peek().kind {
            TokenKind::Not => {
                let start = self.bump().span;
                let operand = self.parse_bp(17); // unary bp
                let span = start.to(operand.span());
                self.arena.alloc(Expr::Unary(UnaryExpr {
                    id: self.arena.next_id(),
                    op: UnaryOp::Not,
                    operand,
                    span,
                }))
            }
            TokenKind::Minus => {
                let start = self.bump().span;
                let operand = self.parse_bp(17);
                let span = start.to(operand.span());
                self.arena.alloc(Expr::Unary(UnaryExpr {
                    id: self.arena.next_id(),
                    op: UnaryOp::Neg,
                    operand,
                    span,
                }))
            }
            TokenKind::Spawn => {
                let start = self.bump().span;
                let callee = self.parse_bp(20); // tightly bind to the call
                let span = start.to(callee.span());
                self.arena.alloc(Expr::Spawn(SpawnExpr {
                    id: self.arena.next_id(),
                    callee,
                    span,
                }))
            }
            _ => self.parse_primary(),
        }
    }

    pub(crate) fn parse_primary(&mut self) -> &'arena Expr<'arena> {
        match self.peek().kind {
            TokenKind::IntLit => self.literal(LiteralKind::Int),
            TokenKind::FloatLit => self.literal(LiteralKind::Float),
            TokenKind::StrLit => self.literal(LiteralKind::Str),
            TokenKind::True => self.literal(LiteralKind::True),
            TokenKind::False => self.literal(LiteralKind::False),
            TokenKind::Nil => self.literal(LiteralKind::Nil),
            TokenKind::Ident
            | TokenKind::Agent
            | TokenKind::Signal
            | TokenKind::Tensor => {
                let path = self.parse_path_ext(true);
                self.arena.alloc(Expr::Path(path))
            }
            TokenKind::LParen => {
                let open = self.bump().span;
                let inner = self.parse_expr();
                if self.at(TokenKind::RParen) {
                    self.bump();
                } else {
                    self.sink.emit(parse_errors::unclosed_delimiter(open, ")"));
                }
                inner
            }
            TokenKind::LBracket => self.parse_array(),
            TokenKind::Eof => {
                let span = self.peek().span;
                self.sink
                    .emit(parse_errors::unexpected_eof(span, "an expression"));
                self.arena.alloc(Expr::Error(self.error_node(span)))
            }
            _ => {
                let tok = self.bump();
                self.sink
                    .emit(parse_errors::unexpected_token(tok.span, tok.kind));
                self.arena.alloc(Expr::Error(self.error_node(tok.span)))
            }
        }
    }

    fn literal(&mut self, kind: LiteralKind) -> &'arena Expr<'arena> {
        let tok = self.bump();
        let int_value = if kind == LiteralKind::Int {
            parse_int_lit(self.text(tok.span))
        } else {
            None
        };
        self.arena.alloc(Expr::Literal(LiteralExpr {
            id: self.arena.next_id(),
            kind,
            int_value,
            span: tok.span,
        }))
    }

    fn parse_array(&mut self) -> &'arena Expr<'arena> {
        let start = self.bump().span;
        let mut elems = Vec::new();
        if !self.at(TokenKind::RBracket) {
            loop {
                elems.push(self.parse_expr());
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
                if self.at(TokenKind::RBracket) {
                    break;
                }
            }
        }
        let end = if self.at(TokenKind::RBracket) {
            self.bump().span
        } else {
            self.sink.emit(parse_errors::unclosed_delimiter(start, "]"));
            Span::empty(self.peek().span.lo())
        };
        self.arena.alloc(Expr::Array(ArrayExpr {
            id: self.arena.next_id(),
            elems: self.arena.alloc_slice(elems),
            span: start.to(end),
        }))
    }

    fn parse_postfix(&mut self, mut lhs: &'arena Expr<'arena>) -> &'arena Expr<'arena> {
        match self.peek().kind {
            TokenKind::LParen => {
                let open = self.bump().span;
                let mut args = Vec::new();
                if !self.at(TokenKind::RParen) {
                    loop {
                        args.push(self.parse_expr());
                        if self.eat(TokenKind::Comma).is_none() {
                            break;
                        }
                        if self.at(TokenKind::RParen) {
                            break;
                        }
                    }
                }
                let end = if self.at(TokenKind::RParen) {
                    self.bump().span
                } else {
                    self.sink.emit(parse_errors::unclosed_delimiter(open, ")"));
                    Span::empty(self.peek().span.lo())
                };
                lhs = self.arena.alloc(Expr::Call(CallExpr {
                    id: self.arena.next_id(),
                    callee: lhs,
                    args: self.arena.alloc_slice(args),
                    span: lhs.span().to(end),
                }));
            }
            TokenKind::LBracket => {
                let open = self.bump().span;
                let index = self.parse_expr();
                let end = if self.at(TokenKind::RBracket) {
                    self.bump().span
                } else {
                    self.sink.emit(parse_errors::unclosed_delimiter(open, "]"));
                    Span::empty(self.peek().span.lo())
                };
                lhs = self.arena.alloc(Expr::Index(IndexExpr {
                    id: self.arena.next_id(),
                    base: lhs,
                    index,
                    span: lhs.span().to(end),
                }));
            }
            TokenKind::Dot => {
                self.bump();
                let name = self.expect_ident("field or method name");
                if self.at(TokenKind::LParen) {
                    let open = self.bump().span;
                    let mut args = Vec::new();
                    if !self.at(TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expr());
                            if self.eat(TokenKind::Comma).is_none() {
                                break;
                            }
                            if self.at(TokenKind::RParen) {
                                break;
                            }
                        }
                    }
                    let end = if self.at(TokenKind::RParen) {
                        self.bump().span
                    } else {
                        self.sink.emit(parse_errors::unclosed_delimiter(open, ")"));
                        Span::empty(self.peek().span.lo())
                    };
                    lhs = self.arena.alloc(Expr::MethodCall(MethodCallExpr {
                        id: self.arena.next_id(),
                        receiver: lhs,
                        method: name,
                        args: self.arena.alloc_slice(args),
                        span: lhs.span().to(end),
                    }));
                } else {
                    let span = lhs.span().to(name.span);
                    lhs = self.arena.alloc(Expr::Field(FieldExpr {
                        id: self.arena.next_id(),
                        base: lhs,
                        field: name,
                        span,
                    }));
                }
            }
            _ => {}
        }
        lhs
    }

    fn infix_op(&self) -> Option<(BinaryOp, BindingPower)> {
        let (op, left, right) = match self.peek().kind {
            TokenKind::Or => (BinaryOp::Or, 1, 2),
            TokenKind::And => (BinaryOp::And, 3, 4),
            TokenKind::Pipe => (BinaryOp::Pipe, 0, 1), // loosest; left-assoc via Pratt
            // Comparisons: structurally left-associative so the chain check
            // sees `(a < b) < c`; we then reject that shape (SPEC: non-associative).
            TokenKind::EqEq => (BinaryOp::EqEq, 5, 6),
            TokenKind::BangEq => (BinaryOp::BangEq, 5, 6),
            TokenKind::Lt => (BinaryOp::Lt, 5, 6),
            TokenKind::LtEq => (BinaryOp::LtEq, 5, 6),
            TokenKind::Gt => (BinaryOp::Gt, 5, 6),
            TokenKind::GtEq => (BinaryOp::GtEq, 5, 6),
            TokenKind::DotDot => (BinaryOp::DotDot, 7, 8),
            TokenKind::DotDotEq => (BinaryOp::DotDotEq, 7, 8),
            TokenKind::Plus => (BinaryOp::Plus, 9, 10),
            TokenKind::Minus => (BinaryOp::Minus, 9, 10),
            TokenKind::Star => (BinaryOp::Star, 11, 12),
            TokenKind::Slash => (BinaryOp::Slash, 11, 12),
            TokenKind::Percent => (BinaryOp::Percent, 11, 12),
            TokenKind::Shr => (BinaryOp::Shr, 13, 14),
            TokenKind::Amp => (BinaryOp::Amp, 15, 16),
            _ => return None,
        };
        Some((op, BindingPower { left, right }))
    }
}

fn parse_int_lit(text: &str) -> Option<i64> {
    let t = text.trim().replace('_', "");
    if let Some(rest) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        i64::from_str_radix(rest, 16).ok()
    } else if let Some(rest) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        i64::from_str_radix(rest, 8).ok()
    } else if let Some(rest) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        i64::from_str_radix(rest, 2).ok()
    } else {
        t.parse().ok()
    }
}

