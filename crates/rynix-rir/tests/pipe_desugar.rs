//! Pipeline `|>` desugars to call with lhs prepended (SPEC §3.2).

use rynix_ast::AstArena;
use rynix_diag::VecSink;
use rynix_parser::parse;
use rynix_rir::{interpret_module, lower_module, run_pipeline, InterpValue};
use rynix_sema::analyze;
use rynix_span::Interner;

fn run(src: &str) -> i64 {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let analysis = analyze(module, &mut interner, &mut sink);
    assert_eq!(sink.error_count(), 0, "{:?}", sink.diags);
    let mut rir = lower_module(module, &analysis, &mut interner, src, 0);
    assert!(run_pipeline(&mut rir).is_empty());
    match interpret_module(&rir, &interner).expect("interp") {
        InterpValue::I64(n) => n,
        other => panic!("expected i64, got {other:?}"),
    }
}

#[test]
fn pipe_bare_path_prepends_lhs() {
    let src = r"
def double(x: i64) -> i64
  return x * 2
end

def main() -> i64
  return 21 |> double
end
";
    assert_eq!(run(src), 42);
}

#[test]
fn pipe_call_prepends_lhs_to_args() {
    let src = r"
def add(a: i64, b: i64) -> i64
  return a + b
end

def main() -> i64
  return 10 |> add(32)
end
";
    assert_eq!(run(src), 42);
}

#[test]
fn pipe_chain_left_assoc() {
    let src = r"
def double(x: i64) -> i64
  return x * 2
end

def add(a: i64, b: i64) -> i64
  return a + b
end

def main() -> i64
  return 10 |> add(1) |> double
end
";
    assert_eq!(run(src), 22);
}
