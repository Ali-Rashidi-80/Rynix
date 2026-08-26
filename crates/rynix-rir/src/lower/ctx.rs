fn lower_function(
    f: &rynix_ast::FnDef<'_>,
    analysis: &Analysis,
    interner: &mut Interner,
    fn_map: &FxHashMap<Symbol, FuncId>,
    fn_bodies: &FxHashMap<Symbol, &FnDef<'_>>,
    src: &str,
    base: u32,
) -> crate::ir::Function {
    let ret_ty = analysis
        .scopes
        .lookup(analysis.module_scope, f.name.name)
        .and_then(|d| analysis.def_types.get(&d).copied())
        .map(|fty| match analysis.types.kind(fty) {
            TypeKind::Fn { ret, .. } => map_ty(analysis, *ret),
            _ => IrTy::Unit,
        })
        .unwrap_or(IrTy::Unit);

    let mut b = FunctionBuilder::new(f.name.name, ret_ty);
    let mut locals: FxHashMap<Symbol, Local> = FxHashMap::default();
    let mut mut_slots: FxHashSet<ValueId> = FxHashSet::default();
    let mut mut_nonneg_syms: FxHashSet<Symbol> = FxHashSet::default();
    let mut mut_positive_syms: FxHashSet<Symbol> = FxHashSet::default();
    // Exclusive upper bound: symbol value ∈ [0, bound).
    let mut mut_excl_bound: FxHashMap<Symbol, i64> = FxHashMap::default();
    let mut mut_binding_sites: FxHashMap<Symbol, AllocSite> = FxHashMap::default();
    let mut loops: Vec<LoopFrame> = Vec::new();

    // Params: direct SSA bindings (copied into `mut` locals at use sites).
    for param in f.params {
        let ty = map_ty(
            analysis,
            param_type(analysis, f, param),
        );
        let incoming = b.add_param(ty);
        locals.insert(param.name.name, Local::Ssa(incoming));
    }

    let mut loop_carried: Vec<Vec<LoopCarried>> = Vec::new();
    let mut loop_carried_linear: Vec<bool> = Vec::new();
    let mut cx = LowerCtx {
        b: &mut b,
        analysis,
        interner,
        fn_map,
        fn_bodies,
        locals: &mut locals,
        mut_slots: &mut mut_slots,
        mut_nonneg_syms: &mut mut_nonneg_syms,
        mut_positive_syms: &mut mut_positive_syms,
        mut_excl_bound: &mut mut_excl_bound,
        value_excl_bound_map: FxHashMap::default(),
        mut_binding_sites: &mut mut_binding_sites,
        loops: &mut loops,
        loop_carried: &mut loop_carried,
        loop_carried_linear: &mut loop_carried_linear,
        src,
        base,
        inlining: false,
        inline_ret: None,
        inline_merge: None,
    };
    if is_euclidean_gcd_fn(f) && f.params.len() == 2 {
        let Local::Ssa(a) = cx.locals[&f.params[0].name.name] else {
            unreachable!("gcd params are SSA");
        };
        let Local::Ssa(bv) = cx.locals[&f.params[1].name.name] else {
            unreachable!("gcd params are SSA");
        };
        let r = cx.lower_binary_gcd(a, bv);
        cx.b.ret(Some(r));
        return b.finish();
    }
    for stmt in f.body {
        cx.stmt(stmt);
    }

    // Implicit return if block not terminated.
    if !cx.is_terminated() {
        if ret_ty == IrTy::Unit {
            cx.b.ret(None);
        } else {
            // Missing return — emit unreachable for verifier friendliness.
            let _ = cx.b.push(Inst::Unreachable);
        }
    }

    b.finish()
}

fn param_type(
    analysis: &Analysis,
    f: &rynix_ast::FnDef<'_>,
    param: &rynix_ast::Param<'_>,
) -> TypeId {
    if let Some(def) = analysis.scopes.lookup(analysis.module_scope, f.name.name)
        && let Some(&fty) = analysis.def_types.get(&def)
        && let TypeKind::Fn { params, .. } = analysis.types.kind(fty)
    {
        // Match by position.
        if let Some(idx) = f.params.iter().position(|p| p.name.name == param.name.name)
            && let Some(&ty) = params.get(idx)
        {
            return ty;
        }
    }
    analysis.types.ty_error
}

struct LowerCtx<'a, 'b> {
    b: &'a mut FunctionBuilder,
    analysis: &'b Analysis,
    interner: &'b mut Interner,
    fn_map: &'b FxHashMap<Symbol, FuncId>,
    fn_bodies: &'b FxHashMap<Symbol, &'b FnDef<'b>>,
    locals: &'a mut FxHashMap<Symbol, Local>,
    /// Stack slots declared with `mut let` (eligible for loop SSA promotion).
    mut_slots: &'a mut FxHashSet<ValueId>,
    /// Symbols known >= 0 (init 0, only += non-negative).
    mut_nonneg_syms: &'a mut FxHashSet<Symbol>,
    /// Symbols known >= 1 (init >= 1, never assigned 0).
    mut_positive_syms: &'a mut FxHashSet<Symbol>,
    /// Exclusive upper bound: symbol's value ∈ `[0, bound)`.
    mut_excl_bound: &'a mut FxHashMap<Symbol, i64>,
    /// ValueIds known ∈ `[0, bound)` (e.g. after small-factor rem peephole).
    value_excl_bound_map: FxHashMap<ValueId, i64>,
    /// Reserved escape-analysis sites for `let mut` bindings (SSA until materialized).
    mut_binding_sites: &'a mut FxHashMap<Symbol, AllocSite>,
    loops: &'a mut Vec<LoopFrame>,
    /// Nested loop-carried SSA frames (innermost last).
    loop_carried: &'a mut Vec<Vec<LoopCarried>>,
    /// Parallel to `loop_carried`: true = phi-only backedge, false = alloca roundtrip.
    loop_carried_linear: &'a mut Vec<bool>,
    src: &'b str,
    base: u32,
    /// When set, `return` inside an inlined callee records here instead of terminating the caller.
    inlining: bool,
    inline_ret: Option<ValueId>,
    /// Join block for inlined callee early `return`.
    inline_merge: Option<BlockId>,
}

fn parse_int_lit(text: &str) -> Option<i64> {
    let t = text.replace('_', "");
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

fn strip_string_lit(text: &str) -> String {
    let t = text.strip_prefix('"').unwrap_or(text);
    let t = t.strip_suffix('"').unwrap_or(t);
    // Minimal unescape for common sequences.
    t.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}

