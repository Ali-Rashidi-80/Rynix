//! Apply compiler-suggested fixes to source text.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_sema::analyze;
use rynix_span::Interner;

/// Apply the first diagnostic fix with the highest confidence, if any.
pub fn apply_first_fix(source: &str) -> String {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = rynix_parser::parse(&arena, &mut interner, source, 0, &mut sink);
    let _ = analyze(module, &mut interner, &mut sink);
    let mut best: Option<&rynix_diag::Fix> = None;
    for d in &sink.diags {
        for fix in &d.fixes {
            if best.is_none_or(|b| fix.confidence > b.confidence) {
                best = Some(fix);
            }
        }
    }
    let Some(fix) = best else {
        return source.to_string();
    };
    let mut bytes = source.as_bytes().to_vec();
    let mut edits = fix.edits.clone();
    edits.sort_by_key(|e| std::cmp::Reverse(e.span.lo()));
    for edit in edits {
        let lo = edit.span.lo() as usize;
        let hi = edit.span.hi() as usize;
        if hi > bytes.len() || lo > hi {
            continue;
        }
        bytes.splice(lo..hi, edit.replacement.bytes());
    }
    String::from_utf8(bytes).unwrap_or_else(|_| source.to_string())
}
