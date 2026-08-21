//! Fuzzes the diagnostic pipeline: every emitted diagnostic must render as
//! valid JSON, and every fix must be mechanically applicable.
//!
//! `cargo +nightly fuzz run lex_diagnostics`

#![no_main]

use libfuzzer_sys::fuzz_target;
use rynix_diag::{render_human, render_json, VecSink};
use rynix_lexer::lex_all;
use rynix_span::SourceMap;

fuzz_target!(|data: &str| {
    let mut sm = SourceMap::new();
    let id = sm.add_owned("fuzz.ryx", data.to_string());
    let file = sm.file(id);
    let text = file.text().to_string();
    let base = file.start_pos();

    let mut sink = VecSink::new();
    lex_all(&text, base, &mut sink);

    for diag in &sink.diags {
        // Both renderers must survive any span the lexer can produce.
        let json = render_json(diag, &sm);
        assert!(!json.contains('\n'), "NDJSON must stay on one line");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed["schema"], "rynix.diag.v1");
        let _ = render_human(diag, &sm);

        // Fixes must be applicable: in-bounds, on character boundaries, and
        // producing valid UTF-8 source.
        for fix in &diag.fixes {
            assert!((0.0..=1.0).contains(&fix.confidence));
            for edit in &fix.edits {
                let lo = (edit.span.lo() - base) as usize;
                let hi = (edit.span.hi() - base) as usize;
                assert!(hi <= text.len());
                assert!(text.is_char_boundary(lo) && text.is_char_boundary(hi));
                let mut patched = String::with_capacity(text.len());
                patched.push_str(&text[..lo]);
                patched.push_str(&edit.replacement);
                patched.push_str(&text[hi..]);
                // The patched source must itself lex cleanly (totally).
                let mut patched_sink = VecSink::new();
                lex_all(&patched, 0, &mut patched_sink);
            }
        }
    }
});
