use crate::{Diagnostic, Severity};

/// Receives diagnostics from compiler stages.
///
/// Stages take `&mut dyn DiagSink` so the hot paths carry only a fat
/// pointer; `emit` is called exclusively on cold (invalid-input) paths.
pub trait DiagSink {
    fn emit(&mut self, diag: Diagnostic);
}

/// Collects every diagnostic. The standard sink for tests and tools.
#[derive(Default, Debug)]
pub struct VecSink {
    pub diags: Vec<Diagnostic>,
}

impl VecSink {
    pub fn new() -> Self {
        VecSink::default()
    }

    pub fn error_count(&self) -> usize {
        self.diags.iter().filter(|d| d.is_error()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.diags.is_empty()
    }
}

impl DiagSink for VecSink {
    fn emit(&mut self, diag: Diagnostic) {
        self.diags.push(diag);
    }
}

/// Counts diagnostics without storing them. Used by benchmarks and the
/// zero-allocation tests (dropping the diagnostic frees nothing on clean
/// runs because none are ever constructed).
#[derive(Default, Debug)]
pub struct CountSink {
    pub errors: usize,
    pub others: usize,
}

impl CountSink {
    pub fn new() -> Self {
        CountSink::default()
    }

    pub fn total(&self) -> usize {
        self.errors + self.others
    }
}

impl DiagSink for CountSink {
    fn emit(&mut self, diag: Diagnostic) {
        if diag.severity == Severity::Error {
            self.errors += 1;
        } else {
            self.others += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Stage, codes};
    use rynix_span::Span;

    #[test]
    fn vec_sink_collects_and_counts() {
        let mut sink = VecSink::new();
        assert!(sink.is_empty());
        sink.emit(Diagnostic::error(
            codes::UNKNOWN_CHAR,
            Stage::Lex,
            "unknown character `$`",
            Span::new(0, 1),
        ));
        assert_eq!(sink.diags.len(), 1);
        assert_eq!(sink.error_count(), 1);
    }

    #[test]
    fn count_sink_discards_but_counts() {
        let mut sink = CountSink::new();
        sink.emit(Diagnostic::error(
            codes::UNKNOWN_CHAR,
            Stage::Lex,
            "x",
            Span::new(0, 1),
        ));
        assert_eq!(sink.errors, 1);
        assert_eq!(sink.total(), 1);
    }
}
