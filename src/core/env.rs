use std::collections::HashMap;

use crate::core::ast::VarSpawner;

pub struct Env {
    map: HashMap<crate::mir::ast::VarID, crate::core::ast::VarID>,
    var_gen: crate::core::ast::VarSpawner,
}
impl Env {
    pub(crate) fn fresh_var(&mut self) -> super::ast::VarID {
        self.var_gen.fresh()
    }

    pub(crate) fn add_var(&mut self, id: crate::mir::ast::VarID) -> super::ast::VarID {
        let new_id = self.var_gen.fresh();
        self.map.insert(id, new_id);
        new_id
    }

    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
            var_gen: VarSpawner::new(),
        }
    }

    pub(crate) fn lookup(&self, var_id: crate::mir::ast::VarID) -> super::ast::VarID {
        *self.map.get(&var_id).unwrap()
    }
}
