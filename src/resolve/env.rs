use std::collections::{BTreeSet, HashMap};

use crate::{
    common::{NodeID, Path},
    error::{InternalError, context::Context, diagnostic::Diagnostic},
    mod_tree::{ScopeInfo, scope::Symbol},
    parser::ast::{RTypeData, RTypeNode},
    resolve::ast::SymRef,
    symtable::{SymInfo, SymTable, TypeInfo},
    tp::{TVar, Type},
};

pub struct Env {
    pub current_module: NodeID,
    scope_info: ScopeInfo,
    node_tvar_map: HashMap<NodeID, TVar>,
    node_map: HashMap<NodeID, SymInfo>,
    tvar_map: HashMap<TVar, TypeInfo>,
    local_scopes: Vec<BTreeSet<String>>,
}

impl Env {
    pub(crate) fn new_scope(&mut self) {
        self.local_scopes.push(BTreeSet::new())
    }

    pub(crate) fn resolve_type(
        &self,
        ctx: &mut Context,
        ret_type: RTypeNode,
    ) -> Result<Type, InternalError> {
        Ok(match ret_type.data {
            RTypeData::Var(path) => {
                let sym_ref = match self.find_symbol(path.clone()) {
                    Ok(sym) => sym,
                    Err(diag) => {
                        ctx.report(diag);
                        return Ok(Type::unknown());
                    }
                };
                match sym_ref {
                    SymRef::Local(_) => panic!("local type definitons not supported"),
                    SymRef::Global(id) => {
                        let tv = self.get_tvar(id)?;
                        let name = path.to_string();
                        Type::named_var(tv.clone(), &name)
                    }
                }
            }
            RTypeData::Fun(args, ret) => {
                let args = args
                    .into_iter()
                    .map(|arg| self.resolve_type(ctx, arg))
                    .collect::<Result<_, _>>()?;
                let ret = self.resolve_type(ctx, *ret)?;
                Type::fun(args, ret)
            }
            RTypeData::Ptr(tp) => Type::ptr(self.resolve_type(ctx, *tp)?),
            RTypeData::MutPtr(tp) => Type::mut_ptr(self.resolve_type(ctx, *tp)?),
            RTypeData::Tuple(tps) => {
                let tps = tps
                    .into_iter()
                    .map(|tp| self.resolve_type(ctx, tp))
                    .collect::<Result<_, _>>()?;
                Type::tuple(tps)
            }
            RTypeData::Array(size, tp) => {
                let tp = self.resolve_type(ctx, *tp)?;
                Type::array(size, tp)
            }
            RTypeData::Slice(tp) => {
                let tp = self.resolve_type(ctx, *tp)?;
                Type::ptr(tp)
            }
            RTypeData::MutSlice(tp) => {
                let tp = self.resolve_type(ctx, *tp)?;
                Type::mut_ptr(tp)
            }
            RTypeData::TypeApp(path, rtype_nodes) => todo!(),
        })
    }

    pub(crate) fn leave_scope(&mut self) {
        match self.local_scopes.pop() {
            Some(_) => (),
            None => panic!("attempted to leave without any local scope"),
        }
    }

    pub(crate) fn find_symbol(&self, path: Path) -> Result<SymRef, Diagnostic> {
        if let Some(id) = path.clone().if_single() {
            let str = id.name_str();
            for scope in self.local_scopes.iter().rev() {
                if scope.contains(&str) {
                    return Ok(SymRef::Local(str));
                }
            }
        };
        let binding = self
            .scope_info
            .find_path(self.current_module, path.clone(), &mut true)?;
        let id = match binding.sym {
            Symbol::Local(node_id) | Symbol::Imported(node_id) | Symbol::GlobImported(node_id) => {
                node_id
            }
            Symbol::Ambiguous(_) => unreachable!("find_path doesn't return ambiguous nodes"),
        };
        Ok(SymRef::Global(id))
    }

    pub(crate) fn init(scope_info: ScopeInfo, node_tvar_map: HashMap<NodeID, TVar>) -> Self {
        Self {
            current_module: NodeID::of_root(),
            scope_info,
            node_tvar_map,
            local_scopes: vec![],
            node_map: HashMap::new(),
            tvar_map: HashMap::new(),
        }
    }

    pub(crate) fn add_local(&mut self, name: String) {
        self.local_scopes
            .last_mut()
            .expect("cant add without a local scope")
            .insert(name);
    }

    pub fn finish(self) -> SymTable {
        SymTable::init(self.node_map, self.tvar_map)
    }

    pub(crate) fn add_sym_info(&mut self, id: NodeID, sym_info: SymInfo) {
        self.node_map.insert(id, sym_info);
    }

    pub(crate) fn get_tvar(&self, id: NodeID) -> Result<TVar, InternalError> {
        self.node_tvar_map
            .get(&id)
            .cloned()
            .ok_or(InternalError::AnyMsg(format!(
                "cant get tvar info for {:#?}",
                id
            )))
    }

    pub(crate) fn add_type_info(&mut self, id: TVar, type_info: crate::symtable::TypeInfo) {
        self.tvar_map.insert(id, type_info);
    }
}
