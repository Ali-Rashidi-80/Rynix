//! Lower typed AST + sema analysis into RIR.

#![allow(clippy::too_many_lines)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]

use rynix_ast::{
    AssignOp, BinaryOp, Expr, FieldExpr, FnDef, Item, LiteralKind, Module as AstModule, Stmt,
    UnaryOp,
};
use rynix_sema::{Analysis, TypeId, TypeKind};
use rynix_span::{Interner, Symbol};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

use crate::builder::FunctionBuilder;
use crate::ir::{AllocSite, BlockId, CmpOp, FuncId, Inst, IrTy, Module, ValueId};

include!("types.rs");
include!("host_math.rs");
include!("recognizers.rs");
include!("loop_carried.rs");
include!("ctx.rs");
include!("value.rs");
include!("loop_kernels.rs");
include!("stmt.rs");
include!("expr.rs");

/// Lower an entire module. `analysis` must be from the same AST.
/// `src` is the original source (for recovering literal values from spans).
pub fn lower_module(
    ast: &AstModule<'_>,
    analysis: &Analysis,
    interner: &mut Interner,
    src: &str,
    base: u32,
) -> Module {
    let mut module = Module::new();

    // First pass: declare all functions so calls can resolve.
    let mut fn_map: FxHashMap<Symbol, FuncId> = FxHashMap::default();
    let mut fn_bodies: FxHashMap<Symbol, &FnDef<'_>> = FxHashMap::default();
    for item in ast.items {
        if let Item::Fn(f) = item {
            let id = FuncId(module.funcs.len() as u32);
            // Placeholder; replaced below.
            module
                .funcs
                .push(FunctionBuilder::new(f.name.name, IrTy::Unit).finish());
            module.func_names.push(f.name.name);
            fn_map.insert(f.name.name, id);
            fn_bodies.insert(f.name.name, f);
        }
    }

    for item in ast.items {
        if let Item::Fn(f) = item {
            let fid = fn_map[&f.name.name];
            let func = lower_function(f, analysis, interner, &fn_map, &fn_bodies, src, base);
            module.funcs[fid.0 as usize] = func;
        }
    }

    module
}
