use std::collections::{BTreeMap, HashSet};

use crate::{
    common::{NodeID, Visibility},
    error::{
        InternalError,
        context::Context,
        diagnostic::{Diagnostic, Label},
    },
    mod_tree::{
        ScopeInfo,
        scope::{Binding, Import, ScopeKind, Symbol},
    },
};

// Idea: to remove cloning, in each iteration calculate the difference that
// should be applied to the tree, and apply it in `while let Some(dif) = ...`

pub(crate) fn solve(ctx: &mut Context, name_tree: ScopeInfo) -> Result<ScopeInfo, InternalError> {
    let mut old_tree;
    let mut new_tree = name_tree.clone();
    let mut changed = true;
    while changed {
        old_tree = new_tree.clone();
        changed = false;
        for (id, scope) in new_tree.iter_mut() {
            let imports = match &scope.kind {
                ScopeKind::Module { imports, .. } => imports,
                _ => continue,
            };
            for import in imports {
                changed = changed | resolve_import(&old_tree, &mut scope.items, id, import)?;
            }
        }
    }
    for (id, scope) in new_tree.iter() {
        let imports = match &scope.kind {
            ScopeKind::Module { imports, .. } => imports,
            _ => continue,
        };
        for import in imports {
            report_import_errors(ctx, &new_tree, id, import);
        }
    }
    Ok(new_tree)
}

fn report_import_errors(ctx: &mut Context, old_tree: &ScopeInfo, id: &NodeID, import: &Import) {
    let binding: Binding = match old_tree.find_path(*id, import.path.clone(), &mut true) {
        Ok(b) => b,
        Err(diag) => {
            ctx.report(diag);
            return;
        }
    };
    let binding_id = match binding.sym {
        Symbol::Ambiguous(_) => unreachable!("find_path doesn't return ambiguous nodes"),
        Symbol::Local(node_id) | Symbol::Imported(node_id) | Symbol::GlobImported(node_id) => {
            node_id
        }
    };
    if import.is_glob {
        match old_tree.get(binding_id) {
            Some(_) => (),
            None => {
                let name = match &import.alias {
                    Some(name) => name.clone(),
                    None => import.path.try_last().unwrap().clone(),
                };
                ctx.report(Diagnostic::error(&name.pos).with_label(
                    Label::new(&name.pos).with_msg(Box::new(move || {
                        format!(
                            "cannot glob import from {}, it is not a namespace",
                            name.data
                        )
                    })),
                ));
            }
        };
    }
}

fn resolve_import(
    old_tree: &ScopeInfo,
    items: &mut BTreeMap<String, Binding>,
    id: &NodeID,
    import: &Import,
) -> Result<bool, InternalError> {
    let mut private_guard = true;
    let binding: Binding = match old_tree.find_path(*id, import.path.clone(), &mut private_guard) {
        Ok(b) => b,
        Err(_) => return Ok(false),
    };
    let mut changed = false;
    let binding_id = match binding.sym {
        Symbol::Ambiguous(_) => unreachable!("find_path doesn't return ambiguous nodes"),
        Symbol::Local(node_id) | Symbol::Imported(node_id) | Symbol::GlobImported(node_id) => {
            node_id
        }
    };
    if !import.is_glob {
        let new_binding = Binding {
            vis: import.vis,
            kind: binding.kind,
            sym: Symbol::Imported(binding_id),
        };
        let name = match &import.alias {
            Some(name) => name.data.clone(),
            None => import.path.try_last().unwrap().name_str(),
        };
        let existing_binding = match items.get_mut(&name) {
            Some(b) => b,
            None => {
                items.insert(name.clone(), new_binding);
                return Ok(true);
            }
        };
        changed = match existing_binding.sym {
            // it can shadow glob import
            Symbol::GlobImported(id) => {
                existing_binding.sym = Symbol::Imported(id);
                true
            }

            // maybe its just the same symbol
            Symbol::Local(id) | Symbol::Imported(id) if id == binding_id => changed,

            // otherwise it becomes ambiguous
            Symbol::Local(id) | Symbol::Imported(id) => {
                make_ambiguous(binding_id, existing_binding, id)
            }

            Symbol::Ambiguous(ref mut ids) => ids.insert(binding_id),
        };
        return Ok(changed);
    }
    let scope = match old_tree.get(binding_id) {
        Some(s) => s,
        None => {
            // not a namespace
            return Ok(false);
        }
    };
    for (name, binding) in &scope.items {
        if !private_guard && let Visibility::Private = binding.vis {
            continue;
        }
        let binding_id = match binding.sym {
            Symbol::Local(node_id) | Symbol::Imported(node_id) | Symbol::GlobImported(node_id) => {
                node_id
            }
            _ => continue,
        };

        let new_binding = Binding {
            vis: import.vis,
            kind: binding.kind,
            sym: Symbol::GlobImported(binding_id),
        };
        let existing_binding = match items.get_mut(name) {
            Some(b) => b,
            None => {
                items.insert(name.clone(), new_binding);
                changed = true;
                continue;
            }
        };
        changed = match existing_binding.sym {
            // if its local or exact-imported, glob import cant shadow it
            Symbol::Local(_) | Symbol::Imported(_) => changed,

            // if its the same import, whatever
            Symbol::GlobImported(id) if id == binding_id => changed,

            // otherwise it becomes ambiguous
            Symbol::GlobImported(id) => make_ambiguous(binding_id, existing_binding, id),
            Symbol::Ambiguous(ref mut ids) => ids.insert(binding_id),
        };
    }
    Ok(changed)
}

fn make_ambiguous(binding_id: NodeID, existing_binding: &mut Binding, id: NodeID) -> bool {
    let mut set = HashSet::new();
    set.insert(id);
    set.insert(binding_id);
    existing_binding.sym = Symbol::Ambiguous(set);
    true
}
