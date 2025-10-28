use std::{
    collections::{BTreeSet, HashMap},
    hash::Hash,
};

use crate::{
    common::{NodeID, Path},
    error::{InternalError, context::Context, diagnostic::Diagnostic},
    mod_tree::{ScopeInfo, scope::Symbol},
    parser::ast::{RTypeData, RTypeNode},
    resolve::{ast::SymRef, error},
    symtable::{SymInfo, SymTable, TypeInfo},
    tp::{TVar, TVarKind, Type},
};

pub struct Env {
    pub current_module: NodeID,
    scope_info: ScopeInfo,
    node_tvar_map: HashMap<NodeID, TVar>,
    node_map: HashMap<NodeID, SymInfo>,
    tvar_map: HashMap<TVar, TypeInfo>,
    local_scopes: Vec<HashMap<String, LocalBinding>>,
}

enum LocalBinding {
    Var,
    TypeVar(TVar),
}

impl Env {
    pub(crate) fn new_scope(&mut self) {
        self.local_scopes.push(HashMap::new())
    }

    pub(crate) fn resolve_type(
        &self,
        ctx: &mut Context,
        tp: RTypeNode,
    ) -> Result<Type, InternalError> {
        Ok(match tp.data {
            RTypeData::Var(path) => {
                let sym_ref = match self.find_symbol(path.clone()) {
                    Ok(sym) => sym,
                    Err(diag) => {
                        ctx.report(diag);
                        return Ok(Type::unknown());
                    }
                };
                let tv = match sym_ref {
                    SymRef::Local(s) => {
                        let binding = match self.find_local_var_kind(s) {
                            Some(b) => b,
                            None => todo!(),
                        };
                        match binding {
                            LocalBinding::Var => {
                                ctx.report(error::expected_type_got_var(&tp.pos));
                                return Ok(Type::unknown());
                            }
                            LocalBinding::TypeVar(tvar) => *tvar,
                        }
                    }
                    SymRef::Global(id) => self.get_tvar(id)?,
                };
                let name = path.to_string();
                match Type::named_var(tv.clone(), &name, &path.try_last().unwrap().pos) {
                    Ok(tp) => tp,
                    Err(diag) => {
                        ctx.report(diag);
                        return Ok(Type::unknown());
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
            RTypeData::TypeApp(path, tps) => {
                let tps = tps
                    .into_iter()
                    .map(|tp| self.resolve_type(ctx, tp))
                    .collect::<Result<_, _>>()?;
                let sym_ref = match self.find_symbol(path.clone()) {
                    Ok(sym) => sym,
                    Err(diag) => {
                        ctx.report(diag);
                        return Ok(Type::unknown());
                    }
                };
                let tv = match sym_ref {
                    SymRef::Local(s) => {
                        let binding = match self.find_local_var_kind(s) {
                            Some(b) => b,
                            None => todo!(),
                        };
                        match binding {
                            LocalBinding::Var => {
                                ctx.report(error::expected_type_got_var(&tp.pos));
                                return Ok(Type::unknown());
                            }
                            LocalBinding::TypeVar(tvar) => *tvar,
                        }
                    }
                    SymRef::Global(id) => self.get_tvar(id)?,
                };
                let name = path.to_string();
                match Type::type_app(tv.clone(), &name, tps, &path.try_last().unwrap().pos) {
                    Ok(tp) => tp,
                    Err(diag) => {
                        ctx.report(diag);
                        return Ok(Type::unknown());
                    }
                }
            }
        })
    }

    pub(crate) fn leave_scope(&mut self) {
        match self.local_scopes.pop() {
            Some(_) => (),
            None => panic!("attempted to leave without any local scope"),
        }
    }

    pub(crate) fn find_local_var_kind(&self, str: String) -> Option<&LocalBinding> {
        for scope in self.local_scopes.iter().rev() {
            if let Some(binding) = scope.get(&str) {
                return Some(binding);
            }
        }
        None
    }

    pub(crate) fn find_symbol(&self, path: Path) -> Result<SymRef, Diagnostic> {
        if let Some(id) = path.clone().if_single() {
            let str = id.name_str();
            for scope in self.local_scopes.iter().rev() {
                if let Some(_) = scope.get(&str) {
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

    pub(crate) fn add_local_var(&mut self, name: String) {
        self.local_scopes
            .last_mut()
            .expect("cant add without a local scope")
            .insert(name, LocalBinding::Var);
    }

    pub(crate) fn add_local_type_var(&mut self, name: String, tv: TVar) {
        assert_eq!(tv.kind(), TVarKind::Parameter);
        self.local_scopes
            .last_mut()
            .expect("cant add without a local scope")
            .insert(name, LocalBinding::TypeVar(tv));
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

    pub(crate) fn add_type_info(&mut self, id: TVar, type_info: TypeInfo) {
        self.tvar_map.insert(id, type_info);
    }
}
