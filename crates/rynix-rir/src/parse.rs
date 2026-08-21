//! Minimal textual `.rir` parser (subset of the printer output).
//!
//! Enough for FileCheck-style pass tests: functions, blocks, constants,
//! arithmetic, alloc/load/store, call, ret, jump, br.

#![allow(clippy::too_many_lines)]
#![allow(clippy::similar_names)]
#![allow(clippy::many_single_char_names)]

use rynix_span::{Interner, Symbol};
use rustc_hash::FxHashMap;

use crate::builder::FunctionBuilder;
use crate::ir::{BlockId, CmpOp, FuncId, Inst, IrTy, Module, ValueId};

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
}

/// Parse a textual RIR module. Interns function and string names into `interner`.
pub fn parse_module(text: &str, interner: &mut Interner) -> Result<Module, ParseError> {
    let mut module = Module::new();
    let mut lines = text.lines().enumerate().peekable();

    // First pass: collect function headers for call resolution.
    let mut headers: Vec<(Symbol, IrTy, Vec<IrTy>, usize)> = Vec::new();
    {
        for (li, line) in text.lines().enumerate() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("func @") {
                let (name, params, ret) = parse_func_header(rest, li + 1)?;
                let name_sym = interner.intern(name);
                headers.push((name_sym, ret, params, li + 1));
            }
        }
    }

    for (name, _, _, _) in &headers {
        let id = FuncId(module.funcs.len() as u32);
        module
            .funcs
            .push(FunctionBuilder::new(*name, IrTy::Unit).finish());
        module.func_names.push(*name);
        let _ = id;
    }

    // Second pass: parse bodies.
    let mut func_idx = 0usize;
    while let Some((li, line)) = lines.next() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with("func @") {
            return Err(ParseError {
                message: format!("expected func, got `{line}`"),
                line: li + 1,
            });
        }
        let (name_sym, ret, param_tys, _) = &headers[func_idx];
        let mut b = FunctionBuilder::new(*name_sym, *ret);
        let mut vmap: FxHashMap<u32, ValueId> = FxHashMap::default();

        // Consume params from entry (builder already created entry params if we add_param).
        // Rebuild: FunctionBuilder::new has empty params; add them.
        // Note: create_block for subsequent blocks.
        for ty in param_tys {
            let v = b.add_param(*ty);
            // The printed form uses %N for params — we don't know N yet.
            // We'll remap when we see block0 params or first use.
            let _ = v;
        }

        // Parse blocks until next func or EOF.
        let mut current_block: Option<BlockId> = Some(b.func.entry);
        let mut block_map: FxHashMap<u32, BlockId> = FxHashMap::default();
        block_map.insert(0, b.func.entry);
        b.seal_block(b.func.entry);

        while let Some(&(li2, raw)) = lines.peek() {
            let trimmed = raw.trim();
            if trimmed.starts_with("func @") {
                break;
            }
            let _ = lines.next();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("block") {
                let (num, params) = parse_block_header(rest, li2 + 1)?;
                let bid = if num == 0 {
                    b.func.entry
                } else {
                    *block_map.entry(num).or_insert_with(|| b.create_block())
                };
                // For block0, params already from add_param — map printed ids.
                if num == 0 {
                    for (i, (pid, _)) in params.iter().enumerate() {
                        if let Some(&(v, _)) = b.func.params.get(i) {
                            vmap.insert(*pid, v);
                        }
                    }
                } else {
                    for (pid, ty) in &params {
                        let v = b.append_block_param(bid, *ty);
                        vmap.insert(*pid, v);
                    }
                    b.seal_block(bid);
                }
                b.switch_to(bid);
                current_block = Some(bid);
                continue;
            }

            if current_block.is_none() {
                return Err(ParseError {
                    message: "instruction outside block".into(),
                    line: li2 + 1,
                });
            }

            parse_inst(
                trimmed,
                li2 + 1,
                &mut b,
                &mut vmap,
                &mut block_map,
                &module,
                interner,
            )?;
        }

        // Ensure all referenced blocks exist.
        let _ = current_block;
        module.funcs[func_idx] = b.finish();
        func_idx += 1;
    }

    Ok(module)
}

fn parse_func_header(rest: &str, line: usize) -> Result<(&str, Vec<IrTy>, IrTy), ParseError> {
    // name(params) -> ret:
    let rest = rest.strip_suffix(':').unwrap_or(rest);
    let (name, after_name) = rest
        .split_once('(')
        .ok_or_else(|| ParseError {
            message: "bad func header".into(),
            line,
        })?;
    let (params_s, after_params) = after_name.split_once(')').ok_or_else(|| ParseError {
        message: "bad func params".into(),
        line,
    })?;
    let after_params = after_params.trim();
    let ret_s = after_params
        .strip_prefix("->")
        .ok_or_else(|| ParseError {
            message: "missing return type".into(),
            line,
        })?
        .trim();
    let mut params = Vec::new();
    if !params_s.trim().is_empty() {
        for p in params_s.split(',') {
            let p = p.trim();
            let ty = p
                .rsplit_once(':')
                .map(|(_, t)| parse_ty(t.trim(), line))
                .transpose()?
                .unwrap_or(IrTy::I64);
            params.push(ty);
        }
    }
    Ok((name.trim(), params, parse_ty(ret_s, line)?))
}

fn parse_block_header(rest: &str, line: usize) -> Result<(u32, Vec<(u32, IrTy)>), ParseError> {
    // 0: or 1(%2:i64):
    let rest = rest.strip_suffix(':').unwrap_or(rest);
    if let Some((num_s, params_s)) = rest.split_once('(') {
        let num: u32 = num_s.parse().map_err(|_| ParseError {
            message: "bad block number".into(),
            line,
        })?;
        let params_s = params_s.strip_suffix(')').unwrap_or(params_s);
        let mut params = Vec::new();
        if !params_s.trim().is_empty() {
            for p in params_s.split(',') {
                let p = p.trim();
                let (vid, ty) = p.split_once(':').ok_or_else(|| ParseError {
                    message: "bad block param".into(),
                    line,
                })?;
                let vid = vid.trim().trim_start_matches('%').parse().map_err(|_| {
                    ParseError {
                        message: "bad value id".into(),
                        line,
                    }
                })?;
                params.push((vid, parse_ty(ty.trim(), line)?));
            }
        }
        Ok((num, params))
    } else {
        let num: u32 = rest.parse().map_err(|_| ParseError {
            message: "bad block number".into(),
            line,
        })?;
        Ok((num, Vec::new()))
    }
}

fn parse_ty(s: &str, line: usize) -> Result<IrTy, ParseError> {
    match s {
        "unit" => Ok(IrTy::Unit),
        "bool" => Ok(IrTy::Bool),
        "i64" => Ok(IrTy::I64),
        "f64" => Ok(IrTy::F64),
        "str" => Ok(IrTy::Str),
        "ptr" => Ok(IrTy::Ptr),
        other => Err(ParseError {
            message: format!("unknown type `{other}`"),
            line,
        }),
    }
}

fn parse_inst(
    line: &str,
    lineno: usize,
    b: &mut FunctionBuilder,
    vmap: &mut FxHashMap<u32, ValueId>,
    block_map: &mut FxHashMap<u32, BlockId>,
    module: &Module,
    interner: &mut Interner,
) -> Result<(), ParseError> {
    let (lhs, rhs) = if let Some((l, r)) = line.split_once('=') {
        (Some(l.trim()), r.trim())
    } else {
        (None, line)
    };

    let map_v = |s: &str, vmap: &FxHashMap<u32, ValueId>| -> Result<ValueId, ParseError> {
        let id: u32 = s
            .trim()
            .trim_start_matches('%')
            .parse()
            .map_err(|_| ParseError {
                message: format!("bad value `{s}`"),
                line: lineno,
            })?;
        vmap.get(&id).copied().ok_or_else(|| ParseError {
            message: format!("unknown value %{id}"),
            line: lineno,
        })
    };

    let ensure_block = |num: u32, b: &mut FunctionBuilder, block_map: &mut FxHashMap<u32, BlockId>| {
        *block_map.entry(num).or_insert_with(|| b.create_block())
    };

    let bind = |lhs: Option<&str>, v: ValueId, vmap: &mut FxHashMap<u32, ValueId>| {
        if let Some(l) = lhs {
            let id: u32 = l.trim().trim_start_matches('%').parse().unwrap_or(0);
            vmap.insert(id, v);
        }
    };

    let parts: Vec<&str> = rhs.split_whitespace().collect();
    let op = *parts.first().ok_or_else(|| ParseError {
        message: "empty instruction".into(),
        line: lineno,
    })?;

    match op {
        "iconst" => {
            let n: i64 = parts.get(1).and_then(|s| s.parse().ok()).ok_or_else(|| {
                ParseError {
                    message: "bad iconst".into(),
                    line: lineno,
                }
            })?;
            bind(lhs, b.iconst(n), vmap);
        }
        "bconst" => {
            let n: bool = parts.get(1).and_then(|s| s.parse().ok()).ok_or_else(|| {
                ParseError {
                    message: "bad bconst".into(),
                    line: lineno,
                }
            })?;
            bind(lhs, b.bconst(n), vmap);
        }
        "iadd" | "isub" | "imul" | "idiv" | "irem" => {
            let a = map_v(parts.get(1).unwrap_or(&"").trim_end_matches(','), vmap)?;
            let bv = map_v(parts.get(2).unwrap_or(&""), vmap)?;
            let v = match op {
                "iadd" => b.push_value(Inst::IAdd(a, bv)),
                "isub" => b.push_value(Inst::ISub(a, bv)),
                "imul" => b.push_value(Inst::IMul(a, bv)),
                "idiv" => b.push_value(Inst::IDiv(a, bv)),
                _ => b.push_value(Inst::IRem(a, bv)),
            };
            bind(lhs, v, vmap);
        }
        "icmp" => {
            let cmp = parse_cmp(parts.get(1).unwrap_or(&""), lineno)?;
            let a = map_v(parts.get(2).unwrap_or(&"").trim_end_matches(','), vmap)?;
            let bv = map_v(parts.get(3).unwrap_or(&""), vmap)?;
            bind(lhs, b.push_value(Inst::ICmp(cmp, a, bv)), vmap);
        }
        "alloc" => {
            // alloc siteN ty
            let ty = parse_ty(parts.get(2).unwrap_or(&"i64"), lineno)?;
            bind(lhs, b.alloc(ty), vmap);
        }
        "load" => {
            let p = map_v(parts.get(1).unwrap_or(&""), vmap)?;
            bind(lhs, b.load(p), vmap);
        }
        "store" => {
            let p = map_v(parts.get(1).unwrap_or(&"").trim_end_matches(','), vmap)?;
            let val = map_v(parts.get(2).unwrap_or(&""), vmap)?;
            b.store(p, val);
        }
        "ret" => {
            if parts.len() == 1 {
                b.ret(None);
            } else {
                b.ret(Some(map_v(parts[1], vmap)?));
            }
        }
        "jump" => {
            let target = parse_jump_target(parts.get(1).unwrap_or(&""), lineno)?;
            let bid = ensure_block(target, b, block_map);
            b.jump(bid, vec![]);
        }
        "br" => {
            // br %c blockT() blockE()
            let cond = map_v(parts.get(1).unwrap_or(&""), vmap)?;
            let t = parse_jump_target(parts.get(2).unwrap_or(&""), lineno)?;
            let e = parse_jump_target(parts.get(3).unwrap_or(&""), lineno)?;
            let tb = ensure_block(t, b, block_map);
            let eb = ensure_block(e, b, block_map);
            b.br(cond, tb, vec![], eb, vec![]);
        }
        "call" => {
            let name = parts
                .get(1)
                .and_then(|s| s.strip_prefix('@'))
                .map(|s| s.split('(').next().unwrap_or(s))
                .ok_or_else(|| ParseError {
                    message: "bad call".into(),
                    line: lineno,
                })?;
            let name_sym = interner.intern(name);
            let fid = module.find_func(name_sym).ok_or_else(|| ParseError {
                message: format!("unknown func @{name}"),
                line: lineno,
            })?;
            // Args inside (...): crude parse from rhs
            let args_s = rhs
                .split_once('(')
                .and_then(|(_, r)| r.strip_suffix(')'))
                .unwrap_or("");
            let mut args = Vec::new();
            if !args_s.trim().is_empty() {
                for a in args_s.split(',') {
                    args.push(map_v(a.trim(), vmap)?);
                }
            }
            let ret = module.func(fid).ret;
            bind(lhs, b.call(fid, args, ret), vmap);
        }
        "unreachable" => {
            let _ = b.push(Inst::Unreachable);
        }
        other => {
            return Err(ParseError {
                message: format!("unsupported op `{other}`"),
                line: lineno,
            });
        }
    }
    Ok(())
}

fn parse_cmp(s: &str, line: usize) -> Result<CmpOp, ParseError> {
    match s {
        "eq" => Ok(CmpOp::Eq),
        "ne" => Ok(CmpOp::Ne),
        "lt" => Ok(CmpOp::Lt),
        "le" => Ok(CmpOp::Le),
        "gt" => Ok(CmpOp::Gt),
        "ge" => Ok(CmpOp::Ge),
        other => Err(ParseError {
            message: format!("bad cmp `{other}`"),
            line,
        }),
    }
}

fn parse_jump_target(s: &str, line: usize) -> Result<u32, ParseError> {
    let s = s.split('(').next().unwrap_or(s);
    let s = s.strip_prefix("block").ok_or_else(|| ParseError {
        message: format!("expected blockN, got `{s}`"),
        line,
    })?;
    s.parse().map_err(|_| ParseError {
        message: "bad block id".into(),
        line,
    })
}
