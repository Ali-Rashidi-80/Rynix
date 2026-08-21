//! Proof of the zero-allocation guarantee (ADR-0004).
//!
//! A counting global allocator is installed for this test binary. While the
//! counter is armed we lex a large clean corpus and assert that the lexer
//! performs *zero* heap allocations: tokens are spans into the source, the
//! keyword table is static, and nothing is buffered.
//!
//! This file contains a single test on purpose: the counter is process-wide,
//! so a second test running concurrently would produce false positives.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rynix_diag::CountSink;
use rynix_lexer::{Lexer, TokenKind};

struct CountingAlloc;

static ARMED: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ARMED.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if ARMED.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ARMED.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

/// Builds a large clean corpus (no diagnostics) from the committed test data.
fn corpus() -> String {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/lexer");
    let mut sources = Vec::new();
    for entry in std::fs::read_dir(dir).expect("read corpus dir") {
        let path = entry.expect("dir entry").path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
        // Skip the deliberately broken files: constructing diagnostics
        // allocates (cold path), which is expected and allowed.
        if !name.ends_with(".ryx") || name.starts_with("errors") {
            continue;
        }
        sources.push(std::fs::read_to_string(&path).expect("read corpus file"));
    }
    assert!(!sources.is_empty(), "corpus is empty");
    sources.join("\n").repeat(200)
}

#[test]
fn lexing_a_clean_corpus_allocates_nothing() {
    let source = corpus();
    assert!(source.len() > 500_000, "corpus too small to be meaningful");
    let mut sink = CountSink::new();

    // Warm up outside the measured region so nothing lazily initializes
    // inside it.
    let mut warmup = Lexer::new(&source[..64], 0);
    while !warmup.next_token(&mut sink).is_eof() {}

    let mut lexer = Lexer::new(&source, 0);
    let mut tokens = 0usize;
    let mut idents = 0usize;

    ARMED.store(true, Ordering::SeqCst);
    let before = ALLOCATIONS.load(Ordering::SeqCst);
    loop {
        let token = lexer.next_token(&mut sink);
        tokens += 1;
        if token.kind == TokenKind::Ident {
            idents += 1;
        }
        if token.is_eof() {
            break;
        }
    }
    let after = ALLOCATIONS.load(Ordering::SeqCst);
    ARMED.store(false, Ordering::SeqCst);

    assert_eq!(
        after - before,
        0,
        "lexer allocated {} times while lexing {} bytes",
        after - before,
        source.len()
    );
    assert_eq!(sink.total(), 0, "clean corpus produced diagnostics");
    assert!(tokens > 100_000, "only {tokens} tokens lexed");
    assert!(idents > 0);
}
