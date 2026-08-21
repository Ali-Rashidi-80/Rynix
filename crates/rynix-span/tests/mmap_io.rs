//! Integration tests for the disk-loading (mmap) path of `SourceMap`.

use std::fs;
use std::path::PathBuf;

use rynix_span::SourceMap;

/// Creates a unique temp file with the given bytes and returns its path.
fn temp_file(tag: &str, bytes: &[u8]) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("rynix_span_test_{}_{tag}.ryx", std::process::id()));
    fs::write(&path, bytes).expect("write temp file");
    path
}

#[test]
fn loads_and_maps_a_regular_file() {
    let path = temp_file("regular", b"def main()\nend\n");
    let mut sm = SourceMap::new();
    let id = sm.load_file(&path).expect("load");
    let f = sm.file(id);
    assert_eq!(f.text(), "def main()\nend\n");
    assert_eq!(f.len(), 15);
    // Line table works over the mapped bytes.
    let (_, lc) = sm.line_col(f.start_pos() + 11);
    assert_eq!((lc.line, lc.col), (2, 1));
    let _ = fs::remove_file(&path);
}

#[test]
fn empty_file_falls_back_to_owned() {
    let path = temp_file("empty", b"");
    let mut sm = SourceMap::new();
    let id = sm.load_file(&path).expect("load");
    assert!(sm.file(id).is_empty());
    let _ = fs::remove_file(&path);
}

#[test]
fn bom_file_is_stripped() {
    let path = temp_file("bom", b"\xEF\xBB\xBFlet x = 1");
    let mut sm = SourceMap::new();
    let id = sm.load_file(&path).expect("load");
    assert_eq!(sm.file(id).text(), "let x = 1");
    let _ = fs::remove_file(&path);
}

#[test]
fn invalid_utf8_is_rejected_with_offset() {
    let path = temp_file("bad_utf8", b"ok \xFF bad");
    let mut sm = SourceMap::new();
    let err = sm.load_file(&path).expect_err("must fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("invalid UTF-8 at byte 3"), "{err}");
    let _ = fs::remove_file(&path);
}

#[test]
fn missing_file_is_an_io_error() {
    let mut sm = SourceMap::new();
    let err = sm
        .load_file(std::path::Path::new("definitely/not/here.ryx"))
        .expect_err("must fail");
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}
