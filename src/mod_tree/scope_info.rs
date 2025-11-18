use std::collections::HashMap;

use super::scope::Scope;
use crate::{
    common::{NodeID, Path, Visibility},
    error::diagnostic::Diagnostic,
    mod_tree::{
        error,
        scope::{Binding, Kind, Symbol},
    },
};

#[derive(Debug, Clone)]
pub struct ScopeInfo {
    data: HashMap<NodeID, Scope>,
}

impl ScopeInfo {
    /// Create a new scope context.
    pub(crate) fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Insert a new scope.
    pub(crate) fn insert(&mut self, scope_id: NodeID, scope: Scope) {
        self.data.insert(scope_id, scope);
    }

    /// Get scope with `scope_id`.
    pub(crate) fn get(&self, scope_id: NodeID) -> Option<&Scope> {
        self.data.get(&scope_id)
    }

    /// Get scope mutably with `scope_id`.
    pub(crate) fn get_mut(&mut self, mod_id: NodeID) -> Option<&mut Scope> {
        self.data.get_mut(&mod_id)
    }

    /// Returns binding of variable in scope with `scope_id`.
    ///
    /// It will never return ambiguous bindings, returning `Diagnostic` instead.
    ///
    /// Private guard will allow for one private access in the path,
    /// used for accessing items in the same module or parent.
    pub fn find_path(
        &self,
        scope_id: NodeID,
        mut path: Path,
        private_guard: &mut bool,
    ) -> Result<Binding, Diagnostic> {
        let name = path.pop_front_inplace().unwrap();
        let namespace = self.get(scope_id).unwrap();
        match namespace.items.get(&name.name_str()) {
            Some(binding) => {
                if let Visibility::Private = binding.vis
                    && !*private_guard
                {
                    return Err(error::private_item(&name.pos, name.data));
                }
                if path.data.is_empty() {
                    if let Symbol::Ambiguous(_) = binding.sym {
                        return Err(error::ambiguous_symbol(&name.pos, name.data));
                    }
                    return Ok(binding.clone());
                }
                let mut private_guard = false;
                let id = match binding.kind {
                    Kind::Module => match &binding.sym {
                        Symbol::Local(node_id)
                        | Symbol::Imported(node_id)
                        | Symbol::GlobImported(node_id) => node_id,
                        Symbol::Ambiguous(_) => {
                            return Err(error::ambiguous_symbol(&name.pos, name.data));
                        }
                    },
                    Kind::Struct | Kind::Enum => match &binding.sym {
                        Symbol::Local(node_id)
                        | Symbol::Imported(node_id)
                        | Symbol::GlobImported(node_id) => {
                            private_guard = true;
                            node_id
                        }
                        Symbol::Ambiguous(_) => {
                            return Err(error::ambiguous_symbol(&name.pos, name.data));
                        }
                    },
                    Kind::Func => {
                        return Err(error::cannot_import_from(&name.pos, name.data.clone())
                            .with_note(format!("{} is a function", name.data)));
                    }
                    Kind::Cons => {
                        return Err(error::cannot_import_from(&name.pos, name.data.clone())
                            .with_note(format!("{} is an enum constructor", name.data)));
                    }
                    Kind::BuiltinType => unreachable!(),
                };
                self.find_path(*id, path, &mut private_guard)
            }
            None => {
                if *private_guard {
                    match name.name_str().as_str() {
                        "super" => {
                            let parent = match namespace.parent() {
                                Some(parent) => parent,
                                None => panic!(),
                            };
                            if path.data.is_empty() {
                                let binding = Binding {
                                    vis: Visibility::Private,
                                    kind: Kind::Module,
                                    sym: Symbol::Imported(parent),
                                };
                                return Ok(binding);
                            }
                            self.find_path(parent, path, private_guard)
                        }
                        _ => {
                            // TODO: its wrong
                            path.push_front_inplace(name);
                            self.find_path(NodeID::of_root(), path, &mut false)
                        }
                    }
                } else {
                    Err(error::unbound_variable(&name.pos, name.data))
                }
            }
        }
    }

    /// Iterate all scopes mutably.
    pub(crate) fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<'_, NodeID, Scope> {
        self.data.iter_mut()
    }

    /// Iterate all scopes.
    pub(crate) fn iter(&self) -> std::collections::hash_map::Iter<'_, NodeID, Scope> {
        self.data.iter()
    }
}
