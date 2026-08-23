//! Name resolution and type checking for Rynix.
//!
//! Pipeline: collect item defs → resolve names in two passes → type-check
//! function bodies. Function signatures are always explicit; only locals are
//! inferred (predictability for AI and interprocedural analysis).

mod check;
mod def;
mod dump;
mod effects;
mod errors;
mod scope;
mod ty;

pub use check::{Analysis, analyze, analyze_with_source};
pub use def::{DefId, DefKind};
pub use dump::dump_types;
pub use effects::{EffectSet, builtin_effects, check_module_effects};
pub use scope::{ScopeId, ScopeKind};
pub use ty::{TypeCtx, TypeId, TypeKind};
