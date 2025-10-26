use std::collections::BTreeMap;

use crate::{
    common::Position,
    error::{InternalError, context::Context},
    tp::{Type, TypeView},
    typecheck::error,
};

#[derive(Debug)]
pub struct Env {
    expected_ret: Type,
    scopes: Vec<BTreeMap<String, (bool, Type)>>,
    uvars: Vec<(Type, Position)>,
}
impl Env {
    pub(crate) fn new(expected_ret: Type) -> Self {
        Self {
            expected_ret,
            scopes: vec![BTreeMap::new()],
            uvars: vec![],
        }
    }

    pub fn fresh_uvar(&mut self, pos: &Position) -> Type {
        let tp = Type::fresh_uvar();
        self.uvars.push((tp.clone(), pos.clone()));
        tp
    }

    pub(crate) fn add_var(&mut self, name: String, is_mut: bool, tp: Type) {
        self.scopes
            .last_mut()
            .expect("there should be at least one scope")
            .insert(name, (is_mut, tp));
    }

    pub(crate) fn finish(self, ctx: &mut Context) -> Result<(), InternalError> {
        for (tp, pos) in self.uvars {
            match tp.view() {
                TypeView::UVar(uvar) => {
                    ctx.report(error::cannot_infer_type(pos));
                }

                TypeView::NumericUVar(uvar) => {
                    uvar.resolve(Type::builtin("i32"));
                }
                TypeView::Unknown
                | TypeView::Var(_)
                | TypeView::NamedVar(_, _)
                | TypeView::Tuple(_)
                | TypeView::Array(_, _)
                | TypeView::Fun(_, _)
                | TypeView::Ptr(_)
                | TypeView::MutPtr(_) => continue,
            }
        }
        Ok(())
    }

    pub(crate) fn lookup(&self, name: &String) -> (bool, &Type) {
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

    pub(crate) fn numeric_uvar(&mut self, pos: &Position) -> Type {
        let tp = Type::numeric_uvar();
        self.uvars.push((tp.clone(), pos.clone()));
        tp
    }
}
