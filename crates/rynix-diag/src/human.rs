//! Compact human-readable rendering.
//!
//! Phase 3 replaces this with a full annotated-snippet renderer; the format
//! here is intentionally close to rustc's "short" style.

use std::fmt::Write as _;

use rynix_span::SourceMap;

use crate::Diagnostic;

/// Renders a diagnostic as short human-readable text (multi-line, no
/// trailing newline).
///
/// ```text
/// error[RYX0002]: unterminated string literal
///  --> a.ryx:1:9: string starts here
///   = note: while lexing this item
///   = help: insert closing `"` (confidence 0.90)
/// ```
pub fn render_human(diag: &Diagnostic, sm: &SourceMap) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "{}[{}]: {}",
        diag.severity.as_str(),
        diag.code,
        diag.message
    );

    let (file, lc) = sm.line_col(diag.primary.span.lo());
    let _ = write!(out, "\n --> {}:{}:{}", file.name(), lc.line, lc.col);
    if !diag.primary.message.is_empty() {
        let _ = write!(out, ": {}", diag.primary.message);
    }

    for label in &diag.secondary {
        let (f, l) = sm.line_col(label.span.lo());
        let _ = write!(
            out,
            "\n  = note: {} ({}:{}:{})",
            label.message,
            f.name(),
            l.line,
            l.col
        );
    }
    for fix in &diag.fixes {
        let _ = write!(
            out,
            "\n  = help: {} (confidence {:.2})",
            fix.message, fix.confidence
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{codes, Diagnostic, Stage};
    use rynix_span::{SourceMap, Span};

    #[test]
    fn short_format() {
        let mut sm = SourceMap::new();
        sm.add_owned("m.ryx", "x $ y\n".to_string());
        let diag = Diagnostic::error(
            codes::UNKNOWN_CHAR,
            Stage::Lex,
            "unknown character `$`",
            Span::new(2, 3),
        )
        .with_replacement_fix("remove it", 0.9, Span::new(2, 3), "");

        let text = render_human(&diag, &sm);
        assert!(text.starts_with("error[RYX0001]: unknown character `$`"));
        assert!(text.contains("--> m.ryx:1:3"));
        assert!(text.contains("= help: remove it (confidence 0.90)"));
    }
}
