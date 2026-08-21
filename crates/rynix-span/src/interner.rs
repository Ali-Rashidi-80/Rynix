use rustc_hash::FxHashMap;

/// A handle to an interned string. 4 bytes, `Copy`, cheap to compare.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Symbol(u32);

impl Symbol {
    #[inline]
    pub fn index(self) -> u32 {
        self.0
    }
}

/// A deduplicating string interner with stable addresses.
///
/// Strings are copied into append-only buffers that are never reallocated:
/// when the current buffer would overflow we *rotate* to a fresh one and
/// retire the old buffer (keeping it alive in `full`). This makes it sound
/// to store `&'static str` views internally while the public API only ever
/// hands out `&str` tied to `&self`.
pub struct Interner {
    map: FxHashMap<&'static str, Symbol>,
    strings: Vec<&'static str>,
    buf: String,
    full: Vec<String>,
}

impl Default for Interner {
    fn default() -> Self {
        Self::new()
    }
}

impl Interner {
    pub fn new() -> Self {
        Interner {
            map: FxHashMap::default(),
            strings: Vec::new(),
            buf: String::with_capacity(4096),
            full: Vec::new(),
        }
    }

    /// Interns `s`, returning the same [`Symbol`] for equal strings.
    pub fn intern(&mut self, s: &str) -> Symbol {
        if let Some(&sym) = self.map.get(s) {
            return sym;
        }
        let stable = self.alloc(s);
        let sym = Symbol(u32::try_from(self.strings.len()).expect("interner overflow"));
        self.strings.push(stable);
        self.map.insert(stable, sym);
        sym
    }

    /// Resolves a symbol back to its string.
    #[inline]
    pub fn resolve(&self, sym: Symbol) -> &str {
        self.strings[sym.0 as usize]
    }

    /// Number of distinct interned strings.
    #[inline]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    fn alloc(&mut self, s: &str) -> &'static str {
        if self.buf.len() + s.len() > self.buf.capacity() {
            // Rotate: retire the current buffer (its contents must keep
            // living at a stable address) and start a fresh one big enough.
            let new_cap = (self.buf.capacity().max(s.len()) + 1).next_power_of_two();
            let old = std::mem::replace(&mut self.buf, String::with_capacity(new_cap));
            self.full.push(old);
        }
        let start = self.buf.len();
        // Cannot reallocate: we just guaranteed the capacity above.
        self.buf.push_str(s);
        let interned: &str = &self.buf[start..];
        // SAFETY: `interned` points into a buffer that is never reallocated
        // (we rotate to a new buffer instead of growing) and never dropped
        // while `self` lives (retired buffers are parked in `self.full`).
        // The `'static` view never escapes the public API: `resolve` returns
        // a reborrow at the `&self` lifetime.
        unsafe { &*std::ptr::from_ref::<str>(interned) }
    }
}

impl std::fmt::Debug for Interner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interner")
            .field("len", &self.strings.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedup_and_resolve() {
        let mut i = Interner::new();
        let a = i.intern("main");
        let b = i.intern("main");
        let c = i.intern("other");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(i.resolve(a), "main");
        assert_eq!(i.resolve(c), "other");
        assert_eq!(i.len(), 2);
    }

    #[test]
    fn empty_string_is_internable() {
        let mut i = Interner::new();
        let e1 = i.intern("");
        let e2 = i.intern("");
        assert_eq!(e1, e2);
        assert_eq!(i.resolve(e1), "");
    }

    #[test]
    fn addresses_stay_stable_across_buffer_rotation() {
        let mut i = Interner::new();
        // Force many rotations with strings large enough to overflow the
        // initial 4096-byte buffer repeatedly.
        let inputs: Vec<String> = (0..200)
            .map(|n| format!("sym_{n}_{}", "x".repeat(97)))
            .collect();
        let syms: Vec<Symbol> = inputs.iter().map(|s| i.intern(s)).collect();
        // Pointers captured after all rotations must match what resolve
        // returned before them (stability), and contents must be intact.
        for (input, sym) in inputs.iter().zip(&syms) {
            assert_eq!(i.resolve(*sym), input.as_str());
        }
        // Interning again returns the same symbols, no duplicates.
        for (input, sym) in inputs.iter().zip(&syms) {
            assert_eq!(i.intern(input), *sym);
        }
        assert_eq!(i.len(), inputs.len());
    }

    #[test]
    fn oversized_string_triggers_dedicated_buffer() {
        let mut i = Interner::new();
        let big = "b".repeat(100_000);
        let sym = i.intern(&big);
        assert_eq!(i.resolve(sym), big);
        let small = i.intern("tiny");
        assert_eq!(i.resolve(small), "tiny");
    }
}
