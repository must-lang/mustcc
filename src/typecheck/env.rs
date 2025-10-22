use std::collections::BTreeMap;

use crate::{error::InternalError, tp::Type};

#[derive(Debug)]
pub struct Env {
    expected_ret: Type,
    scopes: Vec<BTreeMap<String, (bool, Type)>>,
}
impl Env {
    pub(crate) fn new(expected_ret: Type) -> Self {
        Self {
            expected_ret,
            scopes: vec![BTreeMap::new()],
        }
    }

    pub(crate) fn add_var(&mut self, name: String, is_mut: bool, tp: Type) {
        self.scopes
            .last_mut()
            .expect("there should be at least one scope")
            .insert(name, (is_mut, tp));
    }

    pub(crate) fn finish(&self) -> Result<(), InternalError> {
        Ok(())
    }

    pub(crate) fn lookup(&self, name: &String) -> (bool, &crate::tp::Type) {
        for scope in self.scopes.iter().rev() {
            if let Some((is_mut, tp)) = scope.get(name) {
                return (*is_mut, tp);
            }
        }
        unreachable!()
    }

    pub(crate) fn new_scope(&mut self) {
        self.scopes.push(BTreeMap::new())
    }

    pub(crate) fn leave_scope(&mut self) {
        self.scopes.pop();
    }

    pub(crate) fn expected_ret(&self) -> Type {
        self.expected_ret.clone()
    }
}
