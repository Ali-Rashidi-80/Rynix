//! Construction of the lexer's structured diagnostics (`RYX0001..RYX0006`).
//!
//! Everything here is on the cold path: these functions allocate, the lexer
//! itself does not. Fix confidences follow the policy in
//! `docs/diagnostics.md` (>= 0.9 is auto-applicable by an agent).

use rynix_diag::{codes, Diagnostic, Stage};
use rynix_span::Span;

/// Renders bytes for a diagnostic message, escaping control characters.
fn display_bytes(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            '\0' => out.push_str("\\0"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{{{:x}}}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// `RYX0001` — a byte that cannot start any token.
///
/// `bytes` is the exact source text of the recovery token (one character, or
/// two for `&&`/`||`), which lets us offer a canonical-syntax replacement.
pub(crate) fn unknown_char(span: Span, bytes: &[u8]) -> Diagnostic {
    let shown = display_bytes(bytes);
    let diag = Diagnostic::error(
        codes::UNKNOWN_CHAR,
        Stage::Lex,
        format!("unknown character `{shown}`"),
        span,
    );
    match bytes {
        b";" => diag
            .with_primary_label("Rynix has no statement separator")
            .with_replacement_fix(
                "remove `;` (newlines terminate statements)",
                0.90,
                span,
                "",
            ),
        b"!" => diag
            .with_primary_label("logical negation is spelled `not`")
            .with_replacement_fix("replace `!` with `not `", 0.85, span, "not "),
        b"&&" => diag
            .with_primary_label("logical conjunction is spelled `and`")
            .with_replacement_fix("replace `&&` with `and`", 0.85, span, "and"),
        b"&" => diag
            .with_primary_label("Rynix has no `&` operator")
            .with_replacement_fix("replace `&` with `and`", 0.60, span, "and"),
        b"||" => diag
            .with_primary_label("logical disjunction is spelled `or`")
            .with_replacement_fix("replace `||` with `or`", 0.85, span, "or"),
        b"|" => diag
            .with_primary_label("Rynix has no `|` operator")
            .with_replacement_fix("replace `|` with `or`", 0.60, span, "or"),
        b"'" => diag
            .with_primary_label("strings use double quotes")
            .with_replacement_fix("replace `'` with `\"`", 0.70, span, "\""),
        _ => diag,
    }
}

/// `RYX0002` — a raw line terminator was found inside a string literal.
pub(crate) fn unterminated_string(span: Span) -> Diagnostic {
    Diagnostic::error(
        codes::UNTERMINATED_STRING,
        Stage::Lex,
        "unterminated string literal",
        span,
    )
    .with_primary_label("string literals may not span lines")
    .with_replacement_fix(
        "insert the closing `\"`",
        0.90,
        Span::empty(span.hi()),
        "\"",
    )
}

/// `RYX0003` — non-ASCII text outside strings and comments (ADR-0002).
pub(crate) fn non_ascii_ident(span: Span, extends_ident: bool) -> Diagnostic {
    let diag = Diagnostic::error(
        codes::NON_ASCII_IDENT,
        Stage::Lex,
        "identifiers must be ASCII-only in Rynix v0.1",
        span,
    );
    if extends_ident {
        diag.with_primary_label("non-ASCII characters in this identifier")
    } else {
        diag.with_primary_label("non-ASCII text is only allowed in strings and comments")
    }
}

/// `RYX0004` — malformed number literal.
///
/// `span` covers the offending bytes; `whole` covers the entire literal so
/// the reader sees the context.
pub(crate) fn malformed_number(
    span: Span,
    whole: Span,
    message: impl Into<String>,
) -> Diagnostic {
    let mut diag = Diagnostic::error(codes::MALFORMED_NUMBER, Stage::Lex, message, span);
    if whole != span {
        diag = diag.with_label(whole, "in this number literal");
    }
    diag
}

/// `RYX0004` with a high-confidence lowercase fix (`0X`, `1E5`).
pub(crate) fn wrong_case_in_number(
    span: Span,
    whole: Span,
    message: impl Into<String>,
    lowered: &str,
    confidence: f32,
) -> Diagnostic {
    malformed_number(span, whole, message).with_replacement_fix(
        format!("write it lowercase: `{lowered}`"),
        confidence,
        span,
        lowered,
    )
}

/// `RYX0005` — invalid escape sequence inside a string literal.
pub(crate) fn invalid_escape(span: Span, message: impl Into<String>, removable: bool) -> Diagnostic {
    let diag = Diagnostic::error(codes::INVALID_ESCAPE, Stage::Lex, message, span);
    if removable {
        diag.with_replacement_fix(
            "remove the backslash to write the character literally",
            0.70,
            Span::new(span.lo(), span.lo() + 1),
            "",
        )
    } else {
        diag
    }
}

/// `RYX0006` — end of file reached inside a string literal.
pub(crate) fn eof_in_string(span: Span) -> Diagnostic {
    Diagnostic::error(
        codes::EOF_IN_STRING,
        Stage::Lex,
        "end of file inside string literal",
        span,
    )
    .with_primary_label("this string is never closed")
    .with_replacement_fix(
        "append the closing `\"`",
        0.80,
        Span::empty(span.hi()),
        "\"",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_replacement_fixes() {
        let span = Span::new(4, 6);
        let d = unknown_char(span, b"&&");
        assert_eq!(d.code.as_str(), "RYX0001");
        assert_eq!(d.fixes[0].edits[0].replacement, "and");
        assert!(d.fixes[0].confidence > 0.8);

        let d = unknown_char(Span::new(1, 2), b";");
        assert_eq!(d.fixes[0].edits[0].replacement, "");

        let d = unknown_char(Span::new(1, 2), b"@");
        assert!(d.fixes.is_empty(), "no canonical replacement for `@`");
        assert_eq!(d.message, "unknown character `@`");
    }

    #[test]
    fn control_characters_are_escaped_in_messages() {
        let d = unknown_char(Span::new(0, 1), b"\x07");
        assert_eq!(d.message, "unknown character `\\u{7}`");
    }

    #[test]
    fn insertion_fixes_are_empty_spans_at_the_end() {
        let d = unterminated_string(Span::new(8, 12));
        let edit = &d.fixes[0].edits[0];
        assert!(edit.span.is_empty());
        assert_eq!(edit.span.lo(), 12);
        assert_eq!(edit.replacement, "\"");
    }
}
