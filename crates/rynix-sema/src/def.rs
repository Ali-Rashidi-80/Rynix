use rynix_ast::NodeId;
use rynix_span::{Span, Symbol};

/// Dense handle for a definition.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct DefId(u32);

impl DefId {
    #[inline]
    pub fn index(self) -> u32 {
        self.0
    }

    pub(crate) fn from_index(index: u32) -> Self {
        DefId(index)
    }
}

/// What a [`DefId`] names.
#[derive(Clone, Debug)]
pub enum DefKind {
    Fn {
        node: NodeId,
        name: Symbol,
        span: Span,
    },
    Struct {
        node: NodeId,
        name: Symbol,
        span: Span,
    },
    Enum {
        node: NodeId,
        name: Symbol,
        span: Span,
    },
    Variant {
        parent: DefId,
        name: Symbol,
        span: Span,
    },
    TypeAlias {
        node: NodeId,
        name: Symbol,
        span: Span,
    },
    /// Builtin type name (`i64`, `f64`, `bool`, `str`).
    BuiltinType { name: Symbol },
    Param {
        name: Symbol,
        span: Span,
        mutable: bool,
    },
    Local {
        name: Symbol,
        span: Span,
        mutable: bool,
    },
    /// `import a::b` binds `b` as an opaque module value.
    Import { name: Symbol, span: Span },
}

impl DefKind {
    pub fn name(&self) -> Symbol {
        match self {
            DefKind::Fn { name, .. }
            | DefKind::Struct { name, .. }
            | DefKind::Enum { name, .. }
            | DefKind::Variant { name, .. }
            | DefKind::TypeAlias { name, .. }
            | DefKind::BuiltinType { name }
            | DefKind::Param { name, .. }
            | DefKind::Local { name, .. }
            | DefKind::Import { name, .. } => *name,
        }
    }

    pub fn span(&self) -> Option<Span> {
        match self {
            DefKind::Fn { span, .. }
            | DefKind::Struct { span, .. }
            | DefKind::Enum { span, .. }
            | DefKind::Variant { span, .. }
            | DefKind::TypeAlias { span, .. }
            | DefKind::Param { span, .. }
            | DefKind::Local { span, .. }
            | DefKind::Import { span, .. } => Some(*span),
            DefKind::BuiltinType { .. } => None,
        }
    }

    pub fn is_type(&self) -> bool {
        matches!(
            self,
            DefKind::Struct { .. }
                | DefKind::Enum { .. }
                | DefKind::TypeAlias { .. }
                | DefKind::BuiltinType { .. }
        )
    }

    pub fn is_value(&self) -> bool {
        matches!(
            self,
            DefKind::Fn { .. }
                | DefKind::Variant { .. }
                | DefKind::Param { .. }
                | DefKind::Local { .. }
                | DefKind::Import { .. }
        )
    }

    pub fn is_mutable(&self) -> bool {
        match self {
            DefKind::Param { mutable, .. } | DefKind::Local { mutable, .. } => *mutable,
            _ => false,
        }
    }
}
