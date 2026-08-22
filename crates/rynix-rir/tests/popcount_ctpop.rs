//! Popcount loop fusion → ctpop in RIR.

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{lower_module, print_module, run_pipeline};
use rynix_sema::analyze;
use rynix_span::Interner;

const POPCOUNT: &str = r#"
def popcount(x: i64) -> i64
  let mut v = x
  let mut c = 0
  loop
    if v == 0
      break
    end
    c += v & 1
    v = v >> 1
  end
  return c
end
"#;

fn lower_rir(src: &str) -> String {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, src, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    print_module(&rir, &interner)
}

#[test]
fn popcount_fn_lowers_to_ctpop() {
    let rir = lower_rir(POPCOUNT);
    assert!(
        rir.contains("ctpop"),
        "expected ctpop in popcount fn, got:\n{rir}"
    );
}
