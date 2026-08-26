# Fuzzing Rynix

Fuzzing requires a nightly toolchain and libFuzzer, so it runs on Linux
(WSL2 works) and in CI, not on the Windows dev loop. This directory is its
own Cargo workspace and is excluded from the main build graph.

## Setup

```sh
rustup toolchain install nightly
cargo install cargo-fuzz
```

## Targets

- `lex_total` — the lexer is total: tokens tile the input byte-exactly, never
  split a UTF-8 character, and always make progress.
- `lex_diagnostics` — every diagnostic renders as valid `rynix.diag.v1` JSON
  and every fix is mechanically applicable to the source.
- `parse_no_crash` — parser must not panic on arbitrary UTF-8 (Phase 27-D);
  seed corpus under `corpus/parse_no_crash/`.

## Running

The committed `.ryx` corpus makes a good seed set:

```sh
cd fuzz
cargo +nightly fuzz run lex_total ../testdata/lexer -- -max_total_time=300
cargo +nightly fuzz run lex_diagnostics ../testdata/lexer -- -max_total_time=300
```

Both targets take `&str`, so libFuzzer only feeds valid UTF-8; invalid UTF-8
is rejected earlier, at `SourceMap` load time.
