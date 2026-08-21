//! Annotated human-readable diagnostic rendering (rustc-style snippets).

use std::fmt::Write as _;

use rynix_span::SourceMap;

use crate::Diagnostic;

/// Renders a diagnostic as annotated human-readable text (multi-line, no
/// trailing newline).
///
/// ```text
/// error[RYX0001]: unknown character `$`
///  --> m.ryx:1:3
///   |
/// 1 | x $ y
///   |   ^
///   = help: remove it (confidence 0.90)
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

    let (file, start) = sm.line_col(diag.primary.span.lo());
    let (_, end) = sm.line_col(diag.primary.span.hi());
    let _ = write!(out, "\n --> {}:{}:{}", file.name(), start.line, start.col);
    if !diag.primary.message.is_empty() {
        let _ = write!(out, ": {}", diag.primary.message);
    }

    // Snippet for the primary span (single-line highlight; multi-line spans
    // mark from the start column through end-of-line on the first line).
    if start.line >= 1 && start.line <= file.line_count() {
        let line = file.line_text(start.line);
        let gutter = format!("{}", start.line);
        let pad = gutter.len();
        let _ = write!(out, "\n{:>pad$} |", "", pad = pad);
        let _ = write!(out, "\n{gutter} | {line}");

        let caret_lo = (start.col as usize).saturating_sub(1);
        let caret_hi = if end.line == start.line {
            (end.col as usize).saturating_sub(1).max(caret_lo + 1)
        } else {
            line.len().max(caret_lo + 1)
        };
        let caret_hi = caret_hi.min(line.len().max(caret_lo + 1));
        let mut mark = String::with_capacity(caret_hi);
        for i in 0..caret_hi {
            if i < caret_lo {
                mark.push(' ');
            } else {
                mark.push('^');
            }
        }
        if mark.is_empty() {
            mark.push('^');
        }
        let _ = write!(out, "\n{:>pad$} | {mark}", "", pad = pad);
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
    use crate::{Diagnostic, Stage, codes};
    use rynix_span::{SourceMap, Span};

    #[test]
    fn annotated_snippet() {
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
        assert!(text.contains("1 | x $ y"), "{text}");
        assert!(text.contains("|   ^"), "{text}");
        assert!(text.contains("= help: remove it (confidence 0.90)"));
    }
}
