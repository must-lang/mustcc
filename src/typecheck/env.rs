use std::{collections::BTreeMap, ops::ControlFlow};

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
        // TODO: also check inside compound types (or perform smart occurs check)
        for (tp, pos) in self.uvars {
            check_resolved(ctx, tp, &pos);
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

fn check_resolved(ctx: &mut Context, tp: Type, pos: &Position) {
    match tp.view() {
        TypeView::UVar(_) => {
            ctx.report(error::cannot_infer_type(pos));
        }
        TypeView::NumericUVar(uvar) => {
            println!("Resolving {:?} at {:?}", uvar, pos);
            uvar.resolve(Type::builtin("i32"));
            println!("Resolved? {:?}", uvar.try_resolved());
        }
        TypeView::Unknown | TypeView::Var(_) | TypeView::NamedVar(_, _) => {}
        TypeView::Tuple(items) => {
            for it in items {
                check_resolved(ctx, it, pos);
            }
        }
        TypeView::Array(_, tp) => check_resolved(ctx, *tp, pos),
        TypeView::Fun(items, ret) => {
            for it in items {
                check_resolved(ctx, it, pos);
            }
            check_resolved(ctx, *ret, pos);
        }
        TypeView::Ptr(tp) => check_resolved(ctx, *tp, pos),
        TypeView::MutPtr(tp) => check_resolved(ctx, *tp, pos),
        TypeView::TypeApp(_, _, items) => {
            for it in items {
                check_resolved(ctx, it, pos);
            }
        }
    }
}
