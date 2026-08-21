# ADR-0001: Canonical keyword-delimited syntax (`def ... end`)

Status: accepted (2026-08-21)

## Context

Rynix is AI-native: most Rynix code will be written and repaired by LLMs.
Research summarized in the project design report shows keyword-based
languages align better with LLM tokenizer vocabularies (up to ~2.6x fewer
tokens than symbol-heavy languages) and reduce hallucinated syntax. Candidate
block styles: C-style braces, Python-style indentation, Ruby-style keywords.

## Decision

Blocks open with a header (`def`, `struct`, `if`, `loop`, `for`, ...) and
close with `end`. Newlines terminate statements; there are no semicolons.
Logical operators are the words `and`/`or`/`not`. For every construct there
is exactly one surface form ("one way to do it"): no `while`, one comment
form (`#`), one string form, no alternative operator spellings.

Indentation was rejected because INDENT/DEDENT lexing is context-sensitive
and brittle for machine-generated code; braces were rejected for token noise
and because `{ }` is reserved for data literals.

## Consequences

- The lexer stays context-free; `Newline` is a significant token and the
  parser ignores it inside bracketed groups.
- The formatter can be zero-config (canonical layout).
- Grammar changes that would introduce a second way to write an existing
  construct are rejected by policy.
