//! Golden validation: every rendered diagnostic must satisfy `rynix.diag.v1`.

use rynix_diag::{Diagnostic, Stage, codes, render_json, validate_diag_v1};
use rynix_span::{SourceMap, Span};

#[test]
fn schema_document_is_present_and_names_v1() {
    let schema = include_str!("../../../docs/schemas/rynix.diag.v1.json");
    assert!(schema.contains("\"rynix.diag.v1\""));
    assert!(schema.contains("\"$defs\""));
    let parsed: serde_json::Value = serde_json::from_str(schema).expect("schema JSON");
    assert_eq!(parsed["title"], "rynix.diag.v1");
    assert_eq!(parsed["properties"]["schema"]["const"], "rynix.diag.v1");
}

#[test]
fn rendered_diagnostics_match_schema() {
    let mut sm = SourceMap::new();
    sm.add_owned("a.ryx", "let s = \"abc\n0X1\n".to_string());

    let samples = [
        Diagnostic::error(
            codes::UNTERMINATED_STRING,
            Stage::Lex,
            "unterminated string literal",
            Span::new(8, 12),
        )
        .with_primary_label("string starts here")
        .with_replacement_fix("insert closing `\"`", 0.9, Span::empty(12), "\""),
        Diagnostic::error(
            codes::MALFORMED_NUMBER,
            Stage::Lex,
            "base prefix must be lowercase",
            Span::new(13, 14),
        )
        .with_replacement_fix("lowercase", 0.95, Span::new(13, 14), "x"),
        Diagnostic::error(
            codes::MISSING_END,
            Stage::Parse,
            "missing `end`",
            Span::new(0, 3),
        )
        .with_fix(
            "insert `end`",
            0.85,
            vec![rynix_diag::Edit {
                span: Span::empty(3),
                replacement: "end\n".into(),
            }],
        ),
        Diagnostic::error(
            codes::UNKNOWN_CHAR,
            Stage::Lex,
            "unknown character `$`",
            Span::new(0, 1),
        ),
    ];

    for diag in &samples {
        let line = render_json(diag, &sm);
        assert!(!line.contains('\n'), "NDJSON must be one line: {line}");
        let value: serde_json::Value = serde_json::from_str(&line).expect("json");
        validate_diag_v1(&value).unwrap_or_else(|e| panic!("{e} for {line}"));
    }
}

#[test]
fn fixture_ndjson_lines_match_schema() {
    let fixtures = include_str!("../../../testdata/diagnostics/golden.ndjson");
    for (i, line) in fixtures.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let value: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("line {i}: {e}\n{line}"));
        validate_diag_v1(&value).unwrap_or_else(|e| panic!("line {i}: {e}\n{line}"));
    }
}
