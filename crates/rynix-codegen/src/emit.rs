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

/// Emit a complete LLVM module as text.
///
/// When `target_triple` is `None`, omit the triple (host clang fills it in).
/// When set (Phase 13: `wasm32-unknown-unknown`), write triple + wasm datalayout.
pub fn emit_llvm(
    module: &Module,
    interner: &Interner,
    report: Option<&EscapeReport>,
) -> String {
    emit_llvm_with_target(module, interner, report, None)
}

/// Like [`emit_llvm`], with an optional LLVM target triple for cross emit.
pub fn emit_llvm_with_target(
    module: &Module,
    interner: &Interner,
    report: Option<&EscapeReport>,
    target_triple: Option<&str>,
) -> String {
    let wasm_host = target_triple.is_some_and(|t| t.starts_with("wasm32"));
    let mut out = String::new();
    out.push_str("; ModuleID = 'rynix'\n");
    out.push_str("source_filename = \"rynix\"\n");
    if let Some(triple) = target_triple {
        let _ = writeln!(out, "target triple = \"{triple}\"");
        if triple.starts_with("wasm32") {
            out.push_str(
                "target datalayout = \"e-m:e-p:32:32-p10:8:8-p20:32:32-i64:64-n32:64-S128-ni:1:10:20\"\n",
            );
        }
    }
    out.push('\n');

    // Runtime declarations.
    out.push_str("declare void @rynix_rt_print(ptr)\n");
    // Freestanding wasm: print_i64 is a host import (env.print_i64), not rt/.
    if wasm_host {
        out.push_str("declare void @print_i64(i64) #0\n");
    } else {
        out.push_str("declare void @rynix_rt_print_i64(i64)\n");
    }
    out.push_str("declare i64 @rynix_rt_opaque_i64(i64)\n");
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
    out.push_str("declare i64 @rynix_rt_now_ms()\n");
    out.push_str("declare ptr @rynix_rt_vec_i64_new(i32)\n");
    out.push_str("declare void @rynix_rt_vec_i64_push(ptr, i64)\n");
    out.push_str("declare i64 @rynix_rt_vec_i64_get(ptr, i64)\n");
    out.push_str("declare i64 @rynix_rt_vec_i64_len(ptr)\n");
    out.push_str("declare ptr @rynix_rt_vec_str_new(i32)\n");
    out.push_str("declare void @rynix_rt_vec_str_push(ptr, ptr)\n");
    out.push_str("declare ptr @rynix_rt_vec_str_get(ptr, i64)\n");
    out.push_str("declare i64 @rynix_rt_vec_str_len(ptr)\n");
    out.push_str("declare ptr @rynix_rt_map_i64_new(i32)\n");
    out.push_str("declare void @rynix_rt_map_i64_insert(ptr, i64, i64)\n");
    out.push_str("declare i64 @rynix_rt_map_i64_get(ptr, i64)\n");
    out.push_str("declare i64 @rynix_rt_map_i64_len(ptr)\n");
    out.push_str("declare ptr @rynix_rt_map_str_i64_new(i32)\n");
    out.push_str("declare void @rynix_rt_map_str_i64_insert(ptr, ptr, i64)\n");
    out.push_str("declare i64 @rynix_rt_map_str_i64_get(ptr, ptr)\n");
    out.push_str("declare i64 @rynix_rt_map_str_i64_len(ptr)\n");
    out.push_str("declare ptr @rynix_rt_map_str_str_new(i32)\n");
    out.push_str("declare void @rynix_rt_map_str_str_insert(ptr, ptr, ptr)\n");
    out.push_str("declare ptr @rynix_rt_map_str_str_get(ptr, ptr)\n");
    out.push_str("declare i64 @rynix_rt_map_str_str_len(ptr)\n");
    out.push_str("declare i64 @rynix_rt_tcp_listen(i64)\n");
    out.push_str("declare i64 @rynix_rt_tcp_accept(i64)\n");
    out.push_str("declare i64 @rynix_rt_tcp_connect(ptr, i64)\n");
    out.push_str("declare i64 @rynix_rt_tcp_recv(i64, ptr, i64)\n");
    out.push_str("declare i64 @rynix_rt_tcp_send(i64, ptr, i64)\n");
    out.push_str("declare i64 @llvm.ctpop.i64(i64)\n");
    out.push_str("declare i64 @llvm.cttz.i64(i64, i1)\n");
    out.push_str("declare void @rynix_rt_tcp_close(i64)\n");
    out.push_str("declare i64 @rynix_rt_json_get_i64(ptr, ptr)\n");
    out.push_str("declare i64 @rynix_rt_json_has_i64(ptr, ptr)\n");
    out.push_str("declare i64 @rynix_rt_http_get_json_i64(ptr, i64, ptr, ptr)\n");
    out.push_str("declare i64 @rynix_rt_http_post_json_i64(ptr, i64, ptr, ptr, ptr)\n");
    out.push_str("declare i64 @rynix_rt_http_serve_once_json_i64(i64, ptr, i64)\n");
    out.push_str("declare i64 @rynix_rt_http_serve_once_echo_json_i64(i64, ptr, ptr)\n");
    out.push_str("declare i64 @rynix_rt_http_serve_loop_json_i64(i64, ptr, i64, i64)\n");
    out.push_str(
        "declare i64 @rynix_rt_http_serve_loop_2paths_json_i64(i64, ptr, i64, ptr, i64, i64)\n",
    );
    out.push_str(
        "declare i64 @rynix_rt_http_serve_loop_3paths_json_i64(i64, ptr, i64, ptr, i64, ptr, i64, i64)\n",
    );
    out.push_str(
        "declare i64 @rynix_rt_http_serve_loop_path_param_json_i64(i64, ptr, i64)\n",
    );
    out.push_str(
        "declare i64 @rynix_rt_http_serve_loop_header_json_i64(i64, ptr, ptr, i64)\n",
    );
    out.push_str(
        "declare i64 @rynix_rt_http_serve_loop_post_echo_json_i64(i64, ptr, ptr, i64, i64)\n",
    );
    out.push_str(
        "declare i64 @rynix_rt_http_serve_loop_keepalive_json_i64(i64, ptr, i64, i64)\n",
    );
    out.push_str("declare i64 @rynix_rt_http_tls_serve_once_json_i64(i64, ptr, i64)\n");
    out.push_str("declare i64 @rynix_rt_http_tls_get_json_i64(ptr, i64, ptr, ptr)\n");
    out.push_str("declare i64 @rynix_rt_frame_serve_once_echo(i64)\n");
    out.push_str("declare i64 @rynix_rt_frame_client_echo(ptr, i64, ptr)\n");
    out.push_str("declare i64 @rynix_rt_tls_serve_once_echo(i64)\n");
    out.push_str("declare i64 @rynix_rt_tls_client_echo(ptr, i64, ptr)\n");
    out.push_str("declare i64 @rynix_rt_sha256_first_i64(ptr)\n");
    out.push_str("declare i64 @rynix_rt_hmac_sha256_first_i64(ptr, ptr)\n");
    out.push_str("declare i64 @rynix_rt_aes128_gcm_nist_empty_tag_first_i64()\n");
    out.push_str("declare i64 @rynix_rt_ws_accept_key_eq(ptr, ptr)\n");
    out.push_str("declare i64 @rynix_rt_ws_accept_sha1_first_i64(ptr)\n");
    out.push_str("declare i64 @rynix_rt_ws_frame_roundtrip_ok()\n");
    out.push_str("declare i64 @rynix_rt_ws_serve_once_echo(i64)\n");
    out.push_str("declare i64 @rynix_rt_ws_client_echo(ptr, i64, ptr)\n");
    out.push_str("declare ptr @rynix_rt_kv_new(i32)\n");
    out.push_str("declare void @rynix_rt_kv_put(ptr, ptr, i64)\n");
    out.push_str("declare i64 @rynix_rt_kv_get(ptr, ptr)\n");
    out.push_str("declare i64 @rynix_rt_kv_len(ptr)\n");
    out.push_str("declare i64 @rynix_rt_fs_write_file(ptr, ptr)\n");
    out.push_str("declare ptr @rynix_rt_fs_read_file(ptr)\n");
    out.push_str("declare i64 @rynix_rt_fs_read_file_eq(ptr, ptr)\n");
    out.push_str("declare i64 @rynix_rt_fs_exists(ptr)\n");
    out.push_str("declare i64 @rynix_rt_fs_remove_file(ptr)\n\n");
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
            wasm_host,
        );
        out.push('\n');
    }

    let mut need_vec = false;
    let mut need_unroll = false;
    for f in &module.funcs {
        for hint in loop_latch_hints(f).values() {
            match hint {
                LoopHint::Vectorize => need_vec = true,
                LoopHint::Unroll => need_unroll = true,
            }
        }
    }
    if need_vec || need_unroll {
        // Selective hints: SIMD-friendly latches keep vectorize + modest unroll;
        // loop-carried `urem` (hash/powmod) get a *bounded* unroll count only.
        out.push_str("; Rynix loop optimizer hints\n");
        out.push_str("!0 = distinct !{!0, !2, !3}\n");
        out.push_str("!1 = distinct !{!1, !4, !3}\n");
        out.push_str("!2 = !{!\"llvm.loop.vectorize.enable\", i1 true}\n");
        out.push_str("!3 = !{!\"llvm.loop.mustprogress\"}\n");
        // Bounded unroll for rem-heavy / nested-style latches (End `#pragma unroll`).
        out.push_str("!4 = !{!\"llvm.loop.unroll.count\", i32 4}\n");
    }

    if wasm_host {
        out.push_str(
            "attributes #0 = { \"wasm-import-module\"=\"env\" \"wasm-import-name\"=\"print_i64\" }\n",
        );
    }

    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoopHint {
    Vectorize,
    Unroll,
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
    final_out: &'a mut String,
    vname: FxHashMap<ValueId, String>,
    strings: &'a [String],
    interner: &'a Interner,
    module: &'a Module,
    fid: FuncId,
    placement: &'a FxHashMap<(u32, u32), Placement>,
    next_tmp: u32,
    /// Freestanding wasm32: call host-import `env.print_i64` instead of `rynix_rt_*`.
    wasm_host: bool,
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
    wasm_host: bool,
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

    let attrs = if name != "main" && func.insts.len() <= 64 && func.params.len() <= 4 {
        " alwaysinline"
    } else {
        ""
    };

    let _ = writeln!(
        out,
        "define {ret_ty} @{name}({params}){attrs} {{",
        params = params.join(", "),
        attrs = attrs
    );

    let mut block_bufs = vec![String::new(); func.blocks.len()];
    let mut ctx = EmitCtx {
        final_out: out,
        vname: FxHashMap::default(),
        strings,
        interner,
        module,
        fid,
        placement,
        next_tmp: 0,
        wasm_host,
    };

    // Map entry params.
    for (v, _) in &func.params {
        ctx.bind(*v, format!("%arg{}", v.0));
    }

    // Pre-bind block-param value ids to their phi names before pass 1.
    for (bi, block) in func.blocks.iter().enumerate() {
        if bi == 0 {
            continue;
        }
        for (pvid, _) in &block.params {
            ctx.bind(*pvid, format!("%bp{}_{}", bi, pvid.0));
        }
    }

    // Pass 1: emit block bodies so all SSA names exist before phi operands are resolved.
    let latches = loop_latch_hints(func);
    let reachable = reachable_blocks(func);
    for (bi, block) in func.blocks.iter().enumerate() {
        if !reachable[bi] {
            continue;
        }
        let latch = latches.get(&(bi as u32)).copied();
        for &iid in &block.insts {
            emit_inst(
                &mut block_bufs[bi],
                &mut ctx,
                func,
                iid,
                is_main,
                latch,
            );
        }
    }

    // Predecessor map for phi.
    let preds = collect_preds(func);

    // Pass 2: labels, phi nodes, then buffered bodies.
    for (bi, block) in func.blocks.iter().enumerate() {
        if !reachable[bi] {
            continue;
        }
        let bid = BlockId(bi as u32);
        if bi == 0 {
            let _ = writeln!(ctx.final_out, "entry:");
        } else {
            let _ = writeln!(ctx.final_out, "b{bi}:");
        }

        if bi > 0 && !block.params.is_empty() {
            let pred_list = preds.get(&bid).cloned().unwrap_or_default();
            // Only reachable predecessors — unreachable fallthrough blocks must not
            // appear in phi (Phase 22: inline match+return phantom joins).
            let pred_list: Vec<_> = pred_list
                .into_iter()
                .filter(|(pred, _)| reachable[pred.0 as usize])
                .collect();
            for (pi, (pvid, pty)) in block.params.iter().enumerate() {
                let name = format!("%bp{}_{}", bi, pvid.0);
                if pred_list.is_empty() {
                    let _ = writeln!(
                        ctx.final_out,
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
                        ctx.final_out,
                        "  {name} = phi {} {}",
                        llvm_ty(*pty),
                        arms.join(", ")
                    );
                }
            }
        }

        ctx.final_out.push_str(&block_bufs[bi]);
    }

    let _ = writeln!(ctx.final_out, "}}");
}

fn reachable_blocks(func: &rynix_rir::Function) -> Vec<bool> {
    let n = func.blocks.len();
    let mut reachable = vec![false; n];
    if n == 0 {
        return reachable;
    }
    let mut stack = vec![func.entry.0 as usize];
    reachable[func.entry.0 as usize] = true;
    while let Some(bi) = stack.pop() {
        let Some(&term) = func.blocks[bi].insts.last() else {
            continue;
        };
        match func.inst(term) {
            Inst::Jump { target, .. } => {
                let t = target.0 as usize;
                if t < n && !reachable[t] {
                    reachable[t] = true;
                    stack.push(t);
                }
            }
            Inst::Br {
                then_target,
                else_target,
                ..
            } => {
                for t in [then_target.0 as usize, else_target.0 as usize] {
                    if t < n && !reachable[t] {
                        reachable[t] = true;
                        stack.push(t);
                    }
                }
            }
            _ => {}
        }
    }
    reachable
}

/// True back-edges only: jump to an earlier header (structured lowering).
/// Forward jumps into an inner-loop header must not get `!llvm.loop` metadata.
fn loop_latch_hints(func: &rynix_rir::Function) -> FxHashMap<u32, LoopHint> {
    let mut latches = FxHashMap::default();
    let entry = func.entry.0;
    for (bi, block) in func.blocks.iter().enumerate() {
        let bi_u = bi as u32;
        if bi_u == entry {
            continue;
        }
        let Some(&term) = block.insts.last() else {
            continue;
        };
        let Inst::Jump { target, args } = func.inst(term) else {
            continue;
        };
        if target.0 >= bi_u || func.block(*target).params.is_empty() {
            continue;
        }
        let rem_defined: FxHashSet<ValueId> = block
            .insts
            .iter()
            .filter_map(|&iid| match func.inst(iid) {
                Inst::URem(..) | Inst::IRem(..) => result_of(func, iid),
                _ => None,
            })
            .collect();
        // Rem of the induction variable (reduce): keep vectorize.
        let rem_of_induction = block.insts.iter().any(|&iid| {
            let (Inst::URem(d, _) | Inst::IRem(d, _)) = func.inst(iid) else {
                return false;
            };
            block.insts.iter().any(|&jid| {
                let Inst::IAdd(a, b) = func.inst(jid) else {
                    return false;
                };
                let res = result_of(func, jid);
                (*a == *d || *b == *d) && res.is_some_and(|r| args.contains(&r))
            })
        });
        // Induction rem (reduce) → vectorize; pure arith → unroll; rem-heavy
        // (gcd / hash-style) → no forced metadata (clang `-funroll-loops` only).
        let hint = if rem_of_induction {
            Some(LoopHint::Vectorize)
        } else if rem_defined.is_empty() {
            Some(LoopHint::Unroll)
        } else {
            None
        };
        if let Some(hint) = hint {
            latches.insert(bi_u, hint);
        }
    }
    latches
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

fn emit_inst(
    out: &mut String,
    ctx: &mut EmitCtx<'_>,
    func: &rynix_rir::Function,
    iid: rynix_rir::InstId,
    is_main: bool,
    loop_latch: Option<LoopHint>,
) {
    let inst = func.inst(iid);
    let result = result_of(func, iid);

    match inst {
        Inst::IConst(n) => {
            let name = ctx.tmp();
            let _ = writeln!(out, "  {name} = add i64 0, {n}");
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::FConst(n) => {
            let name = ctx.tmp();
            // Debug float formatting is accepted by LLVM's textual parser.
            let _ = writeln!(out, "  {name} = fadd double 0.0, {n:?}");
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::BConst(b) => {
            let name = ctx.tmp();
            let _ = writeln!(
                out,
                "  {name} = add i1 0, {}",
                i32::from(*b)
            );
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::SConst(sym) => {
            let s = ctx.interner.resolve(*sym);
            let idx = ctx
                .strings
                .iter()
                .position(|x| x == s)
                .unwrap_or_else(|| unreachable!("string collected"));
            // Globals of array type decay to ptr in calls; bind the global directly.
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), format!("@.str.{idx}"));
        }
        Inst::Nil => {
            let name = ctx.tmp();
            let _ = writeln!(out, "  {name} = add i64 0, 0");
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::IAdd(a, b) => bin_i(out, ctx, func, result, "add", a, b),
        Inst::ISub(a, b) => bin_i(out, ctx, func, result, "sub", a, b),
        Inst::IMul(a, b) => bin_i(out, ctx, func, result, "mul", a, b),
        Inst::IDiv(a, b) => {
            if let Some(shift) = iconst_of(func, *b).and_then(pow2_shift) {
                emit_signed_sdiv_pow2(out, ctx, result, a, shift);
            } else {
                bin_i(out, ctx, func, result, "sdiv", a, b);
            }
        }
        Inst::UDiv(a, b) => match iconst_of(func, *b).and_then(udiv_const_plan) {
            Some(UdivPlan::Pow2(shift)) => {
                let x = ctx.val(*a);
                let name = ctx.tmp();
                let _ = writeln!(out, "  {name} = lshr i64 {x}, {shift}");
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
            }
            Some(UdivPlan::Mulhu { magic, shift }) => {
                emit_udiv_mulhu(out, ctx, result, a, magic, shift);
            }
            None => bin_i(out, ctx, func, result, "udiv", a, b),
        },
        Inst::IRem(a, b) => {
            if let Some(shift) = iconst_of(func, *b).and_then(pow2_shift) {
                emit_signed_srem_pow2(out, ctx, result, a, shift);
            } else {
                bin_i(out, ctx, func, result, "srem", a, b);
            }
        }
        Inst::URem(a, b) => bin_i(out, ctx, func, result, "urem", a, b),
        Inst::IAnd(a, b) => bin_i(out, ctx, func, result, "and", a, b),
        Inst::IOr(a, b) => bin_i(out, ctx, func, result, "or", a, b),
        Inst::LShr(a, b) => bin_i(out, ctx, func, result, "lshr", a, b),
        Inst::LShl(a, b) => bin_i(out, ctx, func, result, "shl", a, b),
        Inst::INeg(a) => {
            let name = ctx.tmp();
            let _ = writeln!(out, "  {name} = sub i64 0, {}", ctx.val(*a));
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::FAdd(a, b) => bin_f(out, ctx, result, "fadd", a, b),
        Inst::FSub(a, b) => bin_f(out, ctx, result, "fsub", a, b),
        Inst::FMul(a, b) => bin_f(out, ctx, result, "fmul", a, b),
        Inst::FDiv(a, b) => bin_f(out, ctx, result, "fdiv", a, b),
        Inst::FNeg(a) => {
            let name = ctx.tmp();
            let _ = writeln!(out, "  {name} = fneg double {}", ctx.val(*a));
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::ICmp(op, a, b) => {
            let name = ctx.tmp();
            let pred = icmp_pred(*op);
            let ta = func.value_ty(*a);
            let tb = func.value_ty(*b);
            // Prefer i1 only when *both* sides are bool; otherwise i64 (zext bools).
            if ta == IrTy::Bool && tb == IrTy::Bool {
                let _ = writeln!(
                    out,
                    "  {name} = icmp {pred} i1 {}, {}",
                    ctx.val(*a),
                    ctx.val(*b)
                );
            } else {
                let va = if ta == IrTy::Bool {
                    let z = ctx.tmp();
                    let _ = writeln!(out, "  {z} = zext i1 {} to i64", ctx.val(*a));
                    z
                } else {
                    ctx.val(*a)
                };
                let vb = if tb == IrTy::Bool {
                    let z = ctx.tmp();
                    let _ = writeln!(out, "  {z} = zext i1 {} to i64", ctx.val(*b));
                    z
                } else {
                    ctx.val(*b)
                };
                let _ = writeln!(out, "  {name} = icmp {pred} i64 {va}, {vb}");
            }
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::FCmp(op, a, b) => {
            let name = ctx.tmp();
            let pred = fcmp_pred(*op);
            let _ = writeln!(
                out,
                "  {name} = fcmp {pred} double {}, {}",
                ctx.val(*a),
                ctx.val(*b)
            );
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::BNot(a) => {
            let name = ctx.tmp();
            let _ = writeln!(out, "  {name} = xor i1 {}, true", ctx.val(*a));
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::BAnd(a, b) => {
            let name = ctx.tmp();
            let _ = writeln!(
                out,
                "  {name} = and i1 {}, {}",
                ctx.val(*a),
                ctx.val(*b)
            );
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::BOr(a, b) => {
            let name = ctx.tmp();
            let _ = writeln!(
                out,
                "  {name} = or i1 {}, {}",
                ctx.val(*a),
                ctx.val(*b)
            );
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::ZExtI64(a) => {
            let name = ctx.tmp();
            let _ = writeln!(
                out,
                "  {name} = zext i1 {} to i64",
                ctx.val(*a)
            );
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::CtPop(a) => {
            let name = ctx.tmp();
            let _ = writeln!(
                out,
                "  {name} = call i64 @llvm.ctpop.i64(i64 {})",
                ctx.val(*a)
            );
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::Cttz(a) => {
            let name = ctx.tmp();
            let _ = writeln!(
                out,
                "  {name} = call i64 @llvm.cttz.i64(i64 {}, i1 false)",
                ctx.val(*a)
            );
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
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
                    let _ = writeln!(out, "  {name} = alloca {lty}, align 8");
                }
                Placement::Region(rid) => {
                    let _ = writeln!(
                        out,
                        "  {name} = call ptr @rynix_rt_region_alloc(i32 {rid}, i64 {size})"
                    );
                }
                Placement::Heap => {
                    let _ = writeln!(
                        out,
                        "  {name} = call ptr @rynix_rt_heap_alloc(i64 {size})"
                    );
                }
            }
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::Load(p) => {
            let name = ctx.tmp();
            // Payload type from alloc if possible.
            let lty = load_ty(func, *p);
            let result_ty = result.map(|r| func.value(r).ty);
            let _ = writeln!(
                out,
                "  {name} = load {lty}, ptr {}, align 8",
                ctx.val(*p)
            );
            // Struct/array i64 slots may hold pointers (str fields): inttoptr.
            if matches!(result_ty, Some(IrTy::Str) | Some(IrTy::Ptr)) && lty == "i64" {
                let name2 = ctx.tmp();
                let _ = writeln!(out, "  {name2} = inttoptr i64 {name} to ptr");
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name2);
            } else if lty == "i8" {
                let name2 = ctx.tmp();
                let _ = writeln!(out, "  {name2} = trunc i8 {name} to i1");
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name2);
            } else {
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
            }
        }
        Inst::Store { ptr, value } => {
            let lty = load_ty(func, *ptr);
            let mut v = ctx.val(*value);
            let vty = func.value(*value).ty;
            if lty == "i8" {
                let name = ctx.tmp();
                let _ = writeln!(out, "  {name} = zext i1 {v} to i8");
                v = name;
            } else if lty == "i64" && matches!(vty, IrTy::Str | IrTy::Ptr) {
                let name = ctx.tmp();
                let _ = writeln!(out, "  {name} = ptrtoint ptr {v} to i64");
                v = name;
            }
            let _ = writeln!(
                out,
                "  store {lty} {v}, ptr {}, align 8",
                ctx.val(*ptr)
            );
        }
        Inst::GepI64 { base, index } => {
            let name = ctx.tmp();
            let _ = writeln!(
                out,
                "  {name} = getelementptr inbounds i64, ptr {}, i64 {}",
                ctx.val(*base),
                ctx.val(*index)
            );
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::BoundsCheck { index, len } => {
            let ok_lo = ctx.tmp();
            let ok_hi = ctx.tmp();
            let ok = ctx.tmp();
            let cont = format!("bc_ok.{}", ctx.next_tmp);
            let fail = format!("bc_fail.{}", ctx.next_tmp);
            ctx.next_tmp += 1;
            let _ = writeln!(
                out,
                "  {ok_lo} = icmp sge i64 {}, 0",
                ctx.val(*index)
            );
            let _ = writeln!(
                out,
                "  {ok_hi} = icmp slt i64 {}, {}",
                ctx.val(*index),
                ctx.val(*len)
            );
            let _ = writeln!(out, "  {ok} = and i1 {ok_lo}, {ok_hi}");
            let _ = writeln!(out, "  br i1 {ok}, label %{cont}, label %{fail}");
            let _ = writeln!(out, "{fail}:");
            let _ = writeln!(
                out,
                "  call void @rynix_rt_panic(ptr @.rynix.bounds)"
            );
            let _ = writeln!(out, "  unreachable");
            let _ = writeln!(out, "{cont}:");
        }
        Inst::ArrayLen(base) => {
            let name = ctx.tmp();
            let _ = writeln!(
                out,
                "  {name} = load i64, ptr {}, align 8",
                ctx.val(*base)
            );
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::LoadIndex { base, index } => {
            let one = ctx.tmp();
            let off = ctx.tmp();
            let slot = ctx.tmp();
            let name = ctx.tmp();
            let _ = writeln!(out, "  {one} = add i64 0, 1");
            let _ = writeln!(
                out,
                "  {off} = add i64 {}, {one}",
                ctx.val(*index)
            );
            let _ = writeln!(
                out,
                "  {slot} = getelementptr inbounds i64, ptr {}, i64 {off}",
                ctx.val(*base)
            );
            let _ = writeln!(out, "  {name} = load i64, ptr {slot}, align 8");
            ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
        }
        Inst::Free { ptr, .. } => {
            let _ = writeln!(
                out,
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
                    out,
                    "  call void @{cname}({args})",
                    args = arg_s.join(", ")
                );
                if let Some(r) = result {
                    let name = ctx.tmp();
                    let _ = writeln!(out, "  {name} = add i64 0, 0");
                    ctx.bind(r, name);
                }
            } else {
                let name = ctx.tmp();
                let _ = writeln!(
                    out,
                    "  {name} = call {rty} @{cname}({args})",
                    args = arg_s.join(", ")
                );
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
            }
        }
        Inst::CallExt { name, args, ret } => {
            let n = ctx.interner.resolve(*name);
            if n == "print" || n == "println" {
                let arg = if let Some(a) = args.first() {
                    ctx.val(*a)
                } else {
                    let t = ctx.tmp();
                    let _ = writeln!(out, "  {t} = inttoptr i64 0 to ptr");
                    t
                };
                let _ = writeln!(out, "  call void @rynix_rt_print(ptr {arg})");
                if let Some(r) = result {
                    let t = ctx.tmp();
                    let _ = writeln!(out, "  {t} = add i64 0, 0");
                    ctx.bind(r, t);
                }
            } else if n == "rynix_rt_print_i64" || n == "print_i64" {
                let arg = args
                    .first()
                    .map(|a| ctx.val(*a))
                    .unwrap_or_else(|| "0".into());
                let callee = if ctx.wasm_host {
                    "print_i64"
                } else {
                    "rynix_rt_print_i64"
                };
                let _ = writeln!(out, "  call void @{callee}(i64 {arg})");
                if let Some(r) = result {
                    let t = ctx.tmp();
                    let _ = writeln!(out, "  {t} = add i64 0, 0");
                    ctx.bind(r, t);
                }
            } else if n == "rynix_rt_opaque_i64" || n == "opaque_i64" {
                let arg = args
                    .first()
                    .map(|a| ctx.val(*a))
                    .unwrap_or_else(|| "0".into());
                let t = ctx.tmp();
                let _ = writeln!(out, "  {t} = call i64 @rynix_rt_opaque_i64(i64 {arg})");
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), t);
            } else if n == "rynix_rt_heap_alloc" {
                let size = args
                    .first()
                    .map(|a| ctx.val(*a))
                    .unwrap_or_else(|| "0".into());
                let t = ctx.tmp();
                let _ = writeln!(
                    out,
                    "  {t} = call ptr @rynix_rt_heap_alloc(i64 {size})"
                );
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), t);
            } else if n == "rynix_rt_spawn" {
                // Prefer named fiber entry: first arg is often an SConst of the fn name.
                let fn_ptr = args
                    .first()
                    .and_then(|a| {
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
                        // Fallback: value already bound to @.str.K → resolve string table.
                        let bound = ctx.val(*a);
                        if let Some(idx) = bound
                            .strip_prefix("@.str.")
                            .and_then(|s| s.parse::<usize>().ok())
                        {
                            if let Some(name) = ctx.strings.get(idx) {
                                if ctx.module.func_names.iter().any(|&n| {
                                    ctx.interner.resolve(n) == name.as_str()
                                }) {
                                    return Some(format!("@{name}"));
                                }
                            }
                        }
                        None
                    })
                    .unwrap_or_else(|| "null".into());
                let t = ctx.tmp();
                let _ = writeln!(
                    out,
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
                let _ = writeln!(out, "  call void @rynix_rt_sleep_ms(i64 {ms})");
                if let Some(r) = result {
                    let t = ctx.tmp();
                    let _ = writeln!(out, "  {t} = add i64 0, 0");
                    ctx.bind(r, t);
                }
            } else if n == "yield" || n == "rynix_rt_yield" {
                let _ = writeln!(out, "  call void @rynix_rt_yield()");
                if let Some(r) = result {
                    let t = ctx.tmp();
                    let _ = writeln!(out, "  {t} = add i64 0, 0");
                    ctx.bind(r, t);
                }
            } else if n == "now_ms" || n == "rynix_rt_now_ms" {
                let t = ctx.tmp();
                let _ = writeln!(out, "  {t} = call i64 @rynix_rt_now_ms()");
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), t);
            } else if n == "rynix_rt_vec_i64_new"
                || n == "rynix_rt_vec_str_new"
                || n == "rynix_rt_map_i64_new"
                || n == "rynix_rt_map_str_i64_new"
                || n == "rynix_rt_map_str_str_new"
                || n == "rynix_rt_kv_new"
            {
                let rid = args.first().map(|a| ctx.val(*a)).unwrap_or_else(|| "0".into());
                let trunc = ctx.tmp();
                let t = ctx.tmp();
                let _ = writeln!(out, "  {trunc} = trunc i64 {rid} to i32");
                let _ = writeln!(out, "  {t} = call ptr @{n}(i32 {trunc})");
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), t);
            } else if n == "rynix_rt_kv_put" {
                let kv = ctx.val(args[0]);
                let key = ctx.val(args[1]);
                let val = ctx.val(args[2]);
                let _ = writeln!(
                    out,
                    "  call void @rynix_rt_kv_put(ptr {kv}, ptr {key}, i64 {val})"
                );
            } else if n == "rynix_rt_kv_get" {
                let kv = ctx.val(args[0]);
                let key = ctx.val(args[1]);
                let t = ctx.tmp();
                let _ = writeln!(out, "  {t} = call i64 @rynix_rt_kv_get(ptr {kv}, ptr {key})");
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), t);
            } else if n == "rynix_rt_kv_len" {
                let kv = ctx.val(args[0]);
                let t = ctx.tmp();
                let _ = writeln!(out, "  {t} = call i64 @rynix_rt_kv_len(ptr {kv})");
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), t);
            } else if n == "rynix_rt_vec_i64_push" {
                let v = ctx.val(args[0]);
                let x = ctx.val(args[1]);
                let _ = writeln!(out, "  call void @rynix_rt_vec_i64_push(ptr {v}, i64 {x})");
            } else if n == "rynix_rt_vec_str_push" {
                let v = ctx.val(args[0]);
                let x = ctx.val(args[1]);
                let _ = writeln!(out, "  call void @rynix_rt_vec_str_push(ptr {v}, ptr {x})");
            } else if n == "rynix_rt_map_i64_insert" {
                let m = ctx.val(args[0]);
                let k = ctx.val(args[1]);
                let val = ctx.val(args[2]);
                let _ = writeln!(
                    out,
                    "  call void @rynix_rt_map_i64_insert(ptr {m}, i64 {k}, i64 {val})"
                );
            } else if n == "rynix_rt_map_str_i64_insert" {
                let m = ctx.val(args[0]);
                let k = ctx.val(args[1]);
                let val = ctx.val(args[2]);
                let _ = writeln!(
                    out,
                    "  call void @rynix_rt_map_str_i64_insert(ptr {m}, ptr {k}, i64 {val})"
                );
            } else if n == "rynix_rt_map_str_str_insert" {
                let m = ctx.val(args[0]);
                let k = ctx.val(args[1]);
                let val = ctx.val(args[2]);
                let _ = writeln!(
                    out,
                    "  call void @rynix_rt_map_str_str_insert(ptr {m}, ptr {k}, ptr {val})"
                );
            } else if n == "rynix_rt_map_str_i64_get" {
                let m = ctx.val(args[0]);
                let k = ctx.val(args[1]);
                let t = ctx.tmp();
                let _ = writeln!(
                    out,
                    "  {t} = call i64 @rynix_rt_map_str_i64_get(ptr {m}, ptr {k})"
                );
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), t);
            } else if n == "rynix_rt_map_str_str_get" {
                let m = ctx.val(args[0]);
                let k = ctx.val(args[1]);
                let t = ctx.tmp();
                let _ = writeln!(
                    out,
                    "  {t} = call ptr @rynix_rt_map_str_str_get(ptr {m}, ptr {k})"
                );
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), t);
            } else if n == "rynix_rt_map_str_i64_len" {
                let m = ctx.val(args[0]);
                let t = ctx.tmp();
                let _ = writeln!(out, "  {t} = call i64 @rynix_rt_map_str_i64_len(ptr {m})");
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), t);
            } else if n == "rynix_rt_map_str_str_len" {
                let m = ctx.val(args[0]);
                let t = ctx.tmp();
                let _ = writeln!(out, "  {t} = call i64 @rynix_rt_map_str_str_len(ptr {m})");
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), t);
            } else if n == "rynix_rt_vec_str_get" {
                let v = ctx.val(args[0]);
                let i = ctx.val(args[1]);
                let t = ctx.tmp();
                let _ = writeln!(out, "  {t} = call ptr @rynix_rt_vec_str_get(ptr {v}, i64 {i})");
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), t);
            } else if n == "rynix_rt_vec_str_len" {
                let v = ctx.val(args[0]);
                let t = ctx.tmp();
                let _ = writeln!(out, "  {t} = call i64 @rynix_rt_vec_str_len(ptr {v})");
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), t);
            } else if n == "rynix_rt_vec_i64_get"
                || n == "rynix_rt_vec_i64_len"
                || n == "rynix_rt_map_i64_get"
                || n == "rynix_rt_map_i64_len"
                || n == "rynix_rt_tcp_listen"
                || n == "rynix_rt_tcp_accept"
            {
                let arg_s: Vec<_> = args
                    .iter()
                    .enumerate()
                    .map(|(i, a)| {
                        if n.starts_with("rynix_rt_tcp") {
                            format!("i64 {}", ctx.val(*a))
                        } else if i == 0 {
                            format!("ptr {}", ctx.val(*a))
                        } else {
                            format!("i64 {}", ctx.val(*a))
                        }
                    })
                    .collect();
                let t = ctx.tmp();
                let _ = writeln!(
                    out,
                    "  {t} = call i64 @{n}({args})",
                    args = arg_s.join(", ")
                );
                ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), t);
            } else if n == "rynix_rt_tcp_close" {
                let fd = args.first().map(|a| ctx.val(*a)).unwrap_or_else(|| "0".into());
                let _ = writeln!(out, "  call void @rynix_rt_tcp_close(i64 {fd})");
            } else {
                // Unknown external: type args from RIR value types (not all-ptr).
                let rty = llvm_abi_ty(*ret);
                let arg_s: Vec<_> = args
                    .iter()
                    .map(|a| {
                        let ty = func.value_ty(*a);
                        format!("{} {}", llvm_abi_ty(ty), ctx.val(*a))
                    })
                    .collect();
                if *ret == IrTy::Unit {
                    let _ = writeln!(
                        out,
                        "  call void @{n}({args})",
                        args = arg_s.join(", ")
                    );
                } else {
                    let t = ctx.tmp();
                    let _ = writeln!(
                        out,
                        "  {t} = call {rty} @{n}({args})",
                        args = arg_s.join(", ")
                    );
                    ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), t);
                }
            }
        }
        Inst::RegionCreate { region } => {
            let _ = writeln!(
                out,
                "  call void @rynix_rt_region_create(i32 {region})"
            );
        }
        Inst::RegionReset { region } => {
            let _ = writeln!(
                out,
                "  call void @rynix_rt_region_reset(i32 {region})"
            );
        }
        Inst::Ret(None) => {
            if is_main {
                let _ = writeln!(out, "  ret i32 0");
            } else {
                let _ = writeln!(out, "  ret void");
            }
        }
        Inst::Ret(Some(v)) => {
            if is_main {
                // Truncate i64 → i32 exit code when possible.
                let ty = func.value_ty(*v);
                match ty {
                    IrTy::I64 => {
                        let t = ctx.tmp();
                        let _ = writeln!(out, "  {t} = trunc i64 {} to i32", ctx.val(*v));
                        let _ = writeln!(out, "  ret i32 {t}");
                    }
                    IrTy::Bool => {
                        let t = ctx.tmp();
                        let _ = writeln!(out, "  {t} = zext i1 {} to i32", ctx.val(*v));
                        let _ = writeln!(out, "  ret i32 {t}");
                    }
                    _ => {
                        let _ = writeln!(out, "  ret i32 0");
                    }
                }
            } else if func.ret == IrTy::Unit {
                let _ = writeln!(out, "  ret void");
            } else {
                let _ = writeln!(
                    out,
                    "  ret {} {}",
                    llvm_abi_ty(func.ret),
                    ctx.val(*v)
                );
            }
        }
        Inst::Jump { target, .. } => {
            match loop_latch {
                Some(LoopHint::Vectorize) => {
                    let _ = writeln!(out, "  br label %{}, !llvm.loop !0", block_label(*target));
                }
                Some(LoopHint::Unroll) => {
                    let _ = writeln!(out, "  br label %{}, !llvm.loop !1", block_label(*target));
                }
                None => {
                    let _ = writeln!(out, "  br label %{}", block_label(*target));
                }
            }
        }
        Inst::Br {
            cond,
            then_target,
            else_target,
            ..
        } => {
            let _ = writeln!(
                out,
                "  br i1 {}, label %{}, label %{}",
                ctx.val(*cond),
                block_label(*then_target),
                block_label(*else_target)
            );
        }
        Inst::Unreachable => {
            let _ = writeln!(out, "  unreachable");
        }
    }
}

fn bin_i(
    out: &mut String,
    ctx: &mut EmitCtx<'_>,
    func: &rynix_rir::Function,
    result: Option<ValueId>,
    op: &str,
    a: &ValueId,
    b: &ValueId,
) {
    let av = i64_operand(func, ctx, *a);
    let bv = i64_operand(func, ctx, *b);
    let name = ctx.tmp();
    let _ = writeln!(out, "  {name} = {op} i64 {av}, {bv}");
    ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
}

fn i64_operand(func: &rynix_rir::Function, ctx: &EmitCtx<'_>, v: ValueId) -> String {
    iconst_of(func, v)
        .map(|n| n.to_string())
        .unwrap_or_else(|| ctx.val(v))
}

fn iconst_of(func: &rynix_rir::Function, v: ValueId) -> Option<i64> {
    let def = func.values.get(v.0 as usize)?.def?;
    match func.inst(def) {
        Inst::IConst(n) => Some(*n),
        _ => None,
    }
}

fn pow2_shift(n: i64) -> Option<u32> {
    if n > 0 && (n & (n - 1)) == 0 {
        Some(n.trailing_zeros())
    } else {
        None
    }
}

/// Granlund–Montgomery `udiv` by a small positive constant (no IDIV).
enum UdivPlan {
    Pow2(u32),
    Mulhu { magic: u64, shift: u32 },
}

fn udiv_const_plan(d: i64) -> Option<UdivPlan> {
    if let Some(shift) = pow2_shift(d) {
        return Some(UdivPlan::Pow2(shift));
    }
    // 0xAAAAAAAAAAAAAAAB = ceil(2^64 / 3); `lshr` 1 → /3, `lshr` 2 → /6.
    const MAGIC3: u64 = 0xAAAA_AAAA_AAAA_AAAB;
    match d {
        3 => Some(UdivPlan::Mulhu {
            magic: MAGIC3,
            shift: 1,
        }),
        6 => Some(UdivPlan::Mulhu {
            magic: MAGIC3,
            shift: 2,
        }),
        _ => None,
    }
}

/// `udiv n, d` → `(mulhu n, magic) >> shift` via i128 (clang `-c` of textual `.ll`
/// often leaves a raw `udiv`/`sdiv` as IDIV).
fn emit_udiv_mulhu(
    out: &mut String,
    ctx: &mut EmitCtx<'_>,
    result: Option<ValueId>,
    a: &ValueId,
    magic: u64,
    shift: u32,
) {
    let x = ctx.val(*a);
    let wide = ctx.tmp();
    let mag = ctx.tmp();
    let prod = ctx.tmp();
    let hi128 = ctx.tmp();
    let hi = ctx.tmp();
    let _ = writeln!(out, "  {wide} = zext i64 {x} to i128");
    let _ = writeln!(out, "  {mag} = zext i64 {magic} to i128");
    let _ = writeln!(out, "  {prod} = mul i128 {wide}, {mag}");
    let _ = writeln!(out, "  {hi128} = lshr i128 {prod}, 64");
    let _ = writeln!(out, "  {hi} = trunc i128 {hi128} to i64");
    if shift == 0 {
        ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), hi);
        return;
    }
    let q = ctx.tmp();
    let _ = writeln!(out, "  {q} = lshr i64 {hi}, {shift}");
    ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), q);
}

/// Signed `sdiv` by `2^shift` without a variable divisor (LLVM-friendly, trunc toward zero).
fn emit_signed_sdiv_pow2(
    out: &mut String,
    ctx: &mut EmitCtx<'_>,
    result: Option<ValueId>,
    a: &ValueId,
    shift: u32,
) {
    let x = ctx.val(*a);
    let sign = ctx.tmp();
    let _ = writeln!(out, "  {sign} = ashr i64 {x}, 63");
    let adj = ctx.tmp();
    let mask = (1i64 << shift) - 1;
    let _ = writeln!(out, "  {adj} = and i64 {sign}, {mask}");
    let t = ctx.tmp();
    let _ = writeln!(out, "  {t} = add i64 {x}, {adj}");
    let q = ctx.tmp();
    let _ = writeln!(out, "  {q} = ashr i64 {t}, {shift}");
    ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), q);
}

/// Signed `srem` by `2^shift` via quotient reconstruction (matches Rynix `/` + `%` semantics).
fn emit_signed_srem_pow2(
    out: &mut String,
    ctx: &mut EmitCtx<'_>,
    result: Option<ValueId>,
    a: &ValueId,
    shift: u32,
) {
    let x = ctx.val(*a);
    let sign = ctx.tmp();
    let _ = writeln!(out, "  {sign} = ashr i64 {x}, 63");
    let adj = ctx.tmp();
    let mask = (1i64 << shift) - 1;
    let _ = writeln!(out, "  {adj} = and i64 {sign}, {mask}");
    let t = ctx.tmp();
    let _ = writeln!(out, "  {t} = add i64 {x}, {adj}");
    let q = ctx.tmp();
    let _ = writeln!(out, "  {q} = ashr i64 {t}, {shift}");
    let divisor = 1i64 << shift;
    let prod = ctx.tmp();
    let _ = writeln!(out, "  {prod} = mul i64 {q}, {divisor}");
    let r = ctx.tmp();
    let _ = writeln!(out, "  {r} = sub i64 {x}, {prod}");
    ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), r);
}

fn bin_f(
    out: &mut String,
    ctx: &mut EmitCtx<'_>,
    result: Option<ValueId>,
    op: &str,
    a: &ValueId,
    b: &ValueId,
) {
    let av = ctx.val(*a);
    let bv = ctx.val(*b);
    let name = ctx.tmp();
    let _ = writeln!(out, "  {name} = {op} double {av}, {bv}");
    ctx.bind(result.unwrap_or_else(|| unreachable!("invariant")), name);
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
