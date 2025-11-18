use std::collections::{BTreeMap, HashSet};

use crate::common::{Ident, NodeID, Path, Visibility};

#[derive(Debug, Clone)]
pub struct Scope {
    pub items: BTreeMap<String, Binding>,
    pub kind: ScopeKind,
}

impl Scope {
    /// Return the parent (if it exsists) of scope.
    ///
    /// The only scope without the parent is root.
    pub fn parent(&self) -> Option<NodeID> {
        match self.kind {
            ScopeKind::Root => None,
            ScopeKind::Module { parent, .. }
            | ScopeKind::Struct { parent, .. }
            | ScopeKind::Enum { parent, .. } => Some(parent),
        }
    }
}

/// The kind of scope.
#[derive(Debug, Clone)]
pub enum ScopeKind {
    Root,
    Module {
        imports: Vec<Import>,
        parent: NodeID,
    },
    Enum {
        parent: NodeID,
    },
    Struct {
        parent: NodeID,
    },
}

#[derive(Debug, Clone)]
pub struct Import {
    pub path: Path,
    pub alias: Option<Ident>,
    pub is_glob: bool,
    pub vis: Visibility,
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub vis: Visibility,
    pub kind: Kind,
    pub sym: Symbol,
}

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Module,
    Func,
    Struct,
    Enum,
    Cons,
    BuiltinType,
}

#[derive(Debug, Clone)]
pub enum Symbol {
    Local(NodeID),
    Imported(NodeID),
    GlobImported(NodeID),
    Ambiguous(HashSet<NodeID>),
}
