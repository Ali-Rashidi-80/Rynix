//! Structured parse diagnostics (`RYX1xxx`).

use rynix_diag::{Diagnostic, Stage, codes};
use rynix_lexer::TokenKind;
use rynix_span::Span;

pub(crate) fn unexpected_token(span: Span, kind: TokenKind) -> Diagnostic {
    Diagnostic::error(
        codes::UNEXPECTED_TOKEN,
        Stage::Parse,
        format!("unexpected {}", kind.describe()),
        span,
    )
}

pub(crate) fn expected_token(
    span: Span,
    expected: &str,
    found: TokenKind,
    insert: Option<&str>,
) -> Diagnostic {
    let mut diag = Diagnostic::error(
        codes::EXPECTED_TOKEN,
        Stage::Parse,
        format!("expected {expected}, found {}", found.describe()),
        span,
    );
    if let Some(text) = insert {
        diag = diag.with_fix(
            format!("insert `{text}`"),
            0.90,
            vec![rynix_diag::Edit {
                span: Span::empty(span.lo()),
                replacement: text.to_string(),
            }],
        );
    }
    diag
}

pub(crate) fn unclosed_delimiter(open: Span, closer: &str) -> Diagnostic {
    Diagnostic::error(
        codes::UNCLOSED_DELIMITER,
        Stage::Parse,
        format!("unclosed delimiter; expected `{closer}`"),
        open,
    )
    .with_label(open, format!("opened here; expected `{closer}`"))
}

pub(crate) fn missing_end(span: Span, insert_at: Span) -> Diagnostic {
    Diagnostic::error(
        codes::MISSING_END,
        Stage::Parse,
        "missing `end` to close this block",
        span,
    )
    .with_fix(
        "insert `end`",
        0.85,
        vec![rynix_diag::Edit {
            span: Span::empty(insert_at.lo()),
            replacement: "end\n".to_string(),
        }],
    )
}

pub(crate) fn reserved_keyword(span: Span, word: &str) -> Diagnostic {
    Diagnostic::error(
        codes::RESERVED_KEYWORD,
        Stage::Parse,
        format!("`{word}` is reserved for a future Rynix release"),
        span,
    )
}

pub(crate) fn unexpected_eof(span: Span, context: &str) -> Diagnostic {
    Diagnostic::error(
        codes::UNEXPECTED_EOF,
        Stage::Parse,
        format!("unexpected end of file while parsing {context}"),
        span,
    )
}

pub(crate) fn chained_comparison(span: Span) -> Diagnostic {
    Diagnostic::error(
        codes::CHAINED_COMPARISON,
        Stage::Parse,
        "comparisons are non-associative; use `and` to combine them",
        span,
    )
    .with_label(span, "write `a < b and b < c`, not `a < b < c`")
}
