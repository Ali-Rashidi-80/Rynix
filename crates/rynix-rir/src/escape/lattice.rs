//! Escape lattice and storage placement.

use std::cmp::Ordering;
use std::fmt;

/// Per-allocation-site escape lattice (ordered).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Escape {
    #[default]
    NoEscape = 0,
    ArgEscape = 1,
    RegionEscape = 2,
    GlobalEscape = 3,
}

impl Escape {
    pub fn as_str(self) -> &'static str {
        match self {
            Escape::NoEscape => "NoEscape",
            Escape::ArgEscape => "ArgEscape",
            Escape::RegionEscape => "RegionEscape",
            Escape::GlobalEscape => "GlobalEscape",
        }
    }

    #[must_use]
    pub fn join(self, other: Self) -> Self {
        if self as u8 >= other as u8 {
            self
        } else {
            other
        }
    }
}

impl PartialOrd for Escape {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Escape {
    fn cmp(&self, other: &Self) -> Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

impl fmt::Display for Escape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Concrete storage after escape analysis.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Placement {
    Stack,
    /// Implicit bump arena; `id` is function-local region index.
    Region(u32),
    Heap,
}

impl Placement {
    pub fn as_str(self) -> &'static str {
        match self {
            Placement::Stack => "stack",
            Placement::Region(_) => "region",
            Placement::Heap => "heap",
        }
    }

    pub fn from_escape(escape: Escape, region_id: u32) -> Self {
        match escape {
            Escape::NoEscape => Placement::Stack,
            Escape::ArgEscape | Escape::RegionEscape => Placement::Region(region_id),
            Escape::GlobalEscape => Placement::Heap,
        }
    }
}

impl fmt::Display for Placement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Placement::Stack => f.write_str("stack"),
            Placement::Region(id) => write!(f, "region{id}"),
            Placement::Heap => f.write_str("heap"),
        }
    }
}
