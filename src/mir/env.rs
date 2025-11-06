use std::collections::HashMap;

use crate::{
    mir::ast::{VarID, VarSpawner},
    tp::Type,
};

#[derive(Debug)]
pub struct Env {
    vars: HashMap<String, VarID>,
    var_gen: VarSpawner,
}
impl Env {
    pub fn fresh_var(&mut self) -> VarID {
        self.var_gen.fresh()
    }

    pub(crate) fn add_var(&mut self, name: String) -> VarID {
        let id = self.var_gen.fresh();
        self.vars.insert(name, id);
        id
    }

    pub(crate) fn new() -> Self {
        Self {
            vars: HashMap::new(),
            var_gen: VarSpawner::new(),
        }
    }

    // pub(crate) fn var_decl(&mut self, name: Option<String>, tp: Type) -> (VarID, Stmt) {
    //     let id = self.var_gen.fresh();
    //     if let Some(s) = name {
    //         self.vars.insert(s, id);
    //     }
    //     let stmt = Stmt::LocalVarDecl { id, tp };
    //     (id, stmt)
    // }

    pub(crate) fn lookup(&self, name: &str) -> VarID {
        *self.vars.get(name).unwrap()
    }
}
