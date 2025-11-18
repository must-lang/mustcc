mod error;
mod tvar;
mod uvar;

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

pub use tvar::{TVar, TVarKind};
use uvar::UVar;

use crate::{
    common::Position,
    error::diagnostic::{Diagnostic, Label},
};

pub const BUILTIN_TYPES: [&'static str; 13] = [
    "never", "bool", "order", "u8", "u16", "u32", "u64", "usize", "i8", "i16", "i32", "i64",
    "isize",
];

/// The abstract type representation.
///
/// Use [Type::view] to see the actual type.
#[derive(Debug, Clone)]
pub struct Type(TypeView);

#[derive(Debug, Clone)]
pub enum TypeView {
    Unknown,
    UVar(UVar),
    NumericUVar(UVar),
    Var(TVar),
    NamedVar(TVar, String),
    Tuple(Vec<Type>),
    Array(usize, Box<Type>),
    Fun(Vec<Type>, Box<Type>),
    Ptr(Box<Type>),
    MutPtr(Box<Type>),
    TypeApp(TVar, String, Vec<Type>),
}

impl Type {
    /// View the underlying type.
    ///
    /// It will never show resolved unification variables, following paths.
    pub fn view(&self) -> TypeView {
        match &self.0 {
            TypeView::UVar(uvar) => {
                let root = uvar.find();
                match root.try_resolved() {
                    Some(tp) => tp.view(),
                    None => TypeView::UVar(uvar.clone()),
                }
            }
            TypeView::NumericUVar(uvar) => {
                let root = uvar.find();
                match root.try_resolved() {
                    Some(tp) => tp.view(),
                    None => TypeView::NumericUVar(uvar.clone()),
                }
            }
            _ => self.0.clone(),
        }
    }

    pub(crate) fn tuple(vec: Vec<Type>) -> Type {
        Type(TypeView::Tuple(vec))
    }

    pub(crate) fn unit() -> Type {
        Self::tuple(vec![])
    }

    pub(crate) fn tvar(p: TVar) -> Type {
        todo!()
    }

    pub(crate) fn ptr(tp: Type) -> Type {
        Type(TypeView::Ptr(Box::new(tp)))
    }

    pub(crate) fn mut_ptr(tp: Type) -> Type {
        Type(TypeView::MutPtr(Box::new(tp)))
    }

    pub(crate) fn named_var(tvar: TVar, name: &str, pos: &Position) -> Result<Type, Diagnostic> {
        if let TVarKind::TypeCons(n) = tvar.kind() {
            return Err(error::type_params_mismatch(pos, n.into(), 0));
        }
        Ok(Type(TypeView::NamedVar(tvar, name.to_string())))
    }

    pub(crate) fn fun(args: Vec<Type>, ret: Type) -> Type {
        Type(TypeView::Fun(args, Box::new(ret)))
    }

    pub(crate) fn unknown() -> Type {
        Type(TypeView::Unknown)
    }

    pub fn fresh_uvar() -> Type {
        Type(TypeView::UVar(UVar::new()))
    }

    pub fn numeric_uvar() -> Type {
        Type(TypeView::NumericUVar(UVar::new()))
    }

    pub(crate) fn array(size: usize, tp: Type) -> Type {
        Type(TypeView::Array(size, Box::new(tp)))
    }

    pub(crate) fn builtin(name: &str) -> Type {
        let tv = TVar::of_builtin(name);
        unsafe { Type::named_var(tv, name, &Position::nowhere()).unwrap_unchecked() }
    }

    pub(crate) fn type_app(
        tvar: TVar,
        name: &str,
        tps: Vec<Type>,
        pos: &Position,
    ) -> Result<Type, Diagnostic> {
        if let TVarKind::TypeCons(n) = tvar.kind() {
            if tps.len() != n.into() {
                return Err(error::type_params_mismatch(pos, n.into(), tps.len()));
            }
            Ok(Type(TypeView::TypeApp(tvar, name.to_string(), tps)))
        } else {
            return Err(error::type_params_mismatch(pos, 0, tps.len()));
        }
    }

    pub fn substitute(&self, subst: &HashMap<TVar, Type>) -> Type {
        match self.view() {
            TypeView::Unknown | TypeView::UVar(_) | TypeView::NumericUVar(_) => self.clone(),
            TypeView::NamedVar(tvar, _) | TypeView::Var(tvar) => match subst.get(&tvar) {
                Some(tp) => tp.clone(),
                None => self.clone(),
            },
            TypeView::Tuple(items) => {
                let tps = items.iter().map(|tp| tp.substitute(subst)).collect();
                Type::tuple(tps)
            }
            TypeView::Array(size, tp) => {
                let tp = tp.substitute(subst);
                Type::array(size, tp)
            }
            TypeView::Fun(items, ret) => {
                let tps = items.iter().map(|tp| tp.substitute(subst)).collect();
                let ret = ret.substitute(subst);
                Type::fun(tps, ret)
            }
            TypeView::Ptr(tp) => {
                let tp = tp.substitute(subst);
                Type::ptr(tp)
            }
            TypeView::MutPtr(tp) => {
                let tp = tp.substitute(subst);
                Type::mut_ptr(tp)
            }
            TypeView::TypeApp(tvar, name, items) => {
                let tps = items.iter().map(|tp| tp.substitute(subst)).collect();
                unsafe { Type::type_app(tvar, &name, tps, &Position::nowhere()).unwrap_unchecked() }
            }
        }
    }

    /// This function returns all type variables
    /// that this type's size depends on.
    pub fn get_size_dependencies(&self) -> HashSet<TVar> {
        match &self.view() {
            TypeView::Unknown => HashSet::new(),
            TypeView::UVar(uvar) | TypeView::NumericUVar(uvar) => panic!(),
            TypeView::TypeApp(tvar, _, _) | TypeView::Var(tvar) | TypeView::NamedVar(tvar, _) => {
                match tvar.kind() {
                    // don't return parameters, they will get the unsized treatment
                    TVarKind::Parameter => return HashSet::new(),
                    TVarKind::Type => (),
                    TVarKind::TypeCons(non_zero) => (),
                }
                let mut set = HashSet::new();
                set.insert(*tvar);
                set
            }
            TypeView::Tuple(items) => {
                let mut set = HashSet::new();
                for tp in items {
                    set.extend(tp.get_size_dependencies());
                }
                set
            }
            TypeView::Array(_, tp) => tp.get_size_dependencies(),
            // pointer types break the dependency
            TypeView::Fun(_, _) | TypeView::Ptr(_) | TypeView::MutPtr(_) => HashSet::new(),
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.view() {
            TypeView::Var(type_var) => write!(f, "T#{}", type_var.id()),
            TypeView::NamedVar(_, name) => write!(f, "{}", name),
            TypeView::Fun(items, ret) => {
                let items = items
                    .iter()
                    .map(|it| it.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "fn({}) -> {}", items, ret)
            }
            TypeView::UVar(uvar) => write!(f, "U?#{}", uvar.id().unwrap()),
            TypeView::Ptr(tp) => write!(f, "*{}", tp),
            TypeView::MutPtr(tp) => write!(f, "*mut {}", tp),
            TypeView::Unknown => write!(f, "{{unknown}}"),
            TypeView::NumericUVar(uvar) => write!(f, "NU?#{}", uvar.id().unwrap()),
            TypeView::Tuple(items) => {
                let items = items
                    .iter()
                    .map(|it| it.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "({})", items)
            }
            TypeView::Array(size, tp) => write!(f, "[{}]{}", size, tp),
            TypeView::TypeApp(_, s, items) => {
                let items = items
                    .iter()
                    .map(|it| it.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "{}<{}>", s, items)
            }
        }
    }
}

/// Unify two types, coercing `act_tp` to `exp_tp` if needed.
///
/// In terms of subtyping relation, `act_tp <: exp_tp` must be satisfied.
#[must_use]
pub fn unify(exp_tp: &Type, act_tp: &Type) -> bool {
    match (exp_tp.view(), act_tp.view()) {
        (_, TypeView::NamedVar(tv2, _)) | (_, TypeView::Var(tv2)) if tv2.is_never() => true,

        (TypeView::NamedVar(tv1, _), TypeView::NamedVar(tv2, _))
        | (TypeView::Var(tv1), TypeView::Var(tv2)) => tv1 == tv2,

        (TypeView::NumericUVar(uv1), TypeView::NumericUVar(uv2))
        | (TypeView::UVar(uv1), TypeView::UVar(uv2)) => {
            uv1.union(&uv2);
            true
        }
        (TypeView::UVar(uvar), _) => {
            if uvar.occurs(&act_tp) {
                false
            } else {
                uvar.resolve(act_tp.clone());
                true
            }
        }
        (_, TypeView::UVar(uvar)) => {
            if uvar.occurs(&exp_tp) {
                false
            } else {
                uvar.resolve(exp_tp.clone());
                true
            }
        }

        (TypeView::TypeApp(tv1, _, tps1), TypeView::TypeApp(tv2, _, tps2)) => {
            let ret = tv1 == tv2;
            let tps = tps1
                .iter()
                .zip(tps2.iter())
                .all(|(it1, it2)| unify(it1, it2));
            ret && tps
        }

        (TypeView::Array(s1, tp1), TypeView::Array(s2, tp2)) => s1 == s2 && unify(&tp1, &tp2),

        (TypeView::Tuple(items1), TypeView::Tuple(items2)) => items1
            .iter()
            .zip(items2.iter())
            .all(|(it1, it2)| unify(it1, it2)),

        (TypeView::NumericUVar(uvar), TypeView::Var(tv) | TypeView::NamedVar(tv, _)) => {
            if !uvar.occurs(&act_tp) && tv.is_numeric() {
                uvar.resolve(act_tp.clone());
                true
            } else {
                false
            }
        }

        (TypeView::Var(tv) | TypeView::NamedVar(tv, _), TypeView::NumericUVar(uvar)) => {
            if !uvar.occurs(&exp_tp) && tv.is_numeric() {
                uvar.resolve(exp_tp.clone());
                true
            } else {
                false
            }
        }

        // mut ptr can be used in place of const ptr
        (TypeView::Ptr(tp1), TypeView::Ptr(tp2))
        | (TypeView::Ptr(tp1), TypeView::MutPtr(tp2))
        | (TypeView::MutPtr(tp1), TypeView::MutPtr(tp2)) => unify(&*tp1, &*tp2),

        (TypeView::Fun(items1, ret1), TypeView::Fun(items2, ret2)) => {
            // use mutable ret here to unify as much as possible
            let mut ret = items1.len() != items2.len();
            if !unify(&ret1, &ret2) {
                ret = false;
            };
            let items = items1
                .iter()
                .zip(items2.iter())
                .all(|(it1, it2)| unify(it1, it2));
            ret && items
        }

        _ => false,
    }
}
