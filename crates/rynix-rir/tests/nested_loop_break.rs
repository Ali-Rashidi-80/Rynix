//! Suite5 nested: opaque trip count uses residue O(m²) form (not n×n urem nest).

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{lower_module, print_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::Interner;

const NESTED_MAIN: &str = include_str!("../../../benchmarks/suite5/nested.ryx");

#[test]
fn nested_opaque_uses_residue_form() {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, NESTED_MAIN, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, NESTED_MAIN, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    let text = print_module(&rir, &interner);
    assert!(
        text.contains("rynix_rt_opaque_i64"),
        "expected opaque barrier:\n{text}"
    );
    assert!(
        text.contains("idiv") && text.contains("jump"),
        "expected residue O(m²) loops:\n{text}"
    );
    let urem_count = text.matches("urem").count() + text.matches("irem").count();
    assert!(
        urem_count < 8,
        "expected compact residue rem use, got {urem_count}:\n{text}"
    );
}
