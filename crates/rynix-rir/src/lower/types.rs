#[derive(Clone, Copy)]
enum Local {
    /// Mutable SSA — no alloca until a non-linear loop materializes one.
    MutSsa(ValueId),
    /// Mutable stack slot (alloca) — addressable / non-linear loops.
    Slot(ValueId),
    /// Immutable SSA value (Braun-style direct binding).
    Ssa(ValueId),
}

/// Loop-carried mutable local promoted to block-param SSA inside a loop.
#[derive(Copy, Clone)]
struct LoopCarried {
    sym: Symbol,
    /// Allocated when non-linear loops need stack slots; absent for pure-SSA linear loops.
    slot: Option<ValueId>,
    /// Phi parameter SSA name (stable across iterations).
    param: ValueId,
    /// Latest value for back-edge when using pure SSA.
    current: ValueId,
    /// Initialized to 0 and only incremented with non-negative deltas in this loop.
    nonneg: bool,
    /// Initialized to >= 1 and never decremented to 0 in this loop.
    strictly_positive: bool,
    /// Exclusive upper bound if known (`current ∈ [0, excl_bound)`).
    excl_bound: Option<i64>,
}

#[derive(Clone)]
struct LoopFrame {
    header: BlockId,
    exit: BlockId,
    /// Pure phi SSA back-edge (no per-iteration alloca traffic).
    linear_carried: bool,
    /// `RemZero` guard syms merged at `exit` (phi over normal vs cleared value).
    guard_clears: Vec<(Symbol, i64)>,
}
/// Lower an entire module. `analysis` must be from the same AST.

fn map_ty(analysis: &Analysis, ty: TypeId) -> IrTy {
    match analysis.types.kind(ty) {
        TypeKind::Error | TypeKind::Never | TypeKind::Unit | TypeKind::Nil | TypeKind::Module => {
            IrTy::Unit
        }
        TypeKind::Bool => IrTy::Bool,
        TypeKind::Int => IrTy::I64,
        TypeKind::Float => IrTy::F64,
        TypeKind::Str => IrTy::Str,
        TypeKind::Enum(_) => IrTy::I64,
        TypeKind::Ptr
        | TypeKind::Vec
        | TypeKind::VecStr
        | TypeKind::Map
        | TypeKind::MapStrI64
        | TypeKind::MapStrStr
        | TypeKind::Slice(_)
        | TypeKind::Struct(_)
        | TypeKind::Fn { .. } => IrTy::Ptr,
    }
}

const INLINE_STMT_LIMIT: usize = 48;

#[derive(Clone, Copy, Debug)]
enum LoopExitGuard {
    /// `if counter >= bound break` (continue while `counter < bound`).
    CountedGe { counter: Symbol, bound: Symbol },
    /// `if counter >= lit break` with a compile-time literal bound.
    CountedGeLit { counter: Symbol, bound: i64 },
    /// `if counter > bound break` (continue while `counter <= bound`).
    CountedGt { counter: Symbol, bound: Symbol },
    /// `if counter == 0 break` (popcount-style).
    Zero { counter: Symbol },
    /// `if counter * counter > bound break` (prime inner loop).
    SquareGt { counter: Symbol, bound: Symbol },
    /// `if dividend % divisor == 0 { clear = 0; break }`.
    RemZero {
        dividend: Symbol,
        divisor: Symbol,
        clear_sym: Symbol,
        clear_val: i64,
    },
}

fn expr_path(e: &Expr<'_>) -> Option<Symbol> {
    match e {
        Expr::Path(p) if p.segments.len() == 1 => Some(p.segments[0].name),
        _ => None,
    }
}

fn lit_is_zero(e: &Expr<'_>) -> bool {
    matches!(e, Expr::Literal(l) if l.kind == LiteralKind::Int && l.int_value == Some(0))
}

fn expr_lit_i64(e: &Expr<'_>) -> Option<i64> {
    match e {
        Expr::Literal(l) if l.kind == LiteralKind::Int => l.int_value,
        _ => None,
    }
}

/// Fully unroll `if counter >= bound break` loops when `bound` is a small literal.
const SMALL_LOOP_UNROLL_TRIP_MAX: i64 = 8;

fn lit_is_one(e: &Expr<'_>) -> bool {
    matches!(e, Expr::Literal(l) if l.kind == LiteralKind::Int && l.int_value == Some(1))
}

fn lit_is_two(e: &Expr<'_>) -> bool {
    matches!(e, Expr::Literal(l) if l.kind == LiteralKind::Int && l.int_value == Some(2))
}

fn paths_equal(a: &Expr<'_>, b: &Expr<'_>) -> bool {
    match (a, b) {
        (Expr::Path(pa), Expr::Path(pb)) => {
            pa.segments.last().map(|s| s.name) == pb.segments.last().map(|s| s.name)
        }
        _ => false,
    }
}

