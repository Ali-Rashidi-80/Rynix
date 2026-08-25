//! Canonical source formatter — zero configuration (Phase 9).
//!
//! Pretty-prints a parsed module using fixed layout rules:
//! - 2-space indent; one statement per line;
//! - `def` / `end` blocks; blank line between top-level items;
//! - no trailing whitespace; final newline.

#![allow(clippy::too_many_lines)]

use std::fmt::Write as _;

use rynix_span::Interner;

use crate::node::{
    AssignOp, BinaryOp, EnumDef, Expr, FnDef, Ident, Item, LiteralKind, MatchPat, Module, Path,
    Stmt, StructDef, Type, TypeAlias, UnaryOp, Visibility,
};

/// Format `module` as canonical Rynix source.
pub fn format_module(
    module: &Module<'_>,
    interner: &Interner,
    src: &str,
    base: u32,
) -> String {
    let mut out = String::new();
    let mut f = Formatter {
        out: &mut out,
        interner,
        src,
        base,
        indent: 0,
    };
    let mut first = true;
    for item in module.items {
        if matches!(item, Item::Error(_)) {
            continue;
        }
        if !first {
            f.out.push('\n');
        }
        first = false;
        f.item(item);
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

struct Formatter<'a> {
    out: &'a mut String,
    interner: &'a Interner,
    src: &'a str,
    base: u32,
    indent: usize,
}

impl Formatter<'_> {
    fn line(&mut self, text: &str) {
        for _ in 0..self.indent {
            self.out.push_str("  ");
        }
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn name(&self, id: Ident) -> &str {
        self.interner.resolve(id.name)
    }

    fn span_text(&self, span: rynix_span::Span) -> &str {
        let lo = span.lo().saturating_sub(self.base) as usize;
        let hi = span.hi().saturating_sub(self.base) as usize;
        let hi = hi.min(self.src.len());
        let lo = lo.min(hi);
        &self.src[lo..hi]
    }

    fn item(&mut self, item: &Item<'_>) {
        match item {
            Item::Fn(f) => self.fn_def(f),
            Item::Struct(s) => self.struct_def(s),
            Item::Enum(e) => self.enum_def(e),
            Item::TypeAlias(t) => self.type_alias(t),
            Item::Import(i) => {
                self.line(&format!("import {}", self.path_str(i.path)));
            }
            Item::Error(_) => {}
        }
    }

    fn fn_def(&mut self, f: &FnDef<'_>) {
        let vis = match f.visibility {
            Visibility::Pub => "pub ",
            Visibility::Private => "",
        };
        let mut head = format!("{vis}def {}", self.name(f.name));
        head.push('(');
        for (i, p) in f.params.iter().enumerate() {
            if i > 0 {
                head.push_str(", ");
            }
            let _ = write!(head, "{}: {}", self.name(p.name), self.type_str(p.ty));
        }
        head.push(')');
        if let Some(ret) = f.ret {
            let _ = write!(head, " -> {}", self.type_str(ret));
        }
        self.line(&head);
        self.indent += 1;
        for s in f.body {
            self.stmt(s);
        }
        self.indent -= 1;
        self.line("end");
    }

    fn struct_def(&mut self, s: &StructDef<'_>) {
        let vis = match s.visibility {
            Visibility::Pub => "pub ",
            Visibility::Private => "",
        };
        self.line(&format!("{vis}struct {}", self.name(s.name)));
        self.indent += 1;
        for field in s.fields {
            self.line(&format!(
                "{}: {}",
                self.name(field.name),
                self.type_str(field.ty)
            ));
        }
        self.indent -= 1;
        self.line("end");
    }

    fn enum_def(&mut self, e: &EnumDef<'_>) {
        let vis = match e.visibility {
            Visibility::Pub => "pub ",
            Visibility::Private => "",
        };
        self.line(&format!("{vis}enum {}", self.name(e.name)));
        self.indent += 1;
        for v in e.variants {
            if let Some(ty) = v.payload {
                self.line(&format!("{}({})", self.name(v.name), self.type_str(ty)));
            } else {
                let name = self.name(v.name).to_string();
                self.line(&name);
            }
        }
        self.indent -= 1;
        self.line("end");
    }

    fn type_alias(&mut self, t: &TypeAlias<'_>) {
        self.line(&format!(
            "type {} = {}",
            self.name(t.name),
            self.type_str(t.ty)
        ));
    }

    fn stmt(&mut self, stmt: &Stmt<'_>) {
        match stmt {
            Stmt::Error(_) => {}
            Stmt::Let(l) => {
                let mut s = String::from(if l.mutable { "let mut " } else { "let " });
                s.push_str(self.name(l.name));
                if let Some(ty) = l.ty {
                    let _ = write!(s, ": {}", self.type_str(ty));
                }
                let _ = write!(s, " = {}", self.expr_str(l.init));
                self.line(&s);
            }
            Stmt::Assign(a) => {
                self.line(&format!(
                    "{} {} {}",
                    self.expr_str(a.target),
                    a.op.as_str(),
                    self.expr_str(a.value)
                ));
            }
            Stmt::Return(r) => {
                if let Some(v) = r.value {
                    self.line(&format!("return {}", self.expr_str(v)));
                } else {
                    self.line("return");
                }
            }
            Stmt::Break(_) => self.line("break"),
            Stmt::Continue(_) => self.line("continue"),
            Stmt::Expr(e) => self.line(&self.expr_str(e.expr)),
            Stmt::Loop(l) => {
                self.line("loop");
                self.indent += 1;
                for s in l.body {
                    self.stmt(s);
                }
                self.indent -= 1;
                self.line("end");
            }
            Stmt::Region(r) => {
                self.line("region");
                self.indent += 1;
                for s in r.body {
                    self.stmt(s);
                }
                self.indent -= 1;
                self.line("end");
            }
            Stmt::For(f) => {
                self.line(&format!(
                    "for {} in {}",
                    self.name(f.binder),
                    self.expr_str(f.iter)
                ));
                self.indent += 1;
                for s in f.body {
                    self.stmt(s);
                }
                self.indent -= 1;
                self.line("end");
            }
            Stmt::If(i) => {
                for (ai, arm) in i.arms.iter().enumerate() {
                    let kw = if ai == 0 { "if" } else { "elif" };
                    self.line(&format!("{kw} {}", self.expr_str(arm.cond)));
                    self.indent += 1;
                    for s in arm.body {
                        self.stmt(s);
                    }
                    self.indent -= 1;
                }
                if let Some(body) = i.else_body {
                    self.line("else");
                    self.indent += 1;
                    for s in body {
                        self.stmt(s);
                    }
                    self.indent -= 1;
                }
                self.line("end");
            }
            Stmt::Match(m) => {
                self.line(&format!("match {}", self.expr_str(m.scrutinee)));
                for arm in m.arms {
                    let pat = match &arm.pattern {
                        MatchPat::Wildcard(_) => "_".into(),
                        MatchPat::Literal(e) => self.expr_str(e),
                    };
                    self.line(&pat);
                    self.indent += 1;
                    for s in arm.body {
                        self.stmt(s);
                    }
                    self.indent -= 1;
                }
                if let Some(body) = m.else_body {
                    self.line("else");
                    self.indent += 1;
                    for s in body {
                        self.stmt(s);
                    }
                    self.indent -= 1;
                }
                self.line("end");
            }
        }
    }

    fn expr_str(&self, expr: &Expr<'_>) -> String {
        match expr {
            Expr::Error(_) => "?".into(),
            Expr::Literal(l) => match l.kind {
                LiteralKind::True => "true".into(),
                LiteralKind::False => "false".into(),
                LiteralKind::Nil => "nil".into(),
                LiteralKind::Int | LiteralKind::Float | LiteralKind::Str => {
                    self.span_text(l.span).to_string()
                }
            },
            Expr::Path(p) => self.path_str(p),
            Expr::Unary(u) => {
                let op = match u.op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Not => "not ",
                };
                format!("{op}{}", self.expr_str(u.operand))
            }
            Expr::Binary(b) => format!(
                "{} {} {}",
                self.expr_str(b.lhs),
                b.op.as_str(),
                self.expr_str(b.rhs)
            ),
            Expr::Cast(c) => format!("{} as {}", self.expr_str(c.expr), self.type_str(c.ty)),
            Expr::Call(c) => {
                let mut s = self.expr_str(c.callee);
                s.push('(');
                for (i, a) in c.args.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&self.expr_str(a));
                }
                s.push(')');
                s
            }
            Expr::MethodCall(m) => {
                let mut s = format!("{}.{}(", self.expr_str(m.receiver), self.name(m.method));
                for (i, a) in m.args.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&self.expr_str(a));
                }
                s.push(')');
                s
            }
            Expr::Index(i) => format!("{}[{}]", self.expr_str(i.base), self.expr_str(i.index)),
            Expr::Field(f) => format!("{}.{}", self.expr_str(f.base), self.name(f.field)),
            Expr::StructLit(s) => {
                let mut out = format!("{} {{", self.path_str(s.path));
                for (i, f) in s.fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push(' ');
                    out.push_str(self.name(f.name));
                    out.push_str(": ");
                    out.push_str(&self.expr_str(f.value));
                }
                if !s.fields.is_empty() {
                    out.push(' ');
                }
                out.push('}');
                out
            }
            Expr::Array(a) => {
                let mut s = String::from("[");
                for (i, e) in a.elems.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&self.expr_str(e));
                }
                s.push(']');
                s
            }
            Expr::Spawn(s) => format!("spawn {}", self.expr_str(s.callee)),
        }
    }

    fn type_str(&self, ty: &Type<'_>) -> String {
        match ty {
            Type::Path(p) => self.path_str(p),
            Type::App { path, args, .. } => {
                let base = self.path_str(path);
                let inner = args
                    .iter()
                    .map(|a| self.type_str(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{base}[{inner}]")
            }
            Type::Slice(inner, _) => format!("[{}]", self.type_str(inner)),
            Type::Error(_) => "?".into(),
        }
    }

    fn path_str(&self, path: &Path<'_>) -> String {
        path.segments
            .iter()
            .map(|s| self.interner.resolve(s.name))
            .collect::<Vec<_>>()
            .join("::")
    }
}

// Ensure BinaryOp has as_str — check node.rs
#[allow(dead_code)]
fn _bin(op: BinaryOp) -> &'static str {
    op.as_str()
}

#[allow(dead_code)]
fn _assign(op: AssignOp) -> &'static str {
    op.as_str()
}
