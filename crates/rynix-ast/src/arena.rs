use std::cell::Cell;

use bumpalo::Bump;

/// Dense handle for an AST node. Used by later phases to hang `SoA` side tables
/// (types, defs, escape facts) off the tree without touching the nodes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct NodeId(u32);

impl NodeId {
    #[inline]
    pub fn index(self) -> u32 {
        self.0
    }
}

/// Bump arena that owns every AST node for one compilation unit.
///
/// Allocation never returns to the OS until the arena is dropped; this is the
/// deliberate Phase-2 trade-off that lets the parser stay allocation-free on
/// the hot path *relative to the system allocator*.
pub struct AstArena {
    bump: Bump,
    next_id: Cell<u32>,
}

impl Default for AstArena {
    fn default() -> Self {
        Self::new()
    }
}

impl AstArena {
    pub fn new() -> Self {
        Self {
            bump: Bump::new(),
            next_id: Cell::new(0),
        }
    }

    /// Allocates the next dense [`NodeId`].
    #[inline]
    pub fn next_id(&self) -> NodeId {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        NodeId(id)
    }

    /// Number of node ids issued so far.
    #[inline]
    pub fn node_count(&self) -> u32 {
        self.next_id.get()
    }

    /// Allocates a single value in the arena and returns a stable reference.
    #[inline]
    pub fn alloc<T>(&self, value: T) -> &T {
        self.bump.alloc(value)
    }

    /// Allocates a slice by moving out of a `Vec` (the `Vec` itself is freed).
    #[inline]
    pub fn alloc_slice<T>(&self, values: Vec<T>) -> &[T] {
        self.bump.alloc_slice_fill_iter(values)
    }

    /// Allocates a copy of an existing slice of `Copy` values.
    #[inline]
    pub fn alloc_slice_copy<T: Copy>(&self, values: &[T]) -> &[T] {
        self.bump.alloc_slice_copy(values)
    }
}
