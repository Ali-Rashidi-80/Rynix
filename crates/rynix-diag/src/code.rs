use std::fmt;

/// A stable, machine-readable diagnostic code (`RYX####`).
///
/// Codes are never reused or renumbered; the full registry with prose
/// documentation lives in `docs/diagnostics.md` (a test enforces that every
/// registered code is documented there).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiagCode(&'static str);

impl DiagCode {
    pub(crate) const fn new(code: &'static str) -> Self {
        DiagCode(code)
    }

    #[inline]
    pub fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Debug for DiagCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl fmt::Display for DiagCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Registry entry: code plus a short title (the long docs are in
/// `docs/diagnostics.md`).
pub struct CodeInfo {
    pub code: DiagCode,
    pub title: &'static str,
}

macro_rules! declare_codes {
    ($($name:ident = ($code:literal, $title:literal);)+) => {
        $(pub const $name: DiagCode = DiagCode::new($code);)+

        /// Every diagnostic code the compiler can currently emit.
        pub const REGISTRY: &[CodeInfo] = &[
            $(CodeInfo { code: $name, title: $title },)+
        ];
    };
}

/// The diagnostic code registry. Numbering plan: `RYX0xxx` lexical,
/// `RYX1xxx` syntactic, `RYX2xxx` names/types, `RYX3xxx` escape/regions,
/// `RYX4xxx` codegen/linking, `RYX5xxx` runtime-facing.
pub mod codes {
    use super::{CodeInfo, DiagCode};

    declare_codes! {
        UNKNOWN_CHAR        = ("RYX0001", "unknown character");
        UNTERMINATED_STRING = ("RYX0002", "unterminated string literal");
        NON_ASCII_IDENT     = ("RYX0003", "non-ASCII identifier");
        MALFORMED_NUMBER    = ("RYX0004", "malformed number literal");
        INVALID_ESCAPE      = ("RYX0005", "invalid escape sequence");
        EOF_IN_STRING       = ("RYX0006", "end of file inside string literal");

        UNEXPECTED_TOKEN    = ("RYX1001", "unexpected token");
        EXPECTED_TOKEN      = ("RYX1002", "expected token");
        UNCLOSED_DELIMITER  = ("RYX1003", "unclosed delimiter");
        MISSING_END         = ("RYX1004", "missing `end`");
        RESERVED_KEYWORD    = ("RYX1005", "reserved keyword");
        UNEXPECTED_EOF      = ("RYX1006", "unexpected end of file");
        CHAINED_COMPARISON  = ("RYX1007", "chained comparison");

        UNRESOLVED_NAME     = ("RYX2001", "unresolved name");
        DUPLICATE_DEF       = ("RYX2002", "duplicate definition");
        TYPE_MISMATCH       = ("RYX2003", "type mismatch");
        EXPECTED_TYPE       = ("RYX2004", "expected a type");
        IMMUTABLE_ASSIGN    = ("RYX2005", "assignment to immutable binding");
        UNKNOWN_FIELD       = ("RYX2006", "unknown field");
        WRONG_ARITY         = ("RYX2007", "wrong number of arguments");
        BREAK_OUTSIDE_LOOP  = ("RYX2008", "break/continue outside loop");
        CONTINUE_OUTSIDE_LOOP = ("RYX2009", "continue outside loop");
        NOT_CALLABLE        = ("RYX2010", "value is not callable");
        USE_AFTER_MOVE      = ("RYX2011", "use of moved value");
        PURITY_VIOLATION    = ("RYX2012", "pure function has impure effects");
    }
}

#[cfg(test)]
mod tests {
    use super::codes::REGISTRY;

    #[test]
    fn codes_are_unique_and_well_formed() {
        let mut seen = std::collections::HashSet::new();
        for info in REGISTRY {
            let c = info.code.as_str();
            assert!(c.starts_with("RYX") && c.len() == 7, "malformed code {c}");
            assert!(c[3..].chars().all(|ch| ch.is_ascii_digit()), "{c}");
            assert!(seen.insert(c), "duplicate code {c}");
            assert!(!info.title.is_empty());
        }
    }

    #[test]
    fn every_code_is_documented() {
        let docs = include_str!("../../../docs/diagnostics.md");
        for info in REGISTRY {
            assert!(
                docs.contains(info.code.as_str()),
                "{} is not documented in docs/diagnostics.md",
                info.code
            );
        }
    }
}
