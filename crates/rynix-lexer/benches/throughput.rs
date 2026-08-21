//! Lexer throughput benchmarks.
//!
//! Run with `cargo bench -p rynix-lexer`. Results are reported in bytes per
//! second (criterion's `Throughput::Bytes`); the Phase 1 target is >= 400
//! MB/s single-core on the mixed corpus, with 1 GB/s as the stretch goal.
//!
//! Each workload isolates one part of the hot loop so a regression points at
//! the responsible scanner:
//!
//! - `identifiers` — dispatch + identifier scan + keyword table
//! - `numbers`     — numeric scanning and validation
//! - `strings`     — memchr-driven string scanning and escape handling
//! - `punctuation` — pure dispatch, the highest token-per-byte density
//! - `corpus`      — realistic Rynix source (the headline number)

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use rynix_diag::CountSink;
use rynix_lexer::Lexer;

/// 1 MiB per workload: large enough to leave L2 and amortize setup, small
/// enough that the whole suite finishes in about a minute.
const TARGET_BYTES: usize = 1 << 20;

fn repeat_to_size(unit: &str) -> String {
    let times = TARGET_BYTES / unit.len() + 1;
    unit.repeat(times)
}

fn corpus_source() -> String {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/lexer");
    let mut unit = String::new();
    for entry in std::fs::read_dir(dir).expect("read corpus dir") {
        let path = entry.expect("dir entry").path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
        if !name.ends_with(".ryx") || name.starts_with("errors") {
            continue;
        }
        unit.push_str(&std::fs::read_to_string(&path).expect("read corpus file"));
        unit.push('\n');
    }
    repeat_to_size(&unit)
}

fn lex_fully(source: &str) -> usize {
    let mut sink = CountSink::new();
    let mut lexer = Lexer::new(source, 0);
    let mut count = 0usize;
    loop {
        let token = lexer.next_token(&mut sink);
        count += 1;
        if token.is_eof() {
            return count;
        }
    }
}

fn bench_throughput(c: &mut Criterion) {
    let workloads = [
        (
            "identifiers",
            repeat_to_size("let value_counter = compute_total_for_request\n"),
        ),
        (
            "numbers",
            repeat_to_size("1 42 1_000_000 0xdead_beef 0o755 0b1010 3.141_592 6e23 2.5e-3\n"),
        ),
        (
            "strings",
            repeat_to_size("\"plain text\" \"with \\n escapes \\u{1F680} and more\"\n"),
        ),
        (
            "punctuation",
            repeat_to_size("(){}[],.::->==!=<=>=+=-=*=/=%=..=\n"),
        ),
        ("corpus", corpus_source()),
    ];

    let mut group = c.benchmark_group("lex");
    // 30 samples over 2 seconds is plenty for a deterministic, allocation-free
    // scan and keeps `cargo bench` usable as part of the edit loop.
    group.sample_size(30);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));
    for (name, source) in &workloads {
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_function(*name, |b| {
            b.iter(|| black_box(lex_fully(black_box(source))));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_throughput);
criterion_main!(benches);
