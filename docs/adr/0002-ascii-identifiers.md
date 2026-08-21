# ADR-0002: ASCII-only identifiers in v0.1

Status: accepted (2026-08-21)

## Context

Unicode identifiers (UAX #31) require XID_Start/XID_Continue tables, NFC
normalization, and confusable detection — significant complexity and a
dependency (`unicode-ident`), plus a homoglyph attack surface. LLM tokenizers
also handle ASCII identifiers far more efficiently.

## Decision

v0.1 identifiers are `[A-Za-z_][A-Za-z0-9_]*`. Any non-ASCII text outside
strings and comments is a structured lexical error (`RYX0003`) with total
recovery (the run is consumed as one token; when it extends an ASCII
identifier the token stays `Ident` so the parser can proceed).

Strings and comments remain fully UTF-8.

## Consequences

- The lexer needs no Unicode tables and keeps a pure byte-level fast path.
- Revisiting Unicode identifiers later is additive (new accepted forms), not
  breaking.
