//! Snapshot tests: s-expression dumps of parsed `.ryx` corpora.

use std::path::PathBuf;

use rynix_ast::{AstArena, dump_module};
use rynix_diag::{Diagnostic, VecSink, render_human};
use rynix_parser::parse;
use rynix_span::{Interner, SourceMap};

fn dump_with_diags(src: &str, name: &str) -> String {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = VecSink::new();
    let module = parse(&arena, &mut interner, src, 0, &mut sink);

    let mut out = dump_module(module, &interner);
    if !sink.diags.is_empty() {
        out.push_str("\n--- diagnostics ---\n");
        let mut sm = SourceMap::new();
        let _ = sm.add_owned(name, src.to_string());
        for d in &sink.diags {
            out.push_str(&render_human(d, &sm));
            out.push('\n');
        }
    }
    out
}

#[test]
fn corpus_snapshots() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/lexer")
        .canonicalize()
        .expect("corpus dir");
    insta::glob!(&dir, "*.ryx", |path| {
        let name = path.file_name().unwrap().to_string_lossy();
        let src = std::fs::read_to_string(path).expect("read");
        // Normalize CRLF so snapshots are stable across Windows checkouts.
        let src = src.replace("\r\n", "\n");
        let dump = dump_with_diags(&src, &name);
        insta::assert_snapshot!(name.as_ref(), dump);
    });
}

#[test]
fn recovery_snapshot() {
    let src = "\
def broken(
  return 1
end

def ok()
  return 2
end
";
    let dump = dump_with_diags(src, "recovery.ryx");
    insta::assert_snapshot!(dump);
}

// Keep Diagnostic in scope for type inference in helpers if needed later.
#[allow(dead_code)]
fn _ty(_: &Diagnostic) {}
