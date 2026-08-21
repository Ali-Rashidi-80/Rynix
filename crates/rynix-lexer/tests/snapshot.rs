//! Snapshot tests over the `.ryx` corpus in `testdata/lexer/`.
//!
//! Line endings are normalized to `\n` before lexing so snapshots are
//! identical on Windows and Linux checkouts; `\r\n` handling itself is
//! covered by unit tests in the lexer.

use std::fmt::Write as _;

use rynix_diag::{render_human, Diagnostic, VecSink};
use rynix_lexer::lex_all;
use rynix_span::SourceMap;

fn dump(name: &str, source: &str) -> String {
    let mut sm = SourceMap::new();
    let id = sm.add_owned(name, source.replace("\r\n", "\n"));
    let file = sm.file(id);

    let mut sink = VecSink::new();
    let tokens = lex_all(file.text(), file.start_pos(), &mut sink);

    let mut out = String::new();
    out.push_str("== tokens ==\n");
    for token in &tokens {
        let text = sm.span_text(token.span);
        let _ = writeln!(
            out,
            "{:>5}..{:<5} {:<12} {:?}",
            token.span.lo(),
            token.span.hi(),
            format!("{:?}", token.kind),
            text
        );
    }

    let _ = write!(out, "\n== diagnostics ({}) ==\n", sink.diags.len());
    for diag in &sink.diags {
        let _ = writeln!(out, "{}", render_human(diag, &sm));
        for fix in &diag.fixes {
            for edit in &fix.edits {
                let _ = writeln!(
                    out,
                    "      edit {}..{} -> {:?}",
                    edit.span.lo(),
                    edit.span.hi(),
                    edit.replacement
                );
            }
        }
    }
    out
}

/// Absolute path of the shared `.ryx` corpus.
fn corpus_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/lexer")
        .canonicalize()
        .expect("corpus directory exists")
}

#[test]
fn corpus_snapshots() {
    insta::glob!(corpus_dir(), "*.ryx", |path| {
        let source = std::fs::read_to_string(path).expect("read corpus file");
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .expect("utf-8 file name");
        insta::assert_snapshot!(dump(name, &source));
    });
}

/// The corpus must exercise every registered diagnostic code, so a new code
/// cannot be added without a corpus case demonstrating it.
#[test]
fn corpus_covers_every_lexical_code() {
    let mut seen = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(corpus_dir()).expect("read corpus dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|s| s.to_str()) != Some("ryx") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read corpus file");
        let mut sink = VecSink::new();
        lex_all(&source.replace("\r\n", "\n"), 0, &mut sink);
        seen.extend(sink.diags.iter().map(|d: &Diagnostic| d.code.as_str()));
    }
    let expected = [
        "RYX0001", "RYX0002", "RYX0003", "RYX0004", "RYX0005", "RYX0006",
    ];
    for code in expected {
        assert!(seen.contains(code), "no corpus file produces {code}");
    }
}
