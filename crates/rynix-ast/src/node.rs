//! AST node definitions for Rynix v0.1.
//!
//! Every node carries a [`Span`] covering its source extent and, where useful
//! for later `SoA` tables, a [`NodeId`]. Child lists are arena slices.

use rynix_span::{Span, Symbol};

use crate::NodeId;

/// An identifier: interned spelling plus its source span.
#[derive(Clone, Copy, Debug)]
pub struct Ident {
    pub name: Symbol,
    pub span: Span,
}

/// `pub` vs private. Items default to private.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Visibility {
    Private,
    Pub,
}

/// Top-level compilation unit.
#[derive(Debug)]
pub struct Module<'a> {
    pub items: &'a [Item<'a>],
    pub span: Span,
}

/// A top-level declaration.
#[derive(Debug)]
pub enum Item<'a> {
    Fn(FnDef<'a>),
    Struct(StructDef<'a>),
    Enum(EnumDef<'a>),
    TypeAlias(TypeAlias<'a>),
    Import(Import<'a>),
    /// Placeholder inserted by error recovery so the tree stays total.
    Error(ErrorNode),
}

impl Item<'_> {
    pub fn span(&self) -> Span {
        match self {
            Item::Fn(n) => n.span,
            Item::Struct(n) => n.span,
            Item::Enum(n) => n.span,
            Item::TypeAlias(n) => n.span,
            Item::Import(n) => n.span,
            Item::Error(n) => n.span,
        }
    }

    pub fn id(&self) -> NodeId {
        match self {
            Item::Fn(n) => n.id,
            Item::Struct(n) => n.id,
            Item::Enum(n) => n.id,
            Item::TypeAlias(n) => n.id,
            Item::Import(n) => n.id,
            Item::Error(n) => n.id,
        }
    }
}

/// Placeholder for a region the parser could not recover into a real node.
#[derive(Clone, Copy, Debug)]
pub struct ErrorNode {
    pub id: NodeId,
    pub span: Span,
}

#[derive(Debug)]
pub struct FnDef<'a> {
    pub id: NodeId,
    pub visibility: Visibility,
    pub name: Ident,
    pub params: &'a [Param<'a>],
    pub ret: Option<&'a Type<'a>>,
    pub body: &'a [Stmt<'a>],
    /// Span of a preceding `##` doc comment, if any.
    pub doc: Option<Span>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Param<'a> {
    pub name: Ident,
    pub ty: &'a Type<'a>,
    pub span: Span,
}

#[derive(Debug)]
pub struct StructDef<'a> {
    pub id: NodeId,
    pub visibility: Visibility,
    pub name: Ident,
    pub fields: &'a [Field<'a>],
    pub doc: Option<Span>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Field<'a> {
    pub name: Ident,
    pub ty: &'a Type<'a>,
    pub span: Span,
}

#[derive(Debug)]
pub struct EnumDef<'a> {
    pub id: NodeId,
    pub visibility: Visibility,
    pub name: Ident,
    pub variants: &'a [Variant<'a>],
    pub doc: Option<Span>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Variant<'a> {
    pub name: Ident,
    pub payload: Option<&'a Type<'a>>,
    pub span: Span,
}

#[derive(Debug)]
pub struct TypeAlias<'a> {
    pub id: NodeId,
    pub name: Ident,
    pub ty: &'a Type<'a>,
    pub doc: Option<Span>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Import<'a> {
    pub id: NodeId,
    pub path: &'a Path<'a>,
    pub span: Span,
}

/// A type expression.
#[derive(Debug)]
pub enum Type<'a> {
    Path(&'a Path<'a>),
    /// `Vec[i64]` / `Map[i64, i64]` — applied type constructor.
    App {
        path: &'a Path<'a>,
        args: &'a [&'a Type<'a>],
        span: Span,
    },
    /// `[T]` — contiguous slice of `T`.
    Slice(&'a Type<'a>, Span),
    Error(ErrorNode),
}

impl Type<'_> {
    pub fn span(&self) -> Span {
        match self {
            Type::Path(p) => p.span,
            Type::App { span, .. } | Type::Slice(_, span) => *span,
            Type::Error(n) => n.span,
        }
    }
}

/// `Ident { "::" Ident }` — used for both types and value paths.
#[derive(Debug)]
pub struct Path<'a> {
    pub id: NodeId,
    pub segments: &'a [Ident],
    pub span: Span,
}

/// A statement inside a function / block body.
#[derive(Debug)]
pub enum Stmt<'a> {
    Let(LetStmt<'a>),
    Assign(AssignStmt<'a>),
    Return(ReturnStmt<'a>),
    Break(BreakStmt),
    Continue(ContinueStmt),
    Loop(LoopStmt<'a>),
    For(ForStmt<'a>),
    If(IfStmt<'a>),
    Match(MatchStmt<'a>),
    /// Explicit bump-region scope: `region … end` (maps to RegionCreate/Reset).
    Region(RegionStmt<'a>),
    Expr(ExprStmt<'a>),
    Error(ErrorNode),
}

impl Stmt<'_> {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let(n) => n.span,
            Stmt::Assign(n) => n.span,
            Stmt::Return(n) => n.span,
            Stmt::Break(n) => n.span,
            Stmt::Continue(n) => n.span,
            Stmt::Loop(n) => n.span,
            Stmt::For(n) => n.span,
            Stmt::If(n) => n.span,
            Stmt::Match(n) => n.span,
            Stmt::Region(n) => n.span,
            Stmt::Expr(n) => n.span,
            Stmt::Error(n) => n.span,
        }
    }

    pub fn id(&self) -> NodeId {
        match self {
            Stmt::Let(n) => n.id,
            Stmt::Assign(n) => n.id,
            Stmt::Return(n) => n.id,
            Stmt::Break(n) => n.id,
            Stmt::Continue(n) => n.id,
            Stmt::Loop(n) => n.id,
            Stmt::For(n) => n.id,
            Stmt::If(n) => n.id,
            Stmt::Match(n) => n.id,
            Stmt::Region(n) => n.id,
            Stmt::Expr(n) => n.id,
            Stmt::Error(n) => n.id,
        }
    }
}

#[derive(Debug)]
pub struct LetStmt<'a> {
    pub id: NodeId,
    pub mutable: bool,
    pub name: Ident,
    pub ty: Option<&'a Type<'a>>,
    pub init: &'a Expr<'a>,
    pub span: Span,
}

/// Assignment is a statement, never an expression (SPEC §3).
#[derive(Debug)]
pub struct AssignStmt<'a> {
    pub id: NodeId,
    pub target: &'a Expr<'a>,
    pub op: AssignOp,
    pub value: &'a Expr<'a>,
    pub span: Span,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AssignOp {
    Eq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
}

impl AssignOp {
    pub fn as_str(self) -> &'static str {
        match self {
            AssignOp::Eq => "=",
            AssignOp::PlusEq => "+=",
            AssignOp::MinusEq => "-=",
            AssignOp::StarEq => "*=",
            AssignOp::SlashEq => "/=",
            AssignOp::PercentEq => "%=",
        }
    }
}

#[derive(Debug)]
pub struct ReturnStmt<'a> {
    pub id: NodeId,
    pub value: Option<&'a Expr<'a>>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug)]
pub struct BreakStmt {
    pub id: NodeId,
    pub span: Span,
}

#[derive(Clone, Copy, Debug)]
pub struct ContinueStmt {
    pub id: NodeId,
    pub span: Span,
}

#[derive(Debug)]
pub struct LoopStmt<'a> {
    pub id: NodeId,
    pub body: &'a [Stmt<'a>],
    pub span: Span,
}

/// Explicit region scope — body allocations prefer this bump arena.
#[derive(Debug)]
pub struct RegionStmt<'a> {
    pub id: NodeId,
    pub body: &'a [Stmt<'a>],
    pub span: Span,
}

#[derive(Debug)]
pub struct ForStmt<'a> {
    pub id: NodeId,
    pub binder: Ident,
    pub iter: &'a Expr<'a>,
    pub body: &'a [Stmt<'a>],
    pub span: Span,
}

#[derive(Debug)]
pub struct IfStmt<'a> {
    pub id: NodeId,
    pub arms: &'a [IfArm<'a>],
    pub else_body: Option<&'a [Stmt<'a>]>,
    pub span: Span,
}

/// One `if` / `elif` arm.
#[derive(Debug)]
pub struct IfArm<'a> {
    pub cond: &'a Expr<'a>,
    pub body: &'a [Stmt<'a>],
}

/// `match scrutinee` / pattern arms / optional `else` / `end`.
#[derive(Debug)]
pub struct MatchStmt<'a> {
    pub id: NodeId,
    pub scrutinee: &'a Expr<'a>,
    pub arms: &'a [MatchArm<'a>],
    pub else_body: Option<&'a [Stmt<'a>]>,
    pub span: Span,
}

#[derive(Debug)]
pub struct MatchArm<'a> {
    pub pattern: MatchPat<'a>,
    pub body: &'a [Stmt<'a>],
}

#[derive(Debug)]
pub enum MatchPat<'a> {
    /// Integer / bool / nil literal.
    Literal(&'a Expr<'a>),
    /// `_` wildcard.
    Wildcard(Span),
}

#[derive(Debug)]
pub struct ExprStmt<'a> {
    pub id: NodeId,
    pub expr: &'a Expr<'a>,
    pub span: Span,
}

/// An expression.
#[derive(Debug)]
pub enum Expr<'a> {
    Literal(LiteralExpr),
    Path(&'a Path<'a>),
    Unary(UnaryExpr<'a>),
    Binary(BinaryExpr<'a>),
    Cast(CastExpr<'a>),
    Call(CallExpr<'a>),
    MethodCall(MethodCallExpr<'a>),
    Index(IndexExpr<'a>),
    Field(FieldExpr<'a>),
    Array(ArrayExpr<'a>),
    Spawn(SpawnExpr<'a>),
    Error(ErrorNode),
}

impl Expr<'_> {
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal(n) => n.span,
            Expr::Path(n) => n.span,
            Expr::Unary(n) => n.span,
            Expr::Binary(n) => n.span,
            Expr::Cast(n) => n.span,
            Expr::Call(n) => n.span,
            Expr::MethodCall(n) => n.span,
            Expr::Index(n) => n.span,
            Expr::Field(n) => n.span,
            Expr::Array(n) => n.span,
            Expr::Spawn(n) => n.span,
            Expr::Error(n) => n.span,
        }
    }

    pub fn id(&self) -> NodeId {
        match self {
            Expr::Literal(n) => n.id,
            Expr::Path(n) => n.id,
            Expr::Unary(n) => n.id,
            Expr::Binary(n) => n.id,
            Expr::Cast(n) => n.id,
            Expr::Call(n) => n.id,
            Expr::MethodCall(n) => n.id,
            Expr::Index(n) => n.id,
            Expr::Field(n) => n.id,
            Expr::Array(n) => n.id,
            Expr::Spawn(n) => n.id,
            Expr::Error(n) => n.id,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LiteralExpr {
    pub id: NodeId,
    pub kind: LiteralKind,
    /// Parsed integer value when `kind == Int` (for compile-time checks).
    pub int_value: Option<i64>,
    pub span: Span,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LiteralKind {
    Int,
    Float,
    Str,
    True,
    False,
    Nil,
}

#[derive(Debug)]
pub struct UnaryExpr<'a> {
    pub id: NodeId,
    pub op: UnaryOp,
    pub operand: &'a Expr<'a>,
    pub span: Span,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnaryOp {
    Neg,
    Not,
}

impl UnaryOp {
    pub fn as_str(self) -> &'static str {
        match self {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "not",
        }
    }
}

#[derive(Debug)]
pub struct BinaryExpr<'a> {
    pub id: NodeId,
    pub op: BinaryOp,
    pub lhs: &'a Expr<'a>,
    pub rhs: &'a Expr<'a>,
    pub span: Span,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinaryOp {
    Or,
    And,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    DotDot,
    DotDotEq,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Amp,
    Shr,
    /// `lhs |> f` / `lhs |> f(a)` — pipeline into a call (SPEC §3.2).
    Pipe,
}

impl BinaryOp {
    pub fn as_str(self) -> &'static str {
        match self {
            BinaryOp::Or => "or",
            BinaryOp::And => "and",
            BinaryOp::EqEq => "==",
            BinaryOp::BangEq => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::LtEq => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::GtEq => ">=",
            BinaryOp::DotDot => "..",
            BinaryOp::DotDotEq => "..=",
            BinaryOp::Plus => "+",
            BinaryOp::Minus => "-",
            BinaryOp::Star => "*",
            BinaryOp::Slash => "/",
            BinaryOp::Percent => "%",
            BinaryOp::Amp => "&",
            BinaryOp::Shr => ">>",
            BinaryOp::Pipe => "|>",
        }
    }

    /// Comparisons are non-associative (SPEC §3).
    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            BinaryOp::EqEq
                | BinaryOp::BangEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq
        )
    }
}

#[derive(Debug)]
pub struct CastExpr<'a> {
    pub id: NodeId,
    pub expr: &'a Expr<'a>,
    pub ty: &'a Type<'a>,
    pub span: Span,
}

#[derive(Debug)]
pub struct CallExpr<'a> {
    pub id: NodeId,
    pub callee: &'a Expr<'a>,
    pub args: &'a [&'a Expr<'a>],
    pub span: Span,
}

#[derive(Debug)]
pub struct MethodCallExpr<'a> {
    pub id: NodeId,
    pub receiver: &'a Expr<'a>,
    pub method: Ident,
    pub args: &'a [&'a Expr<'a>],
    pub span: Span,
}

#[derive(Debug)]
pub struct IndexExpr<'a> {
    pub id: NodeId,
    pub base: &'a Expr<'a>,
    pub index: &'a Expr<'a>,
    pub span: Span,
}

#[derive(Debug)]
pub struct FieldExpr<'a> {
    pub id: NodeId,
    pub base: &'a Expr<'a>,
    pub field: Ident,
    pub span: Span,
}

#[derive(Debug)]
pub struct ArrayExpr<'a> {
    pub id: NodeId,
    pub elems: &'a [&'a Expr<'a>],
    pub span: Span,
}

/// `spawn callee(...)` — colorless concurrency primitive.
#[derive(Debug)]
pub struct SpawnExpr<'a> {
    pub id: NodeId,
    pub callee: &'a Expr<'a>,
    pub span: Span,
}
