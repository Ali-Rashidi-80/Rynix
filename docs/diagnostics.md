# Rynix Diagnostic Code Registry

Every diagnostic the compiler can emit has a stable code. Codes are never
reused or renumbered. The machine-readable registry lives in
`crates/rynix-diag/src/code.rs`; a test asserts that every registered code is
documented in this file.

Numbering plan:

- `RYX0xxx` — lexical (Phase 1)
- `RYX1xxx` — syntactic (Phase 2)
- `RYX2xxx` — names and types (Phase 4)
- `RYX3xxx` — escape/region analysis (Phase 6)
- `RYX4xxx` — code generation and linking (Phase 7)
- `RYX5xxx` — runtime-facing checks (Phase 8)

Fix policy: fixes carry a confidence in `[0.0, 1.0]`. Anything at or above
`0.9` is safe for an AI agent to apply without confirmation; below that, the
fix is a suggestion. Fixes are edit lists (span + replacement) and must be
mechanically applicable.

## Lexical diagnostics

### RYX0001 — unknown character

A byte that cannot start any token (e.g. `$`, `@`, `;`, `` ` ``, a lone `!`,
`&`, `|`, or a stray control character). The character is consumed as an
`Unknown` token and lexing continues.

Targeted fixes:

- `;` → remove it (newlines terminate statements), confidence 0.90
- `!` → replace with `not `, confidence 0.85
- `&`/`&&` → replace with `and`, confidence 0.85 (0.60 for a single `&`)
- `|`/`||` → replace with `or`, confidence 0.85 (0.60 for a single `|`)
- `'` → replace with `"` (strings use double quotes), confidence 0.70

### RYX0002 — unterminated string literal

A raw line terminator was found inside a string. The token ends before the
newline. Fix: insert `"` at the end of the string content, confidence 0.90.

### RYX0003 — non-ASCII identifier

Non-ASCII text outside strings and comments. Identifiers are ASCII-only in
v0.1 (ADR-0002). The whole run is consumed (as `Ident` when it extends an
ASCII identifier, otherwise as `Unknown`) and lexing continues.

### RYX0004 — malformed number literal

Covers: uppercase base prefix (`0X1` — fix: lowercase, confidence 0.95),
missing digits after a base prefix (`0x`), invalid digit for the base
(`0b12`, `0o9`), misplaced underscore (`1__0`, `1_`, `0x_1`), missing
exponent digits (`1e`, `1e+`), uppercase exponent (`1E5` — fix: lowercase
`e`, confidence 0.90), and numeric suffixes (`123abc`, suffixes do not exist
in v0.1). The sub-span points at the offending bytes.

### RYX0005 — invalid escape sequence

Unknown escape character, malformed `\x` (needs exactly two hex digits), or
malformed `\u{...}` (needs 1–6 hex digits, a closing `}`, and a valid Unicode
scalar value). Fix for unknown escapes: remove the backslash, confidence 0.70.

### RYX0006 — end of file inside string literal

The file ended before the closing `"`. Fix: append `"`, confidence 0.80.
