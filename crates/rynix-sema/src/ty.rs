use rustc_hash::FxHashMap;
use rynix_span::Symbol;

use crate::def::DefId;

/// Dense handle for a hash-consed type.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct TypeId(u32);

impl TypeId {
    #[inline]
    pub fn index(self) -> u32 {
        self.0
    }
}

/// Canonical type kinds. Nominal types point at [`DefId`]s.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum TypeKind {
    /// Poison type for error recovery — unifies with everything.
    Error,
    Unit,
    Never,
    Bool,
    /// Default integer type (`i64`).
    Int,
    /// Default float type (`f64`).
    Float,
    Str,
    Nil,
    /// Opaque runtime pointer (generic heap/region handle).
    Ptr,
    /// Region Vec[i64] handle (ADR-0006).
    Vec,
    /// Region Map[i64, i64] handle (ADR-0006).
    Map,
    Slice(TypeId),
    Fn {
        params: Vec<TypeId>,
        ret: TypeId,
    },
    Struct(DefId),
    Enum(DefId),
    /// Opaque imported module (`import std::io` → `io`).
    Module,
}

/// Hash-consing type context.
#[derive(Debug)]
pub struct TypeCtx {
    kinds: Vec<TypeKind>,
    intern: FxHashMap<TypeKind, TypeId>,
    /// Cached builtins.
    pub ty_error: TypeId,
    pub ty_unit: TypeId,
    pub ty_never: TypeId,
    pub ty_bool: TypeId,
    pub ty_int: TypeId,
    pub ty_float: TypeId,
    pub ty_str: TypeId,
    pub ty_nil: TypeId,
    pub ty_ptr: TypeId,
    pub ty_vec: TypeId,
    pub ty_map: TypeId,
    pub ty_module: TypeId,
}

impl Default for TypeCtx {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeCtx {
    pub fn new() -> Self {
        let mut ctx = TypeCtx {
            kinds: Vec::new(),
            intern: FxHashMap::default(),
            ty_error: TypeId(0),
            ty_unit: TypeId(0),
            ty_never: TypeId(0),
            ty_bool: TypeId(0),
            ty_int: TypeId(0),
            ty_float: TypeId(0),
            ty_str: TypeId(0),
            ty_nil: TypeId(0),
            ty_ptr: TypeId(0),
            ty_vec: TypeId(0),
            ty_map: TypeId(0),
            ty_module: TypeId(0),
        };
        ctx.ty_error = ctx.intern(TypeKind::Error);
        ctx.ty_unit = ctx.intern(TypeKind::Unit);
        ctx.ty_never = ctx.intern(TypeKind::Never);
        ctx.ty_bool = ctx.intern(TypeKind::Bool);
        ctx.ty_int = ctx.intern(TypeKind::Int);
        ctx.ty_float = ctx.intern(TypeKind::Float);
        ctx.ty_str = ctx.intern(TypeKind::Str);
        ctx.ty_nil = ctx.intern(TypeKind::Nil);
        ctx.ty_ptr = ctx.intern(TypeKind::Ptr);
        ctx.ty_vec = ctx.intern(TypeKind::Vec);
        ctx.ty_map = ctx.intern(TypeKind::Map);
        ctx.ty_module = ctx.intern(TypeKind::Module);
        ctx
    }

    pub fn intern(&mut self, kind: TypeKind) -> TypeId {
        if let Some(&id) = self.intern.get(&kind) {
            return id;
        }
        let id = TypeId(self.kinds.len() as u32);
        self.kinds.push(kind.clone());
        self.intern.insert(kind, id);
        id
    }

    #[inline]
    pub fn kind(&self, id: TypeId) -> &TypeKind {
        &self.kinds[id.0 as usize]
    }

    pub fn slice(&mut self, elem: TypeId) -> TypeId {
        self.intern(TypeKind::Slice(elem))
    }

    pub fn fn_type(&mut self, params: Vec<TypeId>, ret: TypeId) -> TypeId {
        self.intern(TypeKind::Fn { params, ret })
    }

    pub fn struct_type(&mut self, def: DefId) -> TypeId {
        self.intern(TypeKind::Struct(def))
    }

    pub fn enum_type(&mut self, def: DefId) -> TypeId {
        self.intern(TypeKind::Enum(def))
    }

    /// Structural equality that treats [`TypeKind::Error`] as a wildcard.
    pub fn compatible(&self, a: TypeId, b: TypeId) -> bool {
        if a == b {
            return true;
        }
        match (self.kind(a), self.kind(b)) {
            (TypeKind::Error | TypeKind::Never, _) | (_, TypeKind::Error | TypeKind::Never) => true,
            (TypeKind::Slice(x), TypeKind::Slice(y)) => self.compatible(*x, *y),
            (
                TypeKind::Fn {
                    params: p1,
                    ret: r1,
                },
                TypeKind::Fn {
                    params: p2,
                    ret: r2,
                },
            ) => {
                p1.len() == p2.len()
                    && p1.iter().zip(p2).all(|(x, y)| self.compatible(*x, *y))
                    && self.compatible(*r1, *r2)
            }
            _ => false,
        }
    }

    /// Display name for diagnostics and dumps.
    pub fn display(
        &self,
        id: TypeId,
        resolve_name: &dyn Fn(DefId) -> Symbol,
        interner: &rynix_span::Interner,
    ) -> String {
        match self.kind(id) {
            TypeKind::Error => "<error>".into(),
            TypeKind::Unit => "()".into(),
            TypeKind::Never => "!".into(),
            TypeKind::Bool => "bool".into(),
            TypeKind::Int => "i64".into(),
            TypeKind::Float => "f64".into(),
            TypeKind::Str => "str".into(),
            TypeKind::Nil => "nil".into(),
            TypeKind::Ptr => "ptr".into(),
            TypeKind::Vec => "Vec[i64]".into(),
            TypeKind::Map => "Map[i64, i64]".into(),
            TypeKind::Module => "<module>".into(),
            TypeKind::Slice(e) => format!("[{}]", self.display(*e, resolve_name, interner)),
            TypeKind::Fn { params, ret } => {
                let ps: Vec<_> = params
                    .iter()
                    .map(|p| self.display(*p, resolve_name, interner))
                    .collect();
                format!(
                    "fn({}) -> {}",
                    ps.join(", "),
                    self.display(*ret, resolve_name, interner)
                )
            }
            TypeKind::Struct(d) | TypeKind::Enum(d) => {
                interner.resolve(resolve_name(*d)).to_string()
            }
        }
    }
}
