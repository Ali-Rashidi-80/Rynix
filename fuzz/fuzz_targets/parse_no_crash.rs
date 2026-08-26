//! Fuzz target: parse must not panic on arbitrary UTF-8 input.
//!
//! Seed corpus: `fuzz/corpus/parse_no_crash/`
//!
//! Run (nightly + cargo-fuzz):
//! `cargo +nightly fuzz run parse_no_crash`

#![no_main]

use libfuzzer_sys::fuzz_target;
use rynix_ast::AstArena;
use rynix_diag::CountSink;
use rynix_parser::parse;
use rynix_span::Interner;

fuzz_target!(|data: &str| {
    let arena = AstArena::new();
    let mut interner = Interner::new();
    let mut sink = CountSink::new();
    let _ = parse(&arena, &mut interner, data, 0, &mut sink);
});
