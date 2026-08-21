//! The `rynix.diag.v1` JSON rendering (one object per line, NDJSON).
//!
//! This is the machine interface consumed by AI agents and editor tooling.
//! The schema is versioned via the `schema` field; it will be frozen with a
//! JSON Schema document in Phase 3, but the shape below is already stable:
//!
//! ```json
//! {
//!   "schema": "rynix.diag.v1",
//!   "code": "RYX0002",
//!   "severity": "error",
//!   "stage": "lex",
//!   "message": "unterminated string literal",
//!   "spans": [{ "file": "a.ryx", "lo": 10, "hi": 14, "line": 1, "col": 11,
//!               "end_line": 1, "end_col": 15, "primary": true, "label": "" }],
//!   "fixes": [{ "message": "insert closing `\"`", "confidence": 0.9,
//!               "edits": [{ "file": "a.ryx", "lo": 14, "hi": 14,
//!                           "replacement": "\"" }] }]
//! }
//! ```
//!
//! Offsets (`lo`/`hi`) are *global* SourceMap offsets; `line`/`col` are
//! 1-based and count bytes.

use rynix_span::{SourceMap, Span};
use serde::Serialize;

use crate::{Diagnostic, Label};

#[derive(Serialize)]
struct JsonDiag<'a> {
    schema: &'static str,
    code: &'a str,
    severity: &'a str,
    stage: &'a str,
    message: &'a str,
    spans: Vec<JsonSpan<'a>>,
    fixes: Vec<JsonFix<'a>>,
}

#[derive(Serialize)]
struct JsonSpan<'a> {
    file: &'a str,
    lo: u32,
    hi: u32,
    line: u32,
    col: u32,
    end_line: u32,
    end_col: u32,
    primary: bool,
    label: &'a str,
}

#[derive(Serialize)]
struct JsonFix<'a> {
    message: &'a str,
    confidence: f32,
    edits: Vec<JsonEdit<'a>>,
}

#[derive(Serialize)]
struct JsonEdit<'a> {
    file: &'a str,
    lo: u32,
    hi: u32,
    replacement: &'a str,
}

fn json_span<'a>(sm: &'a SourceMap, label: &'a Label, primary: bool) -> JsonSpan<'a> {
    let (file, start) = sm.line_col(label.span.lo());
    let (_, end) = sm.line_col(label.span.hi());
    JsonSpan {
        file: file.name(),
        lo: label.span.lo(),
        hi: label.span.hi(),
        line: start.line,
        col: start.col,
        end_line: end.line,
        end_col: end.col,
        primary,
        label: &label.message,
    }
}

fn edit_location<'a>(sm: &'a SourceMap, span: Span) -> &'a str {
    sm.line_col(span.lo()).0.name()
}

/// Renders one diagnostic as a single `rynix.diag.v1` JSON line (no trailing
/// newline).
pub fn render_json(diag: &Diagnostic, sm: &SourceMap) -> String {
    let mut spans = Vec::with_capacity(1 + diag.secondary.len());
    spans.push(json_span(sm, &diag.primary, true));
    spans.extend(diag.secondary.iter().map(|l| json_span(sm, l, false)));

    let fixes = diag
        .fixes
        .iter()
        .map(|f| JsonFix {
            message: &f.message,
            confidence: f.confidence,
            edits: f
                .edits
                .iter()
                .map(|e| JsonEdit {
                    file: edit_location(sm, e.span),
                    lo: e.span.lo(),
                    hi: e.span.hi(),
                    replacement: &e.replacement,
                })
                .collect(),
        })
        .collect();

    let payload = JsonDiag {
        schema: "rynix.diag.v1",
        code: diag.code.as_str(),
        severity: diag.severity.as_str(),
        stage: diag.stage.as_str(),
        message: &diag.message,
        spans,
        fixes,
    };
    serde_json::to_string(&payload).expect("diagnostic serialization cannot fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{codes, Stage};
    use rynix_span::SourceMap;

    #[test]
    fn schema_v1_shape() {
        let mut sm = SourceMap::new();
        sm.add_owned("a.ryx", "let s = \"abc\n".to_string());
        let diag = Diagnostic::error(
            codes::UNTERMINATED_STRING,
            Stage::Lex,
            "unterminated string literal",
            Span::new(8, 12),
        )
        .with_primary_label("string starts here")
        .with_replacement_fix("insert closing `\"`", 0.9, Span::empty(12), "\"");

        let line = render_json(&diag, &sm);
        let v: serde_json::Value = serde_json::from_str(&line).expect("valid JSON");

        assert_eq!(v["schema"], "rynix.diag.v1");
        assert_eq!(v["code"], "RYX0002");
        assert_eq!(v["severity"], "error");
        assert_eq!(v["stage"], "lex");
        assert_eq!(v["spans"][0]["file"], "a.ryx");
        assert_eq!(v["spans"][0]["lo"], 8);
        assert_eq!(v["spans"][0]["hi"], 12);
        assert_eq!(v["spans"][0]["line"], 1);
        assert_eq!(v["spans"][0]["col"], 9);
        assert_eq!(v["spans"][0]["end_col"], 13);
        assert_eq!(v["spans"][0]["primary"], true);
        assert_eq!(v["fixes"][0]["confidence"], 0.9);
        assert_eq!(v["fixes"][0]["edits"][0]["replacement"], "\"");
        assert!(!line.contains('\n'), "NDJSON: exactly one line");
    }
}
