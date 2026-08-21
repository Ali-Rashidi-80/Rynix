//! Intraprocedural points-to + interprocedural SCC escape analysis.

#![allow(clippy::too_many_lines)]
#![allow(clippy::similar_names)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::items_after_statements)]

use rustc_hash::{FxHashMap, FxHashSet};

use rynix_span::{Interner, Span, Symbol};

use crate::ir::{AllocSite, FuncId, Inst, IrTy, Module, ValueId};

use super::lattice::{Escape, Placement};

/// Per-site result after whole-module analysis.
#[derive(Clone, Debug)]
pub struct SiteInfo {
    pub func: FuncId,
    pub site: AllocSite,
    pub span: Span,
    pub escape: Escape,
    pub placement: Placement,
    /// Human reason for the final escape class (for `--explain-alloc`).
    pub reason: String,
    /// If heap: last-use instruction index within the function (for free injection).
    pub free_at: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct EscapeReport {
    pub sites: Vec<SiteInfo>,
}

impl EscapeReport {
    pub fn sites_for_func(&self, func: FuncId) -> impl Iterator<Item = &SiteInfo> {
        self.sites.iter().filter(move |s| s.func == func)
    }
}

/// Analyze the whole module. Does not mutate IR; call [`super::inject_regions`] after.
pub fn analyze_escape(module: &Module, interner: &Interner) -> EscapeReport {
    let n = module.funcs.len();
    let mut summaries: Vec<FuncSummary> = (0..n).map(|_| FuncSummary::default()).collect();

    // Seed: intraprocedural pass without callee info (CallExt conservative).
    for fid in 0..n {
        summaries[fid] = analyze_func(module, FuncId(fid as u32), interner, &summaries);
    }

    // Call graph + SCC bottom-up fixpoint.
    let graph = call_graph(module);
    let sccs = tarjan_sccs(n, &graph);
    let mut changed = true;
    let mut rounds = 0;
    while changed && rounds < 64 {
        changed = false;
        rounds += 1;
        for scc in &sccs {
            // Fixpoint within SCC.
            let mut local = true;
            let mut inner = 0;
            while local && inner < 32 {
                local = false;
                inner += 1;
                for &fid in scc {
                    let next = analyze_func(module, FuncId(fid as u32), interner, &summaries);
                    if next != summaries[fid] {
                        summaries[fid] = next;
                        local = true;
                        changed = true;
                    }
                }
            }
        }
    }

    // Materialize site infos with placements.
    let mut sites = Vec::new();
    for (fi, summary) in summaries.iter().enumerate() {
        let region_id = 0u32; // one region per function in v0
        for (site_idx, (escape, reason, span, free_at)) in summary.sites.iter().enumerate() {
            let escape = *escape;
            sites.push(SiteInfo {
                func: FuncId(fi as u32),
                site: AllocSite(site_idx as u32),
                span: *span,
                escape,
                placement: Placement::from_escape(escape, region_id),
                reason: reason.clone(),
                free_at: *free_at,
            });
        }
    }

    EscapeReport { sites }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct FuncSummary {
    /// Escape of each formal parameter (if a pointer into it escapes).
    param_escape: Vec<Escape>,
    /// Per local alloc site: (`escape`, reason, span, `free_at` inst index).
    sites: Vec<(Escape, String, Span, Option<u32>)>,
}

fn call_graph(module: &Module) -> Vec<Vec<usize>> {
    let n = module.funcs.len();
    let mut g = vec![Vec::new(); n];
    for (fi, func) in module.funcs.iter().enumerate() {
        for inst in &func.insts {
            if let Inst::Call { func: callee, .. } = inst {
                let c = callee.0 as usize;
                if c < n && !g[fi].contains(&c) {
                    g[fi].push(c);
                }
            }
        }
    }
    g
}

/// Tarjan SCCs; returned in reverse topological order (callees before callers).
fn tarjan_sccs(n: usize, graph: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut index = 0usize;
    let mut stack = Vec::new();
    let mut on_stack = vec![false; n];
    let mut indices = vec![None; n];
    let mut lowlink = vec![0usize; n];
    let mut sccs = Vec::new();

    fn strongconnect(
        v: usize,
        graph: &[Vec<usize>],
        index: &mut usize,
        stack: &mut Vec<usize>,
        on_stack: &mut [bool],
        indices: &mut [Option<usize>],
        lowlink: &mut [usize],
        sccs: &mut Vec<Vec<usize>>,
    ) {
        indices[v] = Some(*index);
        lowlink[v] = *index;
        *index += 1;
        stack.push(v);
        on_stack[v] = true;

        for &w in &graph[v] {
            if indices[w].is_none() {
                strongconnect(w, graph, index, stack, on_stack, indices, lowlink, sccs);
                lowlink[v] = lowlink[v].min(lowlink[w]);
            } else if on_stack[w] {
                lowlink[v] = lowlink[v].min(indices[w].unwrap());
            }
        }

        if lowlink[v] == indices[v].unwrap() {
            let mut scc = Vec::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack[w] = false;
                scc.push(w);
                if w == v {
                    break;
                }
            }
            sccs.push(scc);
        }
    }

    for v in 0..n {
        if indices[v].is_none() {
            strongconnect(
                v,
                graph,
                &mut index,
                &mut stack,
                &mut on_stack,
                &mut indices,
                &mut lowlink,
                &mut sccs,
            );
        }
    }
    // Tarjan emits SCCs in reverse topo already for condensation.
    sccs
}

fn analyze_func(
    module: &Module,
    fid: FuncId,
    interner: &Interner,
    summaries: &[FuncSummary],
) -> FuncSummary {
    let func = module.func(fid);
    let mut points: FxHashMap<ValueId, FxHashSet<AllocSite>> = FxHashMap::default();
    let mut contents: FxHashMap<AllocSite, FxHashSet<AllocSite>> = FxHashMap::default();
    let mut site_escape: FxHashMap<AllocSite, Escape> = FxHashMap::default();
    let mut site_reason: FxHashMap<AllocSite, String> = FxHashMap::default();
    let mut site_span: FxHashMap<AllocSite, Span> = FxHashMap::default();
    let mut site_last_use: FxHashMap<AllocSite, u32> = FxHashMap::default();
    let mut param_of: FxHashMap<ValueId, usize> = FxHashMap::default();
    let mut param_escape: Vec<Escape> = vec![Escape::NoEscape; func.params.len()];

    // Params: treat as pointing to synthetic "param sites" only for tracking returns.
    // Real escape of params is recorded in param_escape.
    for (i, (vid, ty)) in func.params.iter().enumerate() {
        param_of.insert(*vid, i);
        if *ty == IrTy::Ptr || *ty == IrTy::Str {
            // Placeholder — param values themselves.
        }
    }

    // Collect allocs.
    let mut max_site = 0u32;
    for (ii, inst) in func.insts.iter().enumerate() {
        if let Inst::Alloc { site, span, .. } = inst {
            max_site = max_site.max(site.0 + 1);
            site_escape.insert(*site, Escape::NoEscape);
            site_reason.insert(*site, "local only".into());
            site_span.insert(*site, *span);
            contents.entry(*site).or_default();
            if let Some(vid) = result_of(func, ii) {
                points.entry(vid).or_default().insert(*site);
            }
        }
    }

    // Propagate points-to to a fixpoint (block args + load/store).
    let mut changed = true;
    let mut rounds = 0;
    while changed && rounds < 64 {
        changed = false;
        rounds += 1;
        for block in &func.blocks {
            for &iid in &block.insts {
                let ii = iid.0 as usize;
                let inst = func.inst(iid);
                match inst {
                    Inst::Load(p) => {
                        if let Some(res) = result_of(func, ii) {
                            let mut add = FxHashSet::default();
                            if let Some(ps) = points.get(p) {
                                for site in ps {
                                    if let Some(c) = contents.get(site) {
                                        add.extend(c.iter().copied());
                                    }
                                }
                            }
                            changed |= union_into(&mut points, res, &add);
                        }
                    }
                    Inst::Store { ptr, value } => {
                        let vs = points.get(value).cloned().unwrap_or_default();
                        if let Some(ps) = points.get(ptr).cloned() {
                            for site in ps {
                                let entry = contents.entry(site).or_default();
                                for v in &vs {
                                    if entry.insert(*v) {
                                        changed = true;
                                    }
                                }
                            }
                        }
                        touch_uses(&mut site_last_use, &points, &[*ptr, *value], ii as u32);
                    }
                    Inst::Jump { args, target } => {
                        let params = &func.block(*target).params;
                        for (i, arg) in args.iter().enumerate() {
                            if let Some((pid, _)) = params.get(i)
                                && let Some(src) = points.get(arg).cloned()
                            {
                                changed |= union_into(&mut points, *pid, &src);
                            }
                        }
                    }
                    Inst::Br {
                        then_target,
                        then_args,
                        else_target,
                        else_args,
                        ..
                    } => {
                        for (target, args) in [
                            (*then_target, then_args.as_slice()),
                            (*else_target, else_args.as_slice()),
                        ] {
                            let params = &func.block(target).params;
                            for (i, arg) in args.iter().enumerate() {
                                if let Some((pid, _)) = params.get(i)
                                    && let Some(src) = points.get(arg).cloned()
                                {
                                    changed |= union_into(&mut points, *pid, &src);
                                }
                            }
                        }
                    }
                    Inst::Call { args, .. } | Inst::CallExt { args, .. } => {
                        touch_uses(&mut site_last_use, &points, args, ii as u32);
                    }
                    Inst::Ret(Some(v)) => {
                        touch_uses(&mut site_last_use, &points, &[*v], ii as u32);
                    }
                    _ => {}
                }
            }
        }
    }

    // Escape rules.
    let escalate = |site: AllocSite,
                    esc: Escape,
                    why: &str,
                    site_escape: &mut FxHashMap<AllocSite, Escape>,
                    site_reason: &mut FxHashMap<AllocSite, String>| {
        let cur = site_escape.entry(site).or_insert(Escape::NoEscape);
        if esc > *cur {
            *cur = esc;
            site_reason.insert(site, why.to_string());
        }
    };

    for (ii, inst) in func.insts.iter().enumerate() {
        match inst {
            Inst::Ret(Some(v)) => {
                if let Some(sites) = points.get(v) {
                    for site in sites {
                        escalate(
                            *site,
                            Escape::ArgEscape,
                            "returned to caller",
                            &mut site_escape,
                            &mut site_reason,
                        );
                    }
                }
                if let Some(&pi) = param_of.get(v) {
                    param_escape[pi] = param_escape[pi].join(Escape::ArgEscape);
                }
                // Returning something loaded from a param slot → param escapes.
            }
            Inst::CallExt { name, args, .. } => {
                let n = interner.resolve(*name);
                let esc = if is_benign_ext(n) {
                    Escape::NoEscape
                } else {
                    Escape::GlobalEscape
                };
                if esc != Escape::NoEscape {
                    for arg in args {
                        escalate_value(
                            *arg,
                            esc,
                            &format!("escaped via call_ext @{n}"),
                            &points,
                            &param_of,
                            &mut site_escape,
                            &mut site_reason,
                            &mut param_escape,
                        );
                    }
                }
                touch_uses(&mut site_last_use, &points, args, ii as u32);
            }
            Inst::Call { func: callee, args } => {
                let summary = summaries.get(callee.0 as usize);
                for (i, arg) in args.iter().enumerate() {
                    let esc = summary
                        .and_then(|s| s.param_escape.get(i).copied())
                        .unwrap_or(Escape::NoEscape);
                    // Even without summary escalation, passing a pointer is at least ArgEscape
                    // if the callee might store it — use summary; default ArgEscape for ptr args
                    // when callee has no info yet is too aggressive. Prefer summary only.
                    let sites = points.get(arg).cloned().unwrap_or_default();
                    if !sites.is_empty() {
                        let esc = if esc == Escape::NoEscape {
                            // Pointer arg with unknown effect: assume ArgEscape (may live in callee frame/region).
                            Escape::ArgEscape
                        } else {
                            esc
                        };
                        for site in sites {
                            escalate(
                                site,
                                esc,
                                &format!("passed to @{}", resolve_func(module, interner, *callee)),
                                &mut site_escape,
                                &mut site_reason,
                            );
                        }
                    }
                    if let Some(&pi) = param_of.get(arg) {
                        let esc = if esc == Escape::NoEscape {
                            Escape::ArgEscape
                        } else {
                            esc
                        };
                        param_escape[pi] = param_escape[pi].join(esc);
                    }
                }
                touch_uses(&mut site_last_use, &points, args, ii as u32);
            }
            Inst::Store { ptr, value } => {
                // Storing a pointer into another allocation → at least RegionEscape.
                let vs = points.get(value).cloned().unwrap_or_default();
                if !vs.is_empty() {
                    for site in vs {
                        escalate(
                            site,
                            Escape::RegionEscape,
                            "stored into another allocation",
                            &mut site_escape,
                            &mut site_reason,
                        );
                    }
                }
                // Store into a param-rooted pointer → ArgEscape for the value's sites.
                if param_of.contains_key(ptr) {
                    for site in points.get(value).into_iter().flatten() {
                        escalate(
                            *site,
                            Escape::ArgEscape,
                            "stored into parameter",
                            &mut site_escape,
                            &mut site_reason,
                        );
                    }
                }
                touch_uses(&mut site_last_use, &points, &[*ptr, *value], ii as u32);
            }
            Inst::Load(p) | Inst::INeg(p) | Inst::FNeg(p) | Inst::BNot(p) => {
                touch_uses(&mut site_last_use, &points, &[*p], ii as u32);
            }
            Inst::IAdd(a, b)
            | Inst::ISub(a, b)
            | Inst::IMul(a, b)
            | Inst::IDiv(a, b)
            | Inst::IRem(a, b)
            | Inst::FAdd(a, b)
            | Inst::FSub(a, b)
            | Inst::FMul(a, b)
            | Inst::FDiv(a, b)
            | Inst::ICmp(_, a, b)
            | Inst::FCmp(_, a, b) => {
                touch_uses(&mut site_last_use, &points, &[*a, *b], ii as u32);
            }
            _ => {}
        }
    }

    // Mark last use for alloc pointer itself.
    for (ii, inst) in func.insts.iter().enumerate() {
        if let Inst::Alloc { site, .. } = inst {
            site_last_use.entry(*site).or_insert(ii as u32);
        }
    }

    let mut sites = Vec::new();
    for idx in 0..max_site {
        let site = AllocSite(idx);
        let escape = site_escape.get(&site).copied().unwrap_or(Escape::NoEscape);
        let reason = site_reason
            .get(&site)
            .cloned()
            .unwrap_or_else(|| "local only".into());
        let span = site_span.get(&site).copied().unwrap_or_else(|| Span::empty(0));
        let free_at = if escape == Escape::GlobalEscape {
            site_last_use.get(&site).copied()
        } else {
            None
        };
        sites.push((escape, reason, span, free_at));
    }

    FuncSummary {
        param_escape,
        sites,
    }
}

fn escalate_value(
    val: ValueId,
    esc: Escape,
    why: &str,
    points: &FxHashMap<ValueId, FxHashSet<AllocSite>>,
    param_of: &FxHashMap<ValueId, usize>,
    site_escape: &mut FxHashMap<AllocSite, Escape>,
    site_reason: &mut FxHashMap<AllocSite, String>,
    param_escape: &mut [Escape],
) {
    if let Some(sites) = points.get(&val) {
        for site in sites {
            let cur = site_escape.entry(*site).or_insert(Escape::NoEscape);
            if esc > *cur {
                *cur = esc;
                site_reason.insert(*site, why.to_string());
            }
        }
    }
    if let Some(&pi) = param_of.get(&val) {
        param_escape[pi] = param_escape[pi].join(esc);
    }
}

fn touch_uses(
    last: &mut FxHashMap<AllocSite, u32>,
    points: &FxHashMap<ValueId, FxHashSet<AllocSite>>,
    vals: &[ValueId],
    inst: u32,
) {
    for v in vals {
        if let Some(sites) = points.get(v) {
            for site in sites {
                let e = last.entry(*site).or_insert(inst);
                if inst >= *e {
                    *e = inst;
                }
            }
        }
    }
}

fn union_into(
    points: &mut FxHashMap<ValueId, FxHashSet<AllocSite>>,
    vid: ValueId,
    add: &FxHashSet<AllocSite>,
) -> bool {
    if add.is_empty() {
        return false;
    }
    let entry = points.entry(vid).or_default();
    let mut changed = false;
    for s in add {
        if entry.insert(*s) {
            changed = true;
        }
    }
    changed
}

fn result_of(func: &crate::ir::Function, inst_index: usize) -> Option<ValueId> {
    let iid = crate::ir::InstId(inst_index as u32);
    func.values
        .iter()
        .enumerate()
        .find(|(_, v)| v.def == Some(iid))
        .map(|(i, _)| ValueId(i as u32))
}

fn is_benign_ext(name: &str) -> bool {
    matches!(name, "print" | "println" | "assert")
}

fn resolve_func(module: &Module, interner: &Interner, fid: FuncId) -> String {
    module
        .funcs
        .get(fid.0 as usize)
        .map_or_else(|| format!("fn{}", fid.0), |f| interner.resolve(f.name).to_string())
}

// Silence unused Symbol import if any.
#[allow(dead_code)]
fn _sym(_: Symbol) {}
