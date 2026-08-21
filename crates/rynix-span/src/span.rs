use std::fmt;

/// A half-open byte range `[lo, hi)` into the global source space of a
/// [`SourceMap`](crate::SourceMap).
///
/// Spans address a *global* `u32` offset space: every loaded file occupies a
/// contiguous, non-overlapping window (separated by a 1-byte gap), so a bare
/// `Span` uniquely identifies both the file and the position within it.
/// Total source per session is capped at 4 GiB (ADR-0003).
///
/// Spans produced by the lexer always lie on UTF-8 character boundaries.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Span {
    lo: u32,
    hi: u32,
}

impl Span {
    /// Creates a span covering `[lo, hi)`.
    #[inline]
    pub fn new(lo: u32, hi: u32) -> Self {
        debug_assert!(lo <= hi, "span lo ({lo}) must not exceed hi ({hi})");
        Span { lo, hi }
    }

    /// An empty span anchored at `pos` (used for `Eof` and pure insertions).
    #[inline]
    pub fn empty(pos: u32) -> Self {
        Span { lo: pos, hi: pos }
    }

    #[inline]
    pub fn lo(self) -> u32 {
        self.lo
    }

    #[inline]
    pub fn hi(self) -> u32 {
        self.hi
    }

    /// Length in bytes.
    #[inline]
    pub fn len(self) -> u32 {
        self.hi - self.lo
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.lo == self.hi
    }

    /// The smallest span covering both `self` and `other`.
    #[inline]
    #[must_use]
    pub fn to(self, other: Span) -> Span {
        Span::new(self.lo.min(other.lo), self.hi.max(other.hi))
    }

    /// Whether `pos` falls inside the half-open range.
    #[inline]
    pub fn contains(self, pos: u32) -> bool {
        self.lo <= pos && pos < self.hi
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.lo, self.hi)
    }
}

#[cfg(test)]
mod tests {
    use super::Span;

    #[test]
    fn basic_accessors() {
        let s = Span::new(3, 7);
        assert_eq!(s.lo(), 3);
        assert_eq!(s.hi(), 7);
        assert_eq!(s.len(), 4);
        assert!(!s.is_empty());
        assert_eq!(format!("{s:?}"), "3..7");
    }

    #[test]
    fn empty_span() {
        let s = Span::empty(5);
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        assert!(!s.contains(5), "half-open: empty span contains nothing");
    }

    #[test]
    fn join_and_contains() {
        let a = Span::new(2, 4);
        let b = Span::new(8, 10);
        let joined = a.to(b);
        assert_eq!(joined, Span::new(2, 10));
        assert_eq!(b.to(a), joined, "join is symmetric");
        assert!(joined.contains(2));
        assert!(joined.contains(9));
        assert!(!joined.contains(10), "hi is exclusive");
    }
}
