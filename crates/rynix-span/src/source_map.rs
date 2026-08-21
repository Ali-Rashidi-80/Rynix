use std::fmt;
use std::fs::File;
use std::io;
use std::path::Path;

use memmap2::Mmap;

use crate::Span;

/// Identifies a file loaded into a [`SourceMap`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FileId(u32);

impl FileId {
    /// Index into [`SourceMap::files`] iteration order.
    #[inline]
    pub fn index(self) -> u32 {
        self.0
    }
}

/// 1-based line and column. The column counts *bytes* from the line start
/// (documented in the `rynix.diag.v1` schema).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

/// The backing storage of one source file.
enum SourceText {
    /// Owned string: tests, stdin, empty files, and BOM-prefixed files.
    Owned(String),
    /// Memory-mapped file, UTF-8-validated once at load time.
    Mapped(Mmap),
}

impl SourceText {
    fn as_str(&self) -> &str {
        match self {
            SourceText::Owned(s) => s,
            // SAFETY: the mapped bytes were UTF-8-validated when the map was
            // created in `SourceMap::load_file`, and source files are assumed
            // not to be modified externally during a compilation session
            // (ADR-0003).
            SourceText::Mapped(m) => unsafe { std::str::from_utf8_unchecked(m) },
        }
    }
}

/// One loaded source file plus its precomputed line-start table.
pub struct SourceFile {
    id: FileId,
    name: String,
    start_pos: u32,
    text: SourceText,
    /// File-local byte offsets of line starts; `line_starts[0] == 0` and a
    /// new entry follows every `\n`. Used only on the cold diagnostic path.
    line_starts: Vec<u32>,
}

impl SourceFile {
    #[inline]
    pub fn id(&self) -> FileId {
        self.id
    }

    /// Display name (the path it was loaded from, or a synthetic name).
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Global offset of the first byte of this file.
    #[inline]
    pub fn start_pos(&self) -> u32 {
        self.start_pos
    }

    #[inline]
    pub fn text(&self) -> &str {
        self.text.as_str()
    }

    /// Length in bytes.
    #[inline]
    pub fn len(&self) -> u32 {
        self.text.as_str().len() as u32
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.text.as_str().is_empty()
    }

    /// Global offset one past the last byte.
    #[inline]
    pub fn end_pos(&self) -> u32 {
        self.start_pos + self.len()
    }

    /// The span covering the whole file.
    #[inline]
    pub fn span(&self) -> Span {
        Span::new(self.start_pos, self.end_pos())
    }

    /// Resolves a *file-local* byte offset to 1-based line/column.
    fn line_col_local(&self, local: u32) -> LineCol {
        debug_assert!(local <= self.len());
        // `line_starts[0] == 0`, so partition_point returns at least 1.
        let line_idx = self.line_starts.partition_point(|&s| s <= local) - 1;
        LineCol {
            line: line_idx as u32 + 1,
            col: local - self.line_starts[line_idx] + 1,
        }
    }
}

impl fmt::Debug for SourceFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SourceFile")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("start_pos", &self.start_pos)
            .field("len", &self.len())
            .finish_non_exhaustive()
    }
}

/// Owns every loaded source file and manages the global `u32` offset space
/// (ADR-0003). Files occupy contiguous windows separated by a 1-byte gap so
/// spans from different files can never touch.
#[derive(Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        SourceMap { files: Vec::new() }
    }

    /// Adds an owned string as a file (tests, stdin, generated code).
    /// A leading BOM is stripped.
    pub fn add_owned(&mut self, name: impl Into<String>, mut src: String) -> FileId {
        if src.starts_with('\u{feff}') {
            src.drain(..'\u{feff}'.len_utf8());
        }
        self.insert(name.into(), SourceText::Owned(src))
    }

    /// Loads a file from disk, memory-mapping it when possible (zero-copy).
    ///
    /// Falls back to an owned string for empty files and BOM-prefixed files.
    /// Fails with `InvalidData` if the file is not valid UTF-8.
    pub fn load_file(&mut self, path: &Path) -> io::Result<FileId> {
        let name = path.display().to_string();
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        if len == 0 {
            return Ok(self.add_owned(name, String::new()));
        }
        if len >= u64::from(u32::MAX) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{name}: file exceeds the 4 GiB source limit (ADR-0003)"),
            ));
        }
        // SAFETY: we require that source files are not modified externally
        // during a compilation session (ADR-0003). This is the standard
        // trade-off for zero-copy source access.
        let mmap = unsafe { Mmap::map(&file)? };
        match std::str::from_utf8(&mmap) {
            Err(e) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{name}: invalid UTF-8 at byte {}", e.valid_up_to()),
            )),
            Ok(s) if s.starts_with('\u{feff}') => {
                let owned = s['\u{feff}'.len_utf8()..].to_string();
                Ok(self.add_owned(name, owned))
            }
            Ok(_) => Ok(self.insert(name, SourceText::Mapped(mmap))),
        }
    }

    fn insert(&mut self, name: String, text: SourceText) -> FileId {
        let start_pos = match self.files.last() {
            // 1-byte gap: spans of adjacent files can never touch.
            Some(prev) => prev.end_pos() + 1,
            None => 0,
        };
        let len = text.as_str().len();
        assert!(
            (start_pos as u64) + (len as u64) < u64::from(u32::MAX),
            "total source exceeds the 4 GiB global span space (ADR-0003)"
        );

        let bytes = text.as_str().as_bytes();
        let mut line_starts = Vec::with_capacity(128);
        line_starts.push(0u32);
        for nl in memchr::memchr_iter(b'\n', bytes) {
            line_starts.push(nl as u32 + 1);
        }

        let id = FileId(self.files.len() as u32);
        self.files.push(SourceFile {
            id,
            name,
            start_pos,
            text,
            line_starts,
        });
        id
    }

    #[inline]
    pub fn file(&self, id: FileId) -> &SourceFile {
        &self.files[id.0 as usize]
    }

    pub fn files(&self) -> impl Iterator<Item = &SourceFile> {
        self.files.iter()
    }

    /// The file containing the global offset `pos`.
    ///
    /// Panics if no file has been loaded. Offsets in the 1-byte gap after a
    /// file resolve to that file (this is where `Eof` spans live).
    pub fn file_at_pos(&self, pos: u32) -> &SourceFile {
        assert!(!self.files.is_empty(), "SourceMap is empty");
        let idx = self.files.partition_point(|f| f.start_pos <= pos);
        debug_assert!(idx > 0);
        &self.files[idx - 1]
    }

    /// Resolves a global offset to its file and 1-based line/column.
    pub fn line_col(&self, pos: u32) -> (&SourceFile, LineCol) {
        let file = self.file_at_pos(pos);
        let local = pos.min(file.end_pos()) - file.start_pos();
        (file, file.line_col_local(local))
    }

    /// The source text covered by `span`. The span must lie within one file
    /// and on UTF-8 character boundaries (always true for lexer spans).
    pub fn span_text(&self, span: Span) -> &str {
        let file = self.file_at_pos(span.lo());
        debug_assert!(
            span.hi() <= file.end_pos(),
            "span {span:?} crosses the end of file {}",
            file.name()
        );
        let lo = (span.lo() - file.start_pos()) as usize;
        let hi = (span.hi() - file.start_pos()) as usize;
        &file.text()[lo..hi]
    }
}

impl fmt::Debug for SourceMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SourceMap")
            .field("files", &self.files)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_file_roundtrip() {
        let mut sm = SourceMap::new();
        let id = sm.add_owned("a.ryx", "let x = 1\n".to_string());
        let f = sm.file(id);
        assert_eq!(f.name(), "a.ryx");
        assert_eq!(f.start_pos(), 0);
        assert_eq!(f.len(), 10);
        assert_eq!(f.text(), "let x = 1\n");
    }

    #[test]
    fn bom_is_stripped() {
        let mut sm = SourceMap::new();
        let id = sm.add_owned("bom.ryx", "\u{feff}def".to_string());
        assert_eq!(sm.file(id).text(), "def");
    }

    #[test]
    fn multi_file_offsets_have_gaps() {
        let mut sm = SourceMap::new();
        let a = sm.add_owned("a", "abc".to_string()); // 0..3
        let b = sm.add_owned("b", "xy".to_string()); // 4..6
        assert_eq!(sm.file(a).span(), Span::new(0, 3));
        assert_eq!(sm.file(b).span(), Span::new(4, 6));
        assert_eq!(sm.file_at_pos(2).id(), a);
        assert_eq!(sm.file_at_pos(3).id(), a, "gap byte belongs to `a`");
        assert_eq!(sm.file_at_pos(4).id(), b);
        assert_eq!(sm.span_text(Span::new(4, 6)), "xy");
    }

    #[test]
    fn line_col_resolution() {
        let mut sm = SourceMap::new();
        // bytes: a=0 b=1 \n=2 c=3 d=4 e=5 \r=6 \n=7 f=8
        sm.add_owned("m", "ab\ncde\r\nf".to_string());
        let lc = |pos: u32| sm.line_col(pos).1;
        assert_eq!(lc(0), LineCol { line: 1, col: 1 });
        assert_eq!(lc(1), LineCol { line: 1, col: 2 });
        assert_eq!(lc(2), LineCol { line: 1, col: 3 }, "the newline itself");
        assert_eq!(lc(3), LineCol { line: 2, col: 1 });
        assert_eq!(lc(6), LineCol { line: 2, col: 4 }, "the \\r before \\r\\n");
        assert_eq!(lc(8), LineCol { line: 3, col: 1 });
        assert_eq!(lc(9), LineCol { line: 3, col: 2 }, "end-of-file position");
    }

    #[test]
    fn empty_file() {
        let mut sm = SourceMap::new();
        let id = sm.add_owned("empty", String::new());
        let f = sm.file(id);
        assert!(f.is_empty());
        assert_eq!(f.span(), Span::empty(0));
        assert_eq!(sm.line_col(0).1, LineCol { line: 1, col: 1 });
    }

    #[test]
    fn span_text_slicing() {
        let mut sm = SourceMap::new();
        sm.add_owned("s", "def main".to_string());
        assert_eq!(sm.span_text(Span::new(0, 3)), "def");
        assert_eq!(sm.span_text(Span::new(4, 8)), "main");
        assert_eq!(sm.span_text(Span::empty(3)), "");
    }
}
