//! Compact s-expression dump of a module — the snapshot format for Phase 2.

use std::fmt::Write as _;

use rynix_span::Interner;

use crate::node::{
    EnumDef, Expr, FnDef, Ident, Item, LiteralKind, Module, Path, Stmt, StructDef, Type, TypeAlias,
    Visibility,
};

/// Renders `module` as a multi-line s-expression. Symbols are resolved through
/// `interner`; the output is deterministic and suitable for `insta` snapshots.
pub fn dump_module(module: &Module<'_>, interner: &Interner) -> String {
    let mut out = String::new();
    let mut d = Dumper {
        out: &mut out,
        interner,
        indent: 0,
    };
    d.module(module);
    out
}

struct Dumper<'a> {
    out: &'a mut String,
    interner: &'a Interner,
    indent: usize,
}

impl Dumper<'_> {
    fn line(&mut self, text: &str) {
        for _ in 0..self.indent {
            self.out.push_str("  ");
        }
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn open(&mut self, head: &str) {
        self.line(&format!("({head}"));
        self.indent += 1;
    }

    fn close(&mut self) {
        self.indent -= 1;
        self.line(")");
    }

    fn atom(&mut self, text: &str) {
        self.line(text);
    }

    fn name(&self, id: Ident) -> &str {
        self.interner.resolve(id.name)
    }

    fn module(&mut self, module: &Module<'_>) {
        self.open("module");
        for item in module.items {
            self.item(item);
        }
        self.close();
    }

    fn item(&mut self, item: &Item<'_>) {
        match item {
            Item::Fn(f) => self.fn_def(f),
            Item::Struct(s) => self.struct_def(s),
            Item::Enum(e) => self.enum_def(e),
            Item::TypeAlias(t) => self.type_alias(t),
            Item::Import(i) => {
                self.atom(&format!("(import {})", self.path_str(i.path)));
            }
            Item::Error(_) => self.atom("(error)"),
        }
    }

    fn fn_def(&mut self, f: &FnDef<'_>) {
        let vis = match f.visibility {
            Visibility::Pub => "pub ",
            Visibility::Private => "",
        };
        let mut head = format!("{vis}fn {}", self.name(f.name));
        if let Some(ret) = f.ret {
            let _ = write!(head, " -> {}", self.type_str(ret));
        }
        self.open(&head);
        if !f.params.is_empty() {
            self.open("params");
            for p in f.params {
                self.atom(&format!("({}: {})", self.name(p.name), self.type_str(p.ty)));
            }
            self.close();
        }
        self.open("body");
        for s in f.body {
            self.stmt(s);
        }
        self.close();
        self.close();
    }

    fn struct_def(&mut self, s: &StructDef<'_>) {
        let vis = match s.visibility {
            Visibility::Pub => "pub ",
            Visibility::Private => "",
        };
        self.open(&format!("{vis}struct {}", self.name(s.name)));
        for f in s.fields {
            self.atom(&format!("({}: {})", self.name(f.name), self.type_str(f.ty)));
        }
        self.close();
    }

    fn enum_def(&mut self, e: &EnumDef<'_>) {
        let vis = match e.visibility {
            Visibility::Pub => "pub ",
            Visibility::Private => "",
        };
        self.open(&format!("{vis}enum {}", self.name(e.name)));
        for v in e.variants {
            match v.payload {
                Some(ty) => self.atom(&format!("({} {})", self.name(v.name), self.type_str(ty))),
                None => self.atom(&format!("({})", self.name(v.name))),
            }
        }
        self.close();
    }

    fn type_alias(&mut self, t: &TypeAlias<'_>) {
        self.atom(&format!(
            "(type {} = {})",
            self.name(t.name),
            self.type_str(t.ty)
        ));
    }

    fn stmt(&mut self, stmt: &Stmt<'_>) {
        match stmt {
            Stmt::Let(l) => {
                let mut head = format!(
                    "let{} {}",
                    if l.mutable { " mut" } else { "" },
                    self.name(l.name)
                );
                if let Some(ty) = l.ty {
                    let _ = write!(head, ": {}", self.type_str(ty));
                }
                self.open(&head);
                self.expr(l.init);
                self.close();
            }
            Stmt::Assign(a) => {
                self.open(&format!("assign {}", a.op.as_str()));
                self.expr(a.target);
                self.expr(a.value);
                self.close();
            }
            Stmt::Return(r) => match r.value {
                Some(v) => {
                    self.open("return");
                    self.expr(v);
                    self.close();
                }
                None => self.atom("(return)"),
            },
            Stmt::Break(_) => self.atom("(break)"),
            Stmt::Continue(_) => self.atom("(continue)"),
            Stmt::Loop(l) => {
                self.open("loop");
                self.stmts(l.body);
                self.close();
            }
            Stmt::Region(r) => {
                self.open("region");
                self.stmts(r.body);
                self.close();
            }
            Stmt::For(f) => {
                self.open(&format!("for {}", self.name(f.binder)));
                self.expr(f.iter);
                self.open("body");
                self.stmts(f.body);
                self.close();
                self.close();
            }
            Stmt::If(i) => self.stmt_if(i),
            Stmt::Match(m) => self.stmt_match(m),
            Stmt::Expr(e) => {
                self.open("expr");
                self.expr(e.expr);
                self.close();
            }
            Stmt::Error(_) => self.atom("(error)"),
        }
    }

    fn stmts(&mut self, body: &[Stmt<'_>]) {
        for s in body {
            self.stmt(s);
        }
    }

    fn stmt_if(&mut self, i: &crate::IfStmt<'_>) {
        self.open("if");
        for (idx, arm) in i.arms.iter().enumerate() {
            self.open(if idx == 0 { "arm" } else { "elif" });
            self.expr(arm.cond);
            self.open("body");
            self.stmts(arm.body);
            self.close();
            self.close();
        }
        if let Some(body) = i.else_body {
            self.open("else");
            self.stmts(body);
            self.close();
        }
        self.close();
    }

    fn stmt_match(&mut self, m: &crate::MatchStmt<'_>) {
        self.open("match");
        self.expr(m.scrutinee);
        for arm in m.arms {
            self.open("arm");
            match &arm.pattern {
                crate::MatchPat::Wildcard(_) => self.atom("_"),
                crate::MatchPat::Literal(e) => self.expr(e),
                crate::MatchPat::Ctor { path, binder } => {
                    self.atom(&format!(
                        "(ctor {} {})",
                        self.path_str(path),
                        self.interner.resolve(binder.name)
                    ));
                }
            }
            self.open("body");
            self.stmts(arm.body);
            self.close();
            self.close();
        }
        if let Some(body) = m.else_body {
            self.open("else");
            self.stmts(body);
            self.close();
        }
        self.close();
    }

    fn expr(&mut self, expr: &Expr<'_>) {
        match expr {
            Expr::Literal(l) => {
                let tag = match l.kind {
                    LiteralKind::Int => "int",
                    LiteralKind::Float => "float",
                    LiteralKind::Str => "str",
                    LiteralKind::True => "true",
                    LiteralKind::False => "false",
                    LiteralKind::Nil => "nil",
                };
                self.atom(&format!("({tag})"));
            }
            Expr::Path(p) => self.atom(&format!("(path {})", self.path_str(p))),
            Expr::Unary(u) => {
                self.open(&format!("unary {}", u.op.as_str()));
                self.expr(u.operand);
                self.close();
            }
            Expr::Binary(b) => {
                self.open(&format!("binary {}", b.op.as_str()));
                self.expr(b.lhs);
                self.expr(b.rhs);
                self.close();
            }
            Expr::Cast(c) => {
                self.open(&format!("as {}", self.type_str(c.ty)));
                self.expr(c.expr);
                self.close();
            }
            Expr::Call(c) => {
                self.open("call");
                self.expr(c.callee);
                for a in c.args {
                    self.expr(a);
                }
                self.close();
            }
            Expr::MethodCall(m) => {
                self.open(&format!("method {}", self.name(m.method)));
                self.expr(m.receiver);
                for a in m.args {
                    self.expr(a);
                }
                self.close();
            }
            Expr::Index(i) => {
                self.open("index");
                self.expr(i.base);
                self.expr(i.index);
                self.close();
            }
            Expr::Field(f) => {
                self.open(&format!("field {}", self.name(f.field)));
                self.expr(f.base);
                self.close();
            }
            Expr::StructLit(s) => {
                self.open(&format!("struct_lit {}", self.path_str(s.path)));
                for f in s.fields {
                    self.open(&format!("field_init {}", self.name(f.name)));
                    self.expr(f.value);
                    self.close();
                }
                self.close();
            }
            Expr::Array(a) => {
                self.open("array");
                for e in a.elems {
                    self.expr(e);
                }
                self.close();
            }
            Expr::Spawn(s) => {
                self.open("spawn");
                self.expr(s.callee);
                self.close();
            }
            Expr::Error(_) => self.atom("(error)"),
        }
    }

    fn path_str(&self, path: &Path<'_>) -> String {
        path.segments
            .iter()
            .map(|s| self.interner.resolve(s.name))
            .collect::<Vec<_>>()
            .join("::")
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
            Type::Error(_) => "<error>".to_string(),
        }
    }
}
