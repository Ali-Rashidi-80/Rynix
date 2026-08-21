use rustc_hash::FxHashMap;
use rynix_span::Symbol;

use crate::def::DefId;

/// Dense handle for a scope node.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct ScopeId(u32);

impl ScopeId {
    #[inline]
    pub fn index(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScopeKind {
    Module,
    Fn,
    Block,
    Loop,
}

#[derive(Debug)]
pub struct Scope {
    pub parent: Option<ScopeId>,
    pub kind: ScopeKind,
    /// Name → definition in this scope only.
    pub bindings: FxHashMap<Symbol, DefId>,
}

/// Parent-linked scope tree.
#[derive(Debug, Default)]
pub struct ScopeTree {
    scopes: Vec<Scope>,
}

impl ScopeTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alloc(&mut self, parent: Option<ScopeId>, kind: ScopeKind) -> ScopeId {
        let id = ScopeId(self.scopes.len() as u32);
        self.scopes.push(Scope {
            parent,
            kind,
            bindings: FxHashMap::default(),
        });
        id
    }

    pub fn get(&self, id: ScopeId) -> &Scope {
        &self.scopes[id.0 as usize]
    }

    pub fn get_mut(&mut self, id: ScopeId) -> &mut Scope {
        &mut self.scopes[id.0 as usize]
    }

    /// Define `name` in `scope`. Returns the previous binding if this shadows
    /// or duplicates within the same scope.
    pub fn define(&mut self, scope: ScopeId, name: Symbol, def: DefId) -> Option<DefId> {
        self.get_mut(scope).bindings.insert(name, def)
    }

    /// Lexical lookup: walk parents until a binding is found.
    pub fn lookup(&self, scope: ScopeId, name: Symbol) -> Option<DefId> {
        let mut cur = Some(scope);
        while let Some(id) = cur {
            let scope = self.get(id);
            if let Some(&def) = scope.bindings.get(&name) {
                return Some(def);
            }
            cur = scope.parent;
        }
        None
    }

    /// Whether `scope` is inside a loop (for break/continue).
    pub fn in_loop(&self, scope: ScopeId) -> bool {
        let mut cur = Some(scope);
        while let Some(id) = cur {
            let scope = self.get(id);
            if scope.kind == ScopeKind::Loop {
                return true;
            }
            cur = scope.parent;
        }
        false
    }
}
