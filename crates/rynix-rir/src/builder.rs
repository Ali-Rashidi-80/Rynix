//! Imperative builder for a single [`Function`].

#![allow(clippy::match_same_arms)]

use rynix_span::Symbol;

use crate::ir::{
    AllocSite, Block, BlockId, FuncId, Function, Inst, InstId, IrTy, ValueData, ValueId,
};

pub struct FunctionBuilder {
    pub func: Function,
    current: BlockId,
}

impl FunctionBuilder {
    pub fn new(name: Symbol, ret: IrTy) -> Self {
        let mut func = Function {
            name,
            params: Vec::new(),
            ret,
            entry: BlockId(0),
            blocks: Vec::new(),
            insts: Vec::new(),
            values: Vec::new(),
            next_site: 0,
            stack_bindings: Vec::new(),
        };
        let entry = Self::alloc_block_raw(&mut func);
        func.entry = entry;
        func.block_mut(entry).sealed = true;
        Self {
            func,
            current: entry,
        }
    }

    fn alloc_block_raw(func: &mut Function) -> BlockId {
        let id = BlockId(func.blocks.len() as u32);
        func.blocks.push(Block {
            params: Vec::new(),
            insts: Vec::new(),
            sealed: false,
        });
        id
    }

    pub fn create_block(&mut self) -> BlockId {
        Self::alloc_block_raw(&mut self.func)
    }

    pub fn seal_block(&mut self, id: BlockId) {
        self.func.block_mut(id).sealed = true;
    }

    pub fn switch_to(&mut self, id: BlockId) {
        self.current = id;
    }

    pub fn current(&self) -> BlockId {
        self.current
    }

    pub fn add_param(&mut self, ty: IrTy) -> ValueId {
        let v = self.alloc_value(ty, None);
        self.func.params.push((v, ty));
        // Entry block params mirror function params for uniformity.
        self.func.block_mut(self.func.entry).params.push((v, ty));
        v
    }

    pub fn append_block_param(&mut self, block: BlockId, ty: IrTy) -> ValueId {
        let v = self.alloc_value(ty, None);
        self.func.block_mut(block).params.push((v, ty));
        v
    }

    fn alloc_value(&mut self, ty: IrTy, def: Option<InstId>) -> ValueId {
        let id = ValueId(self.func.values.len() as u32);
        self.func.values.push(ValueData { ty, def });
        id
    }

    pub fn push(&mut self, inst: Inst) -> Option<ValueId> {
        let id = InstId(self.func.insts.len() as u32);
        let has_result = inst.has_result();
        let ty = self.result_ty(&inst);
        self.func.insts.push(inst);
        self.func.block_mut(self.current).insts.push(id);
        if has_result {
            let v = self.alloc_value(ty, Some(id));
            Some(v)
        } else {
            None
        }
    }

    /// Like [`push`] but asserts the instruction produces a value.
    pub fn push_value(&mut self, inst: Inst) -> ValueId {
        self.push(inst).expect("instruction must produce a value")
    }

    fn result_ty(&self, inst: &Inst) -> IrTy {
        match inst {
            Inst::IConst(_)
            | Inst::IAdd(_, _)
            | Inst::ISub(_, _)
            | Inst::IMul(_, _)
            | Inst::IDiv(_, _)
            | Inst::IRem(_, _)
            | Inst::URem(_, _)
            | Inst::IAnd(_, _)
            | Inst::IOr(_, _)
            | Inst::LShr(_, _)
            | Inst::LShl(_, _)
            | Inst::INeg(_)
            | Inst::ZExtI64(_)
            | Inst::CtPop(_)
            | Inst::Cttz(_) => IrTy::I64,
            Inst::FConst(_)
            | Inst::FAdd(_, _)
            | Inst::FSub(_, _)
            | Inst::FMul(_, _)
            | Inst::FDiv(_, _)
            | Inst::FNeg(_) => IrTy::F64,
            Inst::BConst(_)
            | Inst::ICmp(_, _, _)
            | Inst::FCmp(_, _, _)
            | Inst::BNot(_)
            | Inst::BAnd(_, _)
            | Inst::BOr(_, _) => IrTy::Bool,
            Inst::SConst(_) => IrTy::Str,
            Inst::Nil => IrTy::Unit,
            Inst::Alloc { ty, .. } => *ty, // pointer-ish; we use the slot's logical ty as Ptr surface
            Inst::Load(p) => {
                // Load returns the allocated slot type stored on the pointer value.
                self.func.value_ty(*p)
            }
            Inst::GepI64 { .. } => IrTy::Ptr,
            Inst::LoadIndex { .. } => IrTy::I64,
            Inst::ArrayLen(_) => IrTy::I64,
            Inst::Call { func: _, args: _ } => IrTy::Unit, // patched by caller via push_call
            Inst::CallExt { ret, .. } => *ret,
            Inst::Store { .. }
            | Inst::BoundsCheck { .. }
            | Inst::RegionCreate { .. }
            | Inst::RegionReset { .. }
            | Inst::Free { .. }
            | Inst::Ret(_)
            | Inst::Jump { .. }
            | Inst::Br { .. }
            | Inst::Unreachable => IrTy::Unit,
        }
    }

    pub fn iconst(&mut self, n: i64) -> ValueId {
        self.push_value(Inst::IConst(n))
    }

    pub fn fconst(&mut self, n: f64) -> ValueId {
        self.push_value(Inst::FConst(n))
    }

    pub fn bconst(&mut self, b: bool) -> ValueId {
        self.push_value(Inst::BConst(b))
    }

    pub fn sconst(&mut self, s: Symbol) -> ValueId {
        self.push_value(Inst::SConst(s))
    }

    pub fn alloc(&mut self, ty: IrTy, span: rynix_span::Span) -> ValueId {
        let site = AllocSite(self.func.next_site);
        self.func.next_site += 1;
        self.emit_alloc(site, ty, span)
    }

    /// Reserve an allocation site for `--explain-alloc` while keeping a `let mut` in SSA form.
    pub fn reserve_stack_binding(&mut self, ty: IrTy, span: rynix_span::Span) -> AllocSite {
        let site = AllocSite(self.func.next_site);
        self.func.next_site += 1;
        self.func.stack_bindings.push(crate::ir::StackBinding { site, span, ty });
        site
    }

    /// Materialize a previously reserved `let mut` site to a real stack `alloca`.
    pub fn alloc_at_site(&mut self, site: AllocSite, ty: IrTy, span: rynix_span::Span) -> ValueId {
        self.emit_alloc(site, ty, span)
    }

    fn emit_alloc(&mut self, site: AllocSite, ty: IrTy, span: rynix_span::Span) -> ValueId {
        let v = self.push_value(Inst::Alloc { site, ty, span });
        // Overwrite value ty to Ptr for the address, keep payload in inst.
        self.func.values[v.0 as usize].ty = IrTy::Ptr;
        v
    }

    pub fn load(&mut self, ptr: ValueId) -> ValueId {
        // Recover payload type from the Alloc inst if possible.
        let payload = self.payload_ty(ptr).unwrap_or(IrTy::I64);
        let id = InstId(self.func.insts.len() as u32);
        self.func.insts.push(Inst::Load(ptr));
        self.func.block_mut(self.current).insts.push(id);
        self.alloc_value(payload, Some(id))
    }

    fn payload_ty(&self, ptr: ValueId) -> Option<IrTy> {
        let def = self.func.value(ptr).def?;
        match self.func.inst(def) {
            Inst::Alloc { ty, .. } => Some(*ty),
            _ => None,
        }
    }

    pub fn store(&mut self, ptr: ValueId, value: ValueId) {
        let _ = self.push(Inst::Store { ptr, value });
    }

    pub fn call(&mut self, func: FuncId, args: Vec<ValueId>, ret: IrTy) -> ValueId {
        let id = InstId(self.func.insts.len() as u32);
        self.func.insts.push(Inst::Call { func, args });
        self.func.block_mut(self.current).insts.push(id);
        self.alloc_value(ret, Some(id))
    }

    pub fn call_ext(&mut self, name: Symbol, args: Vec<ValueId>, ret: IrTy) -> ValueId {
        self.push_value(Inst::CallExt { name, args, ret })
    }

    pub fn ret(&mut self, value: Option<ValueId>) {
        let _ = self.push(Inst::Ret(value));
    }

    pub fn jump(&mut self, target: BlockId, args: Vec<ValueId>) {
        let _ = self.push(Inst::Jump { target, args });
    }

    pub fn br(
        &mut self,
        cond: ValueId,
        then_target: BlockId,
        then_args: Vec<ValueId>,
        else_target: BlockId,
        else_args: Vec<ValueId>,
    ) {
        let _ = self.push(Inst::Br {
            cond,
            then_target,
            then_args,
            else_target,
            else_args,
        });
    }

    pub fn finish(self) -> Function {
        self.func
    }
}
