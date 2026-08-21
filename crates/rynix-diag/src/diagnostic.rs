use rynix_span::Span;

use crate::DiagCode;

/// Diagnostic severity.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
            Severity::Help => "help",
        }
    }
}

/// The compiler stage that produced a diagnostic.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Lex,
    Parse,
    Sema,
    Ir,
    Codegen,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Stage::Lex => "lex",
            Stage::Parse => "parse",
            Stage::Sema => "sema",
            Stage::Ir => "ir",
            Stage::Codegen => "codegen",
        }
    }
}

/// A span with an explanatory message.
#[derive(Clone, Debug)]
pub struct Label {
    pub span: Span,
    pub message: String,
}

/// A single text edit: replace the bytes covered by `span` with
/// `replacement`. An empty span is a pure insertion; an empty replacement is
/// a deletion.
#[derive(Clone, Debug)]
pub struct Edit {
    pub span: Span,
    pub replacement: String,
}

/// A machine-applicable fix.
///
/// `confidence` is in `[0.0, 1.0]`. Policy (see `docs/diagnostics.md`):
/// fixes at or above 0.9 are safe for an AI agent to apply without
/// confirmation.
#[derive(Clone, Debug)]
pub struct Fix {
    pub message: String,
    pub confidence: f32,
    pub edits: Vec<Edit>,
}

/// A structured compiler diagnostic (see `rynix.diag.v1` in
/// [`render_json`](crate::render_json)).
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub code: DiagCode,
    pub severity: Severity,
    pub stage: Stage,
    pub message: String,
    pub primary: Label,
    pub secondary: Vec<Label>,
    pub fixes: Vec<Fix>,
}

impl Diagnostic {
    pub fn new(
        code: DiagCode,
        severity: Severity,
        stage: Stage,
        message: impl Into<String>,
        span: Span,
    ) -> Self {
        Diagnostic {
            code,
            severity,
            stage,
            message: message.into(),
            primary: Label {
                span,
                message: String::new(),
            },
            secondary: Vec::new(),
            fixes: Vec::new(),
        }
    }

    /// Shorthand for an error diagnostic.
    pub fn error(code: DiagCode, stage: Stage, message: impl Into<String>, span: Span) -> Self {
        Diagnostic::new(code, Severity::Error, stage, message, span)
    }

    /// Sets the message shown at the primary span.
    #[must_use]
    pub fn with_primary_label(mut self, message: impl Into<String>) -> Self {
        self.primary.message = message.into();
        self
    }

    /// Attaches a secondary label.
    #[must_use]
    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.secondary.push(Label {
            span,
            message: message.into(),
        });
        self
    }

    /// Attaches a multi-edit fix.
    #[must_use]
    pub fn with_fix(mut self, message: impl Into<String>, confidence: f32, edits: Vec<Edit>) -> Self {
        debug_assert!(
            (0.0..=1.0).contains(&confidence),
            "confidence {confidence} out of range"
        );
        self.fixes.push(Fix {
            message: message.into(),
            confidence,
            edits,
        });
        self
    }

    /// Attaches the common single-edit fix: replace `span` with `replacement`.
    #[must_use]
    pub fn with_replacement_fix(
        self,
        message: impl Into<String>,
        confidence: f32,
        span: Span,
        replacement: impl Into<String>,
    ) -> Self {
        self.with_fix(
            message,
            confidence,
            vec![Edit {
                span,
                replacement: replacement.into(),
            }],
        )
    }

    #[inline]
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes;

    #[test]
    fn builder_composes() {
        let d = Diagnostic::error(
            codes::UNTERMINATED_STRING,
            Stage::Lex,
            "unterminated string literal",
            Span::new(10, 14),
        )
        .with_primary_label("string starts here")
        .with_label(Span::new(0, 3), "in this item")
        .with_replacement_fix("insert closing `\"`", 0.9, Span::empty(14), "\"");

        assert!(d.is_error());
        assert_eq!(d.code.as_str(), "RYX0002");
        assert_eq!(d.stage.as_str(), "lex");
        assert_eq!(d.primary.span, Span::new(10, 14));
        assert_eq!(d.secondary.len(), 1);
        assert_eq!(d.fixes.len(), 1);
        assert_eq!(d.fixes[0].edits[0].replacement, "\"");
        assert!(d.fixes[0].confidence > 0.89);
    }
}
