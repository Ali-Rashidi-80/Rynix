//! Core RIR data structures (`SoA`, block arguments, typed values).

use rynix_span::Symbol;

/// Dense function handle inside a [`Module`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct FuncId(pub u32);

/// Dense basic-block handle inside a [`Function`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct BlockId(pub u32);

/// Dense instruction handle inside a [`Function`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct InstId(pub u32);

/// Dense SSA value handle inside a [`Function`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct ValueId(pub u32);

/// Allocation site id — the unit of escape analysis (Phase 6).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct AllocSite(pub u32);

/// RIR types (subset of sema types, flattened).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum IrTy {
    Unit,
    Bool,
    I64,
    F64,
    /// Thin pointer to a UTF-8 string constant or runtime string.
    Str,
    /// Pointer to an array/slice header (elem type is not tracked in v0).
    Ptr,
}

impl IrTy {
    pub fn as_str(self) -> &'static str {
        match self {
            IrTy::Unit => "unit",
            IrTy::Bool => "bool",
            IrTy::I64 => "i64",
            IrTy::F64 => "f64",
            IrTy::Str => "str",
            IrTy::Ptr => "ptr",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl CmpOp {
    pub fn as_str(self) -> &'static str {
        match self {
            CmpOp::Eq => "eq",
            CmpOp::Ne => "ne",
            CmpOp::Lt => "lt",
            CmpOp::Le => "le",
            CmpOp::Gt => "gt",
            CmpOp::Ge => "ge",
        }
    }
}

/// One SSA instruction. Produces at most one result value.
#[derive(Clone, Debug)]
pub enum Inst {
    /// `v = iconst i64 N`
    IConst(i64),
    /// `v = fconst f64 N` (bits preserved via f64).
    FConst(f64),
    /// `v = bconst bool`
    BConst(bool),
    /// `v = sconst "…"` — interned string constant.
    SConst(Symbol),
    /// `v = nil`
    Nil,
    /// Integer arithmetic.
    IAdd(ValueId, ValueId),
    ISub(ValueId, ValueId),
    IMul(ValueId, ValueId),
    IDiv(ValueId, ValueId),
    IRem(ValueId, ValueId),
    INeg(ValueId),
    /// Float arithmetic.
    FAdd(ValueId, ValueId),
    FSub(ValueId, ValueId),
    FMul(ValueId, ValueId),
    FDiv(ValueId, ValueId),
    FNeg(ValueId),
    /// Comparisons (produce bool).
    ICmp(CmpOp, ValueId, ValueId),
    FCmp(CmpOp, ValueId, ValueId),
    BNot(ValueId),
    /// `v = alloc site ty` — storage; site feeds escape analysis.
    Alloc {
        site: AllocSite,
        ty: IrTy,
        span: rynix_span::Span,
    },
    /// `v = load ptr`
    Load(ValueId),
    /// `store ptr, val` — no result.
    Store {
        ptr: ValueId,
        value: ValueId,
    },
    /// `v = call @fn(args…)`
    Call {
        func: FuncId,
        args: Vec<ValueId>,
    },
    /// `v = call_ext name(args…)` — unresolved / builtin external.
    CallExt {
        name: Symbol,
        args: Vec<ValueId>,
        ret: IrTy,
    },
    /// Begin an implicit bump region (Phase 6).
    RegionCreate {
        region: u32,
    },
    /// Reset a bump region at a dominating loop/handler scope.
    RegionReset {
        region: u32,
    },
    /// Compiler-injected heap free (GoFree-style) for a [`AllocSite`].
    Free {
        site: AllocSite,
    },
    /// `ret` / `ret val`
    Ret(Option<ValueId>),
    /// Unconditional branch with block arguments.
    Jump {
        target: BlockId,
        args: Vec<ValueId>,
    },
    /// Conditional branch.
    Br {
        cond: ValueId,
        then_target: BlockId,
        then_args: Vec<ValueId>,
        else_target: BlockId,
        else_args: Vec<ValueId>,
    },
    /// `unreachable`
    Unreachable,
}

impl Inst {
    pub fn is_terminator(&self) -> bool {
        matches!(
            self,
            Inst::Ret(_) | Inst::Jump { .. } | Inst::Br { .. } | Inst::Unreachable
        )
    }

    pub fn has_result(&self) -> bool {
        !matches!(
            self,
            Inst::Store { .. }
                | Inst::RegionCreate { .. }
                | Inst::RegionReset { .. }
                | Inst::Free { .. }
                | Inst::Ret(_)
                | Inst::Jump { .. }
                | Inst::Br { .. }
                | Inst::Unreachable
        )
    }
}

#[derive(Clone, Debug)]
pub struct Block {
    pub params: Vec<(ValueId, IrTy)>,
    pub insts: Vec<InstId>,
    /// Once sealed, no more predecessors may be added (Braun SSA).
    pub sealed: bool,
}

#[derive(Clone, Debug)]
pub struct ValueData {
    pub ty: IrTy,
    /// `None` for block parameters; `Some` for instruction results.
    pub def: Option<InstId>,
}

#[derive(Debug)]
pub struct Function {
    pub name: Symbol,
    pub params: Vec<(ValueId, IrTy)>,
    pub ret: IrTy,
    pub entry: BlockId,
    pub blocks: Vec<Block>,
    pub insts: Vec<Inst>,
    pub values: Vec<ValueData>,
    pub next_site: u32,
}

impl Function {
    pub fn block(&self, id: BlockId) -> &Block {
        &self.blocks[id.0 as usize]
    }

    pub fn block_mut(&mut self, id: BlockId) -> &mut Block {
        &mut self.blocks[id.0 as usize]
    }

    pub fn inst(&self, id: InstId) -> &Inst {
        &self.insts[id.0 as usize]
    }

    pub fn value(&self, id: ValueId) -> &ValueData {
        &self.values[id.0 as usize]
    }

    pub fn value_ty(&self, id: ValueId) -> IrTy {
        self.values[id.0 as usize].ty
    }
}

#[derive(Debug, Default)]
pub struct Module {
    pub funcs: Vec<Function>,
    /// Map from source-level function symbol → [`FuncId`] (filled by lowering).
    pub func_names: Vec<Symbol>,
}

impl Module {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn func(&self, id: FuncId) -> &Function {
        &self.funcs[id.0 as usize]
    }

    pub fn find_func(&self, name: Symbol) -> Option<FuncId> {
        self.func_names
            .iter()
            .position(|&n| n == name)
            .map(|i| FuncId(i as u32))
    }
}
