//! Emit textual LLVM IR from a RIR module.

#![allow(clippy::too_many_lines)]
#![allow(clippy::similar_names)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::trivially_copy_pass_by_ref)]
#![allow(clippy::map_unwrap_or)]

use std::fmt::Write as _;

use rynix_rir::{
    AllocSite, BlockId, CmpOp, EscapeReport, FuncId, Inst, IrTy, Module, Placement, ValueId,
};
use rynix_span::Interner;
use rustc_hash::{FxHashMap, FxHashSet};

/// Emit a complete LLVM module as text (no `target` triple — clang fills it in).
pub fn emit_llvm(
    module: &Module,
    interner: &Interner,
    report: Option<&EscapeReport>,
) -> String {
    let mut out = String::new();
    out.push_str("; ModuleID = 'rynix'\n");
    out.push_str("source_filename = \"rynix\"\n\n");

    // Runtime declarations.
    out.push_str("declare void @rynix_rt_print(ptr)\n");
    out.push_str("declare ptr @rynix_rt_heap_alloc(i64)\n");
    out.push_str("declare void @rynix_rt_heap_free(ptr)\n");
    out.push_str("declare void @rynix_rt_region_create(i32)\n");
    out.push_str("declare void @rynix_rt_region_reset(i32)\n");
    out.push_str("declare ptr @rynix_rt_region_alloc(i32, i64)\n");
    out.push_str("declare void @rynix_rt_panic(ptr)\n");
    out.push_str("declare ptr @rynix_rt_spawn(ptr, ptr)\n");
    out.push_str("declare void @rynix_rt_yield()\n");
    out.push_str("declare void @rynix_rt_sleep_ms(i64)\n");
    out.push_str("declare void @rynix_rt_run()\n");
    out.push_str("declare i64 @rynix_rt_fiber_count()\n");
    out.push_str("declare i64 @rynix_rt_read(i64, ptr, i64)\n");
    out.push_str("declare i64 @rynix_rt_write(i64, ptr, i64)\n");
    out.push_str("declare i64 @rynix_rt_now_ms()\n\n");
    out.push_str(
        "@.rynix.bounds = private unnamed_addr constant [20 x i8] c\"index out of bounds\\00\", align 1\n\n",
    );

    // String constants.
    let strings = collect_strings(module, interner);
    for (i, s) in strings.iter().enumerate() {
        let esc = llvm_string_bytes(s);
        let len = s.len() + 1; // NUL
        let _ = writeln!(
            out,
            "@.str.{i} = private unnamed_addr constant [{len} x i8] c\"{esc}\\00\", align 1"
        );
    }
    if !strings.is_empty() {
        out.push('\n');
    }

    let placement = build_placement_map(report);

    for (fi, func) in module.funcs.iter().enumerate() {
        let fid = FuncId(fi as u32);
        let name = interner.resolve(func.name);
        let is_main = name == "main";
        emit_function(
            &mut out,
            module,
            interner,
            fid,
            func,
            is_main,
            &strings,
            &placement,
        );
        out.push('\n');
    }

    out
}

fn build_placement_map(report: Option<&EscapeReport>) -> FxHashMap<(u32, u32), Placement> {
    let mut m = FxHashMap::default();
    if let Some(report) = report {
        for site in &report.sites {
            m.insert((site.func.0, site.site.0), site.placement);
        }
    }
    m
}

fn collect_strings(module: &Module, interner: &Interner) -> Vec<String> {
    let mut seen = FxHashSet::default();
    let mut out = Vec::new();
    for func in &module.funcs {
        for inst in &func.insts {
            if let Inst::SConst(sym) = inst {
                let s = interner.resolve(*sym).to_string();
                if seen.insert(s.clone()) {
                    out.push(s);
                }
            }
        }
    }
    out
}

fn llvm_string_bytes(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            b'\n' => out.push_str("\\0A"),
            b'\t' => out.push_str("\\09"),
            b'\0' => out.push_str("\\00"),
            0x20..=0x7e => out.push(b as char),
            _ => {
                let _ = write!(out, "\\{b:02X}");
            }
        }
    }
    out
}

fn llvm_ty(ty: IrTy) -> &'static str {
    match ty {
        IrTy::Unit => "void",
        IrTy::Bool => "i1",
        IrTy::I64 => "i64",
        IrTy::F64 => "double",
        IrTy::Str | IrTy::Ptr => "ptr",
    }
}

fn llvm_abi_ty(ty: IrTy) -> &'static str {
    // Unit returns become void; main is special-cased separately.
    llvm_ty(ty)
}

fn alloc_size(ty: IrTy) -> u64 {
    match ty {
        IrTy::Bool | IrTy::Unit => 1,
        IrTy::I64 | IrTy::F64 | IrTy::Str | IrTy::Ptr => 8,
    }
}

fn placement_for(
    map: &FxHashMap<(u32, u32), Placement>,
    fid: FuncId,
    site: AllocSite,
) -> Placement {
    map.get(&(fid.0, site.0))
        .copied()
        .unwrap_or(Placement::Stack)
}

struct EmitCtx<'a> {
    out: &'a mut String,
    vname: FxHashMap<ValueId, String>,
    strings: &'a [String],
    interner: &'a Interner,
    module: &'a Module,
    fid: FuncId,
    placement: &'a FxHashMap<(u32, u32), Placement>,
    next_tmp: u32,
}

impl EmitCtx<'_> {
    fn tmp(&mut self) -> String {
        let n = self.next_tmp;
        self.next_tmp += 1;
        format!("%t{n}")
    }

    fn val(&self, id: ValueId) -> String {
        self.vname
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("%v{}", id.0))
    }

    fn bind(&mut self, id: ValueId, name: String) {
        self.vname.insert(id, name);
    }
}

fn result_of(func: &rynix_rir::Function, iid: rynix_rir::InstId) -> Option<ValueId> {
    func.values
        .iter()
        .enumerate()
        .find(|(_, v)| v.def == Some(iid))
        .map(|(i, _)| ValueId(i as u32))
}

fn emit_function(
    out: &mut String,
    module: &Module,
    interner: &Interner,
    fid: FuncId,
    func: &rynix_rir::Function,
    is_main: bool,
    strings: &[String],
    placement: &FxHashMap<(u32, u32), Placement>,
) {
    let name = interner.resolve(func.name);
    let ret_ty = if is_main {
        "i32"
    } else if func.ret == IrTy::Unit {
        "void"
    } else {
        llvm_abi_ty(func.ret)
    };

    let params: Vec<String> = func
        .params
        .iter()
        .map(|(v, ty)| format!("{} %arg{}", llvm_ty(*ty), v.0))
        .collect();

    let _ = writeln!(
        out,
        "define {ret_ty} @{name}({params}) {{",
        params = params.join(", ")
    );

    let mut ctx = EmitCtx {
        out,
        vname: FxHashMap::default(),
        strings,
        interner,
        module,
        fid,
        placement,
        next_tmp: 0,
    };

    // Map entry params.
    for (v, _) in &func.params {
        ctx.bind(*v, format!("%arg{}", v.0));
    }

    // Predecessor map for phi.
    let preds = collect_preds(func);

    for (bi, block) in func.blocks.iter().enumerate() {
        let bid = BlockId(bi as u32);
        if bi == 0 {
            let _ = writeln!(ctx.out, "entry:");
        } else {
            let _ = writeln!(ctx.out, "b{bi}:");
        }

        // Phi for block params (skip entry — params are function args).
        if bi > 0 && !block.params.is_empty() {
            let pred_list = preds.get(&bid).cloned().unwrap_or_default();
            for (pi, (pvid, pty)) in block.params.iter().enumerate() {
                let name = format!("%bp{}_{}", bi, pvid.0);
                if pred_list.is_empty() {
                    let _ = writeln!(
                        ctx.out,
                        "  {name} = phi {} [ poison, %entry ]",
                        llvm_ty(*pty)
                    );
                } else {
                    let mut arms = Vec::new();
                    for (pred, args) in &pred_list {
                        let arg = args
                            .get(pi)
                            .map(|a| ctx.val(*a))
                            .unwrap_or_else(|| "poison".into());
                        let plabel = block_label(*pred);
                        arms.push(format!("[ {arg}, %{plabel} ]"));
                    }
                    let _ = writeln!(
                        ctx.out,
                        "  {name} = phi {} {}",
                        llvm_ty(*pty),
                        arms.join(", ")
                    );
                }
                ctx.bind(*pvid, name);
            }
        }

        for &iid in &block.insts {
            emit_inst(&mut ctx, func, iid, is_main);
        }
    }

    // If main somehow falls through without ret, add ret i32 0.
    // (Verifier should ensure terminators.)
    let _ = writeln!(ctx.out, "}}");
}

fn block_label(b: BlockId) -> String {
    if b.0 == 0 {
        "entry".into()
    } else {
        format!("b{}", b.0)
    }
}

fn collect_preds(
    func: &rynix_rir::Function,
) -> FxHashMap<BlockId, Vec<(BlockId, Vec<ValueId>)>> {
    let mut preds: FxHashMap<BlockId, Vec<(BlockId, Vec<ValueId>)>> = FxHashMap::default();
    for (bi, block) in func.blocks.iter().enumerate() {
        let from = BlockId(bi as u32);
        if let Some(&last) = block.insts.last() {
            match func.inst(last) {
                Inst::Jump { target, args } => {
                    preds.entry(*target).or_default().push((from, args.clone()));
                }
                Inst::Br {
                    then_target,
                    then_args,
                    else_target,
                    else_args,
                    ..
                } => {
                    preds
                        .entry(*then_target)
                        .or_default()
                        .push((from, then_args.clone()));
                    preds
                        .entry(*else_target)
                        .or_default()
                        .push((from, else_args.clone()));
                }
                _ => {}
            }
        }
    }
    preds
}

fn emit_inst(ctx: &mut EmitCtx<'_>, func: &rynix_rir::Function, iid: rynix_rir::InstId, is_main: bool) {
    let inst = func.inst(iid);
    let result = result_of(func, iid);

    match inst {
        Inst::IConst(n) => {
            let name = ctx.tmp();
            let _ = writeln!(ctx.out, "  {name} = add i64 0, {n}");
            ctx.bind(result.unwrap(), name);
        }
        Inst::FConst(n) => {
            let name = ctx.tmp();
            // Debug float formatting is accepted by LLVM's textual parser.
            let _ = writeln!(ctx.out, "  {name} = fadd double 0.0, {n:?}");
            ctx.bind(result.unwrap(), name);
        }
        Inst::BConst(b) => {
            let name = ctx.tmp();
            let _ = writeln!(
                ctx.out,
                "  {name} = add i1 0, {}",
                i32::from(*b)
            );
            ctx.bind(result.unwrap(), name);
        }
        Inst::SConst(sym) => {
            let s = ctx.interner.resolve(*sym);
            let idx = ctx
                .strings
                .iter()
                .position(|x| x == s)
                .expect("string collected");
            // Globals of array type decay to ptr in calls; bind the global directly.
            ctx.bind(result.unwrap(), format!("@.str.{idx}"));
        }
        Inst::Nil => {
            let name = ctx.tmp();
            let _ = writeln!(ctx.out, "  {name} = add i64 0, 0");
            ctx.bind(result.unwrap(), name);
        }
        Inst::IAdd(a, b) => bin_i(ctx, result, "add", a, b),
        Inst::ISub(a, b) => bin_i(ctx, result, "sub", a, b),
        Inst::IMul(a, b) => bin_i(ctx, result, "mul", a, b),
        Inst::IDiv(a, b) => bin_i(ctx, result, "sdiv", a, b),
        Inst::IRem(a, b) => bin_i(ctx, result, "srem", a, b),
        Inst::INeg(a) => {
            let name = ctx.tmp();
            let _ = writeln!(ctx.out, "  {name} = sub i64 0, {}", ctx.val(*a));
            ctx.bind(result.unwrap(), name);
        }
        Inst::FAdd(a, b) => bin_f(ctx, result, "fadd", a, b),
        Inst::FSub(a, b) => bin_f(ctx, result, "fsub", a, b),
        Inst::FMul(a, b) => bin_f(ctx, result, "fmul", a, b),
        Inst::FDiv(a, b) => bin_f(ctx, result, "fdiv", a, b),
        Inst::FNeg(a) => {
            let name = ctx.tmp();
            let _ = writeln!(ctx.out, "  {name} = fneg double {}", ctx.val(*a));
            ctx.bind(result.unwrap(), name);
        }
        Inst::ICmp(op, a, b) => {
            let name = ctx.tmp();
            let pred = icmp_pred(*op);
            let _ = writeln!(
                ctx.out,
                "  {name} = icmp {pred} i64 {}, {}",
                ctx.val(*a),
                ctx.val(*b)
            );
            ctx.bind(result.unwrap(), name);
        }
        Inst::FCmp(op, a, b) => {
            let name = ctx.tmp();
            let pred = fcmp_pred(*op);
            let _ = writeln!(
                ctx.out,
                "  {name} = fcmp {pred} double {}, {}",
                ctx.val(*a),
                ctx.val(*b)
            );
            ctx.bind(result.unwrap(), name);
        }
        Inst::BNot(a) => {
            let name = ctx.tmp();
            let _ = writeln!(ctx.out, "  {name} = xor i1 {}, true", ctx.val(*a));
            ctx.bind(result.unwrap(), name);
        }
        Inst::Alloc { site, ty, .. } => {
            let place = placement_for(ctx.placement, ctx.fid, *site);
            let name = ctx.tmp();
            let size = alloc_size(*ty);
            match place {
                Placement::Stack => {
                    let lty = match ty {
                        IrTy::Bool => "i8",
                        IrTy::I64 => "i64",
                        IrTy::F64 => "double",
                        IrTy::Str | IrTy::Ptr | IrTy::Unit => "ptr",
                    };
                    let _ = writeln!(ctx.out, "  {name} = alloca {lty}, align 8");
                }
                Placement::Region(rid) => {
                    let _ = writeln!(
                        ctx.out,
                        "  {name} = call ptr @rynix_rt_region_alloc(i32 {rid}, i64 {size})"
                    );
                }
                Placement::Heap => {
                    let _ = writeln!(
                        ctx.out,
                        "  {name} = call ptr @rynix_rt_heap_alloc(i64 {size})"
                    );
                }
            }
            ctx.bind(result.unwrap(), name);
        }
        Inst::Load(p) => {
            let name = ctx.tmp();
            // Payload type from alloc if possible.
            let lty = load_ty(func, *p);
            let _ = writeln!(
                ctx.out,
                "  {name} = load {lty}, ptr {}, align 8",
                ctx.val(*p)
            );
            // If bool stored as i8, trunc to i1.
            if lty == "i8" {
                let name2 = ctx.tmp();
                let _ = writeln!(ctx.out, "  {name2} = trunc i8 {name} to i1");
                ctx.bind(result.unwrap(), name2);
            } else {
                ctx.bind(result.unwrap(), name);
            }
        }
        Inst::Store { ptr, value } => {
            let lty = load_ty(func, *ptr);
            let mut v = ctx.val(*value);
            if lty == "i8" {
                let name = ctx.tmp();
                let _ = writeln!(ctx.out, "  {name} = zext i1 {v} to i8");
                v = name;
            }
            let _ = writeln!(
                ctx.out,
                "  store {lty} {v}, ptr {}, align 8",
                ctx.val(*ptr)
            );
        }
        Inst::GepI64 { base, index } => {
            let name = ctx.tmp();
            let _ = writeln!(
                ctx.out,
                "  {name} = getelementptr inbounds i64, ptr {}, i64 {}",
                ctx.val(*base),
                ctx.val(*index)
            );
            ctx.bind(result.unwrap(), name);
        }
        Inst::BoundsCheck { index, len } => {
            let ok_lo = ctx.tmp();
            let ok_hi = ctx.tmp();
            let ok = ctx.tmp();
            let cont = format!("bc_ok.{}", ctx.next_tmp);
            let fail = format!("bc_fail.{}", ctx.next_tmp);
            ctx.next_tmp += 1;
            let _ = writeln!(
                ctx.out,
                "  {ok_lo} = icmp sge i64 {}, 0",
                ctx.val(*index)
            );
            let _ = writeln!(
                ctx.out,
                "  {ok_hi} = icmp slt i64 {}, {}",
                ctx.val(*index),
                ctx.val(*len)
            );
            let _ = writeln!(ctx.out, "  {ok} = and i1 {ok_lo}, {ok_hi}");
            let _ = writeln!(ctx.out, "  br i1 {ok}, label %{cont}, label %{fail}");
            let _ = writeln!(ctx.out, "{fail}:");
            let _ = writeln!(
                ctx.out,
                "  call void @rynix_rt_panic(ptr @.rynix.bounds)"
            );
            let _ = writeln!(ctx.out, "  unreachable");
            let _ = writeln!(ctx.out, "{cont}:");
        }
        Inst::ArrayLen(base) => {
            let name = ctx.tmp();
            let _ = writeln!(
                ctx.out,
                "  {name} = load i64, ptr {}, align 8",
                ctx.val(*base)
            );
            ctx.bind(result.unwrap(), name);
        }
        Inst::LoadIndex { base, index } => {
            let one = ctx.tmp();
            let off = ctx.tmp();
            let slot = ctx.tmp();
            let name = ctx.tmp();
            let _ = writeln!(ctx.out, "  {one} = add i64 0, 1");
            let _ = writeln!(
                ctx.out,
                "  {off} = add i64 {}, {one}",
                ctx.val(*index)
            );
            let _ = writeln!(
                ctx.out,
                "  {slot} = getelementptr inbounds i64, ptr {}, i64 {off}",
                ctx.val(*base)
            );
            let _ = writeln!(ctx.out, "  {name} = load i64, ptr {slot}, align 8");
            ctx.bind(result.unwrap(), name);
        }
        Inst::Free { ptr, .. } => {
            let _ = writeln!(
                ctx.out,
                "  call void @rynix_rt_heap_free(ptr {})",
                ctx.val(*ptr)
            );
        }
        Inst::Call { func: callee, args } => {
            let cf = ctx.module.func(*callee);
            let cname = ctx.interner.resolve(cf.name);
            let arg_s: Vec<_> = cf
                .params
                .iter()
                .zip(args.iter())
                .map(|((_, ty), a)| format!("{} {}", llvm_ty(*ty), ctx.val(*a)))
                .collect();
            let rty = llvm_abi_ty(cf.ret);
            if cf.ret == IrTy::Unit {
                let _ = writeln!(
                    ctx.out,
                    "  call void @{cname}({args})",
                    args = arg_s.join(", ")
                );
                if let Some(r) = result {
                    let name = ctx.tmp();
                    let _ = writeln!(ctx.out, "  {name} = add i64 0, 0");
                    ctx.bind(r, name);
                }
            } else {
                let name = ctx.tmp();
                let _ = writeln!(
                    ctx.out,
                    "  {name} = call {rty} @{cname}({args})",
                    args = arg_s.join(", ")
                );
                ctx.bind(result.unwrap(), name);
            }
        }
        Inst::CallExt { name, args, ret } => {
            let n = ctx.interner.resolve(*name);
            if n == "print" || n == "println" {
                let arg = if let Some(a) = args.first() {
                    ctx.val(*a)
                } else {
                    let t = ctx.tmp();
                    let _ = writeln!(ctx.out, "  {t} = inttoptr i64 0 to ptr");
                    t
                };
                let _ = writeln!(ctx.out, "  call void @rynix_rt_print(ptr {arg})");
                if let Some(r) = result {
                    let t = ctx.tmp();
                    let _ = writeln!(ctx.out, "  {t} = add i64 0, 0");
                    ctx.bind(r, t);
                }
            } else if n == "rynix_rt_heap_alloc" {
                let size = args
                    .first()
                    .map(|a| ctx.val(*a))
                    .unwrap_or_else(|| "0".into());
                let t = ctx.tmp();
                let _ = writeln!(
                    ctx.out,
                    "  {t} = call ptr @rynix_rt_heap_alloc(i64 {size})"
                );
                ctx.bind(result.unwrap(), t);
            } else if n == "rynix_rt_spawn" {
                // First arg may be an sconst naming the fiber entry; bitcast @name.
                let fn_ptr = if let Some(a) = args.first() {
                    // If bound to a string global, we cannot bitcast easily; use null.
                    // Prefer looking up function by resolving sconst content.
                    let _ = a;
                    "null".to_string()
                } else {
                    "null".into()
                };
                // Better: if first arg is SConst of a known function name, use @name.
                let fn_ptr = match args.first().and_then(|a| {
                    // Recover symbol from RIR if this value is an SConst.
                    for (ii, inst) in func.insts.iter().enumerate() {
                        if let Inst::SConst(sym) = inst {
                            let iid = rynix_rir::InstId(ii as u32);
                            if result_of(func, iid) == Some(*a) {
                                let name = ctx.interner.resolve(*sym);
                                if ctx.module.func_names.iter().any(|&n| {
                                    ctx.interner.resolve(n) == name
                                }) {
                                    return Some(format!("@{name}"));
                                }
                            }
                        }
                    }
                    None
                }) {
                    Some(p) => p,
                    None => fn_ptr,
                };
                let t = ctx.tmp();
                let _ = writeln!(
                    ctx.out,
                    "  {t} = call ptr @rynix_rt_spawn(ptr {fn_ptr}, ptr null)"
                );
                if let Some(r) = result {
                    ctx.bind(r, t);
                }
            } else if n == "sleep_ms" || n == "rynix_rt_sleep_ms" {
                let ms = args
                    .first()
                    .map(|a| ctx.val(*a))
                    .unwrap_or_else(|| "0".into());
                let _ = writeln!(ctx.out, "  call void @rynix_rt_sleep_ms(i64 {ms})");
                if let Some(r) = result {
                    let t = ctx.tmp();
                    let _ = writeln!(ctx.out, "  {t} = add i64 0, 0");
                    ctx.bind(r, t);
                }
            } else if n == "yield" || n == "rynix_rt_yield" {
                let _ = writeln!(ctx.out, "  call void @rynix_rt_yield()");
                if let Some(r) = result {
                    let t = ctx.tmp();
                    let _ = writeln!(ctx.out, "  {t} = add i64 0, 0");
                    ctx.bind(r, t);
                }
            } else if n == "now_ms" || n == "rynix_rt_now_ms" {
                let t = ctx.tmp();
                let _ = writeln!(ctx.out, "  {t} = call i64 @rynix_rt_now_ms()");
                ctx.bind(result.unwrap(), t);
            } else {
                // Unknown external: declare on the fly and call.
                let rty = llvm_abi_ty(*ret);
                let arg_s: Vec<_> = args
                    .iter()
                    .map(|a| format!("ptr {}", ctx.val(*a)))
                    .collect();
                if *ret == IrTy::Unit {
                    let _ = writeln!(
                        ctx.out,
                        "  call void @{n}({args})",
                        args = arg_s.join(", ")
                    );
                } else {
                    let t = ctx.tmp();
                    let _ = writeln!(
                        ctx.out,
                        "  {t} = call {rty} @{n}({args})",
                        args = arg_s.join(", ")
                    );
                    ctx.bind(result.unwrap(), t);
                }
            }
        }
        Inst::RegionCreate { region } => {
            let _ = writeln!(
                ctx.out,
                "  call void @rynix_rt_region_create(i32 {region})"
            );
        }
        Inst::RegionReset { region } => {
            let _ = writeln!(
                ctx.out,
                "  call void @rynix_rt_region_reset(i32 {region})"
            );
        }
        Inst::Ret(None) => {
            if is_main {
                let _ = writeln!(ctx.out, "  ret i32 0");
            } else {
                let _ = writeln!(ctx.out, "  ret void");
            }
        }
        Inst::Ret(Some(v)) => {
            if is_main {
                // Truncate i64 → i32 exit code when possible.
                let ty = func.value_ty(*v);
                match ty {
                    IrTy::I64 => {
                        let t = ctx.tmp();
                        let _ = writeln!(ctx.out, "  {t} = trunc i64 {} to i32", ctx.val(*v));
                        let _ = writeln!(ctx.out, "  ret i32 {t}");
                    }
                    IrTy::Bool => {
                        let t = ctx.tmp();
                        let _ = writeln!(ctx.out, "  {t} = zext i1 {} to i32", ctx.val(*v));
                        let _ = writeln!(ctx.out, "  ret i32 {t}");
                    }
                    _ => {
                        let _ = writeln!(ctx.out, "  ret i32 0");
                    }
                }
            } else if func.ret == IrTy::Unit {
                let _ = writeln!(ctx.out, "  ret void");
            } else {
                let _ = writeln!(
                    ctx.out,
                    "  ret {} {}",
                    llvm_abi_ty(func.ret),
                    ctx.val(*v)
                );
            }
        }
        Inst::Jump { target, .. } => {
            let _ = writeln!(ctx.out, "  br label %{}", block_label(*target));
        }
        Inst::Br {
            cond,
            then_target,
            else_target,
            ..
        } => {
            let _ = writeln!(
                ctx.out,
                "  br i1 {}, label %{}, label %{}",
                ctx.val(*cond),
                block_label(*then_target),
                block_label(*else_target)
            );
        }
        Inst::Unreachable => {
            let _ = writeln!(ctx.out, "  unreachable");
        }
    }
}

fn bin_i(ctx: &mut EmitCtx<'_>, result: Option<ValueId>, op: &str, a: &ValueId, b: &ValueId) {
    let name = ctx.tmp();
    let _ = writeln!(
        ctx.out,
        "  {name} = {op} i64 {}, {}",
        ctx.val(*a),
        ctx.val(*b)
    );
    ctx.bind(result.unwrap(), name);
}

fn bin_f(ctx: &mut EmitCtx<'_>, result: Option<ValueId>, op: &str, a: &ValueId, b: &ValueId) {
    let name = ctx.tmp();
    let _ = writeln!(
        ctx.out,
        "  {name} = {op} double {}, {}",
        ctx.val(*a),
        ctx.val(*b)
    );
    ctx.bind(result.unwrap(), name);
}

fn icmp_pred(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "eq",
        CmpOp::Ne => "ne",
        CmpOp::Lt => "slt",
        CmpOp::Le => "sle",
        CmpOp::Gt => "sgt",
        CmpOp::Ge => "sge",
    }
}

fn fcmp_pred(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "oeq",
        CmpOp::Ne => "one",
        CmpOp::Lt => "olt",
        CmpOp::Le => "ole",
        CmpOp::Gt => "ogt",
        CmpOp::Ge => "oge",
    }
}

fn load_ty(func: &rynix_rir::Function, ptr: ValueId) -> &'static str {
    if let Some(def) = func.value(ptr).def
        && let Inst::Alloc { ty, .. } = func.inst(def)
    {
        return match ty {
            IrTy::Bool => "i8",
            IrTy::I64 => "i64",
            IrTy::F64 => "double",
            IrTy::Str | IrTy::Ptr | IrTy::Unit => "ptr",
        };
    }
    "i64"
}
