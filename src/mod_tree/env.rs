use std::collections::BTreeMap;

use crate::{
    common::{Ident, NodeID},
    error::{InternalError, context::Context, diagnostic::Diagnostic},
    mod_tree::{
        error,
        scope::{Binding, Import, Scope, ScopeKind},
    },
};

use super::ScopeInfo;
use super::in_a;

#[derive(Debug)]
pub(super) struct Env {
    current_namespace_id: NodeID,
    current_namespace_path: Vec<String>,
    scope_info: ScopeInfo,
    file_map: BTreeMap<Vec<String>, in_a::Module>,
}

impl Env {
    pub fn init(mod_map: BTreeMap<Vec<String>, in_a::Module>) -> Self {
        let mut name_tree = ScopeInfo::new();
        let root_info = Scope {
            items: BTreeMap::new(),
            kind: ScopeKind::Root,
        };
        name_tree.insert(NodeID::of_root(), root_info);
        Self {
            current_namespace_id: NodeID::of_root(),
            current_namespace_path: vec![],
            scope_info: name_tree,
            file_map: mod_map,
        }
    }

    pub fn enter(&mut self, name: String) {
        let mod_info = self.scope_info.get(self.current_namespace_id).unwrap();
        let binding = mod_info.items.get(&name).unwrap();
        match binding.kind {
            _ => match &binding.sym {
                super::scope::Symbol::Local(node_id) => {
                    self.current_namespace_id = node_id.clone();
                    self.current_namespace_path.push(name);
                }
                _ => unreachable!(),
            },
        }
    }

    pub fn leave(&mut self) {
        let mod_info = self.scope_info.get(self.current_namespace_id).unwrap();

        let id = match mod_info.kind {
            ScopeKind::Root => panic!("cannot leave from root"),
            ScopeKind::Module { parent, .. } | ScopeKind::Enum { parent, .. } => parent,
        };

        self.current_namespace_id = id;
        self.current_namespace_path.pop();
    }

    pub fn remove_module(&mut self, path: &Vec<String>) -> Option<in_a::Module> {
        self.file_map.remove(path)
    }

    pub fn add_item(&mut self, name: Ident, binding: Binding) -> Result<(), Diagnostic> {
        let name_s = name.name_str();
        assert_ne!(name_s, "super");
        assert_ne!(name_s, "self");
        assert_ne!(name_s, "Self");
        let mod_info = self.scope_info.get_mut(self.current_namespace_id).unwrap();
        match mod_info.items.get_mut(&name_s) {
            Some(bind) => return Err(error::already_bound(&name.pos, name_s)),
            None => {
                mod_info.items.insert(name_s, binding);
            }
        }
        Ok(())
    }

    pub(crate) fn solve_imports(self, ctx: &mut Context) -> Result<ScopeInfo, InternalError> {
        super::import_solve::solve(ctx, self.scope_info)
    }

    pub(crate) fn get_current_module_id(&self) -> NodeID {
        self.current_namespace_id
    }

    pub(crate) fn add_mod_info(&mut self, id: NodeID, mod_info: Scope) {
        self.scope_info.insert(id, mod_info);
    }

    pub(crate) fn get_current_name_path(&self) -> &Vec<String> {
        &self.current_namespace_path
    }

    pub(crate) fn add_import(&mut self, import: Import) {
        let mod_info = self.scope_info.get_mut(self.current_namespace_id).unwrap();
        match mod_info.kind {
            ScopeKind::Module {
                ref mut imports, ..
            } => imports.push(import),
            _ => panic!(),
        }
    }
}
