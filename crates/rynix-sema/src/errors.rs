//! Semantic diagnostic constructors (`RYX2xxx`).

use rynix_diag::{Diagnostic, Stage, codes};
use rynix_span::Span;

pub(crate) fn unresolved_name(span: Span, name: &str) -> Diagnostic {
    Diagnostic::error(
        codes::UNRESOLVED_NAME,
        Stage::Sema,
        format!("cannot find `{name}` in this scope"),
        span,
    )
}

pub(crate) fn duplicate_def(span: Span, name: &str, previous: Span) -> Diagnostic {
    Diagnostic::error(
        codes::DUPLICATE_DEF,
        Stage::Sema,
        format!("the name `{name}` is defined multiple times"),
        span,
    )
    .with_label(previous, "previous definition here")
}

pub(crate) fn type_mismatch(span: Span, expected: &str, found: &str) -> Diagnostic {
    Diagnostic::error(
        codes::TYPE_MISMATCH,
        Stage::Sema,
        format!("mismatched types: expected `{expected}`, found `{found}`"),
        span,
    )
}

pub(crate) fn expected_type_name(span: Span, name: &str) -> Diagnostic {
    Diagnostic::error(
        codes::EXPECTED_TYPE,
        Stage::Sema,
        format!("expected a type, found value `{name}`"),
        span,
    )
}

pub(crate) fn immutable_assign(span: Span, name: &str) -> Diagnostic {
    Diagnostic::error(
        codes::IMMUTABLE_ASSIGN,
        Stage::Sema,
        format!("cannot assign to immutable binding `{name}`"),
        span,
    )
}

pub(crate) fn unknown_field(span: Span, ty: &str, field: &str) -> Diagnostic {
    Diagnostic::error(
        codes::UNKNOWN_FIELD,
        Stage::Sema,
        format!("no field `{field}` on type `{ty}`"),
        span,
    )
}

pub(crate) fn unknown_method(span: Span, ty: &str, method: &str) -> Diagnostic {
    Diagnostic::error(
        codes::UNKNOWN_FIELD,
        Stage::Sema,
        format!("no method `{method}` on type `{ty}`"),
        span,
    )
}

pub(crate) fn wrong_arity(span: Span, expected: usize, found: usize) -> Diagnostic {
    Diagnostic::error(
        codes::WRONG_ARITY,
        Stage::Sema,
        format!(
            "this function takes {expected} argument{}, but {found} {} supplied",
            if expected == 1 { "" } else { "s" },
            if found == 1 { "was" } else { "were" }
        ),
        span,
    )
}

pub(crate) fn break_outside_loop(span: Span) -> Diagnostic {
    Diagnostic::error(
        codes::BREAK_OUTSIDE_LOOP,
        Stage::Sema,
        "`break` outside of a loop",
        span,
    )
}

pub(crate) fn continue_outside_loop(span: Span) -> Diagnostic {
    Diagnostic::error(
        codes::CONTINUE_OUTSIDE_LOOP,
        Stage::Sema,
        "`continue` outside of a loop",
        span,
    )
}

pub(crate) fn not_callable(span: Span, ty: &str) -> Diagnostic {
    Diagnostic::error(
        codes::NOT_CALLABLE,
        Stage::Sema,
        format!("expected a function, found `{ty}`"),
        span,
    )
}

pub(crate) fn use_after_move(span: Span, name: &str, to: &str, moved_at: Span) -> Diagnostic {
    Diagnostic::error(
        codes::USE_AFTER_MOVE,
        Stage::Sema,
        format!("use of moved value `{name}` (moved to `{to}`)"),
        span,
    )
    .with_label(moved_at, "value moved here")
}

pub(crate) fn purity_violation(span: Span, name: &str, effects: &str) -> Diagnostic {
    Diagnostic::error(
        codes::PURITY_VIOLATION,
        Stage::Sema,
        format!("function `{name}` is marked `#^ effect: pure` but has impure effects: {effects}"),
        span,
    )
}

pub(crate) fn stub_reserved(span: Span, name: &str) -> Diagnostic {
    Diagnostic::error(
        codes::STUB_RESERVED,
        Stage::Sema,
        format!("`{name}` is reserved and not callable in v0.1 (no runtime)"),
        span,
    )
}

#[allow(dead_code)] // retained for future unsupported place forms
pub(crate) fn field_assign_unsupported(span: Span) -> Diagnostic {
    Diagnostic::error(
        codes::FIELD_ASSIGN_UNSUPPORTED,
        Stage::Sema,
        "assignment to index is unsupported",
        span,
    )
}
