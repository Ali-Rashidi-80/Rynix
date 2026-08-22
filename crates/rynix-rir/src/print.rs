//! Textual `.rir` printer.

use std::fmt::Write as _;

use rynix_span::Interner;

use crate::ir::{BlockId, Inst, Module, ValueId};

/// Pretty-print a module as canonical textual RIR.
pub fn print_module(module: &Module, interner: &Interner) -> String {
    let mut out = String::new();
    for (i, func) in module.funcs.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let name = interner.resolve(func.name);
        let params: Vec<_> = func
            .params
            .iter()
            .map(|(v, ty)| format!("%{vid}:{ty}", vid = v.0, ty = ty.as_str()))
            .collect();
        let _ = writeln!(
            out,
            "func @{name}({params}) -> {ret}:",
            params = params.join(", "),
            ret = func.ret.as_str()
        );
        for (bi, block) in func.blocks.iter().enumerate() {
            let bid = BlockId(bi as u32);
            let bparams: Vec<_> = block
                .params
                .iter()
                .map(|(v, ty)| format!("%{vid}:{ty}", vid = v.0, ty = ty.as_str()))
                .collect();
            if bparams.is_empty() {
                let _ = writeln!(out, "block{bi}:");
            } else {
                let _ = writeln!(out, "block{bi}({}):", bparams.join(", "));
            }
            for &iid in &block.insts {
                let inst = func.inst(iid);
                let result = func
                    .values
                    .iter()
                    .enumerate()
                    .find(|(_, v)| v.def == Some(iid))
                    .map(|(vi, _)| ValueId(vi as u32));
                let _ = writeln!(
                    out,
                    "  {line}",
                    line = format_inst(module, interner, result, inst)
                );
            }
            let _ = bid;
        }
    }
    out
}

fn format_inst(
    module: &Module,
    interner: &Interner,
    result: Option<ValueId>,
    inst: &Inst,
) -> String {
    let lhs = result.map_or(String::new(), |v| format!("%{} = ", v.0));
    match inst {
        Inst::IConst(n) => format!("{lhs}iconst {n}"),
        Inst::FConst(n) => format!("{lhs}fconst {n}"),
        Inst::BConst(b) => format!("{lhs}bconst {b}"),
        Inst::SConst(s) => format!("{lhs}sconst \"{}\"", interner.resolve(*s).escape_debug()),
        Inst::Nil => format!("{lhs}nil"),
        Inst::IAdd(a, b) => format!("{lhs}iadd {}, {}", v(*a), v(*b)),
        Inst::ISub(a, b) => format!("{lhs}isub {}, {}", v(*a), v(*b)),
        Inst::IMul(a, b) => format!("{lhs}imul {}, {}", v(*a), v(*b)),
        Inst::IDiv(a, b) => format!("{lhs}idiv {}, {}", v(*a), v(*b)),
        Inst::IRem(a, b) => format!("{lhs}irem {}, {}", v(*a), v(*b)),
        Inst::URem(a, b) => format!("{lhs}urem {}, {}", v(*a), v(*b)),
        Inst::IAnd(a, b) => format!("{lhs}iand {}, {}", v(*a), v(*b)),
        Inst::LShr(a, b) => format!("{lhs}lshr {}, {}", v(*a), v(*b)),
        Inst::LShl(a, b) => format!("{lhs}lshl {}, {}", v(*a), v(*b)),
        Inst::INeg(a) => format!("{lhs}ineg {}", v(*a)),
        Inst::FAdd(a, b) => format!("{lhs}fadd {}, {}", v(*a), v(*b)),
        Inst::FSub(a, b) => format!("{lhs}fsub {}, {}", v(*a), v(*b)),
        Inst::FMul(a, b) => format!("{lhs}fmul {}, {}", v(*a), v(*b)),
        Inst::FDiv(a, b) => format!("{lhs}fdiv {}, {}", v(*a), v(*b)),
        Inst::FNeg(a) => format!("{lhs}fneg {}", v(*a)),
        Inst::ICmp(op, a, b) => format!("{lhs}icmp {} {}, {}", op.as_str(), v(*a), v(*b)),
        Inst::FCmp(op, a, b) => format!("{lhs}fcmp {} {}, {}", op.as_str(), v(*a), v(*b)),
        Inst::BNot(a) => format!("{lhs}bnot {}", v(*a)),
        Inst::BAnd(a, b) => format!("{lhs}band {}, {}", v(*a), v(*b)),
        Inst::BOr(a, b) => format!("{lhs}bor {}, {}", v(*a), v(*b)),
        Inst::ZExtI64(a) => format!("{lhs}zext_i64 {}", v(*a)),
        Inst::CtPop(a) => format!("{lhs}ctpop {}", v(*a)),
        Inst::Alloc { site, ty, .. } => format!("{lhs}alloc site{} {}", site.0, ty.as_str()),
        Inst::Load(p) => format!("{lhs}load {}", v(*p)),
        Inst::Store { ptr, value } => format!("store {}, {}", v(*ptr), v(*value)),
        Inst::GepI64 { base, index } => format!("{lhs}gep_i64 {}, {}", v(*base), v(*index)),
        Inst::BoundsCheck { index, len } => format!("bounds_check {}, {}", v(*index), v(*len)),
        Inst::LoadIndex { base, index } => {
            format!("{lhs}load_index {}, {}", v(*base), v(*index))
        }
        Inst::ArrayLen(p) => format!("{lhs}array_len {}", v(*p)),
        Inst::RegionCreate { region } => format!("region_create {region}"),
        Inst::RegionReset { region } => format!("region_reset {region}"),
        Inst::Free { site, ptr } => format!("free site{} {}", site.0, v(*ptr)),
        Inst::Call { func, args } => {
            let name = module
                .funcs
                .get(func.0 as usize)
                .map_or("?", |f| interner.resolve(f.name));
            let args = args.iter().map(|a| v(*a)).collect::<Vec<_>>().join(", ");
            format!("{lhs}call @{name}({args})")
        }
        Inst::CallExt { name, args, ret } => {
            let args = args.iter().map(|a| v(*a)).collect::<Vec<_>>().join(", ");
            format!(
                "{lhs}call_ext @{}({args}) -> {}",
                interner.resolve(*name),
                ret.as_str()
            )
        }
        Inst::Ret(None) => "ret".into(),
        Inst::Ret(Some(val)) => format!("ret {}", v(*val)),
        Inst::Jump { target, args } => {
            let args = args.iter().map(|a| v(*a)).collect::<Vec<_>>().join(", ");
            if args.is_empty() {
                format!("jump block{}", target.0)
            } else {
                format!("jump block{}({args})", target.0)
            }
        }
        Inst::Br {
            cond,
            then_target,
            then_args,
            else_target,
            else_args,
        } => format!(
            "br {} block{}({}) block{}({})",
            v(*cond),
            then_target.0,
            then_args.iter().map(|a| v(*a)).collect::<Vec<_>>().join(", "),
            else_target.0,
            else_args.iter().map(|a| v(*a)).collect::<Vec<_>>().join(", ")
        ),
        Inst::Unreachable => "unreachable".into(),
    }
}

fn v(id: ValueId) -> String {
    format!("%{}", id.0)
}