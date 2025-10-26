//! Unification variable and related functions.

use super::{Type, TypeView};
use std::{cell::RefCell, rc::Rc};

/// Unification variable that can be substituted by some concrete type.
#[derive(Debug, Clone)]
pub struct UVar(Rc<RefCell<UVarData>>);

#[derive(Debug, Clone)]
enum UVarData {
    Unresolved(usize),
    Link(UVar),
    Resolved(Type),
}

static mut COUNTER: usize = 64;

impl UVar {
    /// Create a fresh unification variable.
    pub fn new() -> Self {
        unsafe {
            COUNTER += 1;
            let uvar = Rc::new(RefCell::new(UVarData::Unresolved(COUNTER)));
            Self(uvar)
        }
    }

    /// Returns id of unresolved unification variable.
    pub fn id(&self) -> Option<usize> {
        match &*self.0.borrow() {
            UVarData::Unresolved(id) => Some(*id),
            UVarData::Link(uvar) => uvar.id(),
            _ => None,
        }
    }

    /// Find representative of a unification variable.
    ///
    /// Performs path compression.
    pub fn find(&self) -> UVar {
        let borrow = &mut *self.0.borrow_mut();
        match borrow {
            UVarData::Unresolved(_) | UVarData::Resolved(_) => self.clone(),
            UVarData::Link(uvar) => {
                let root = uvar.clone().find();
                *borrow = UVarData::Link(root.clone());
                root
            }
        }
    }

    /// Union two unification variables.
    pub fn union(&self, other: &UVar) {
        let root1 = self.find();
        let root2 = other.find();
        if !Rc::ptr_eq(&root1.0, &root2.0) {
            *root1.0.borrow_mut() = UVarData::Link(root2.clone());
        }
    }

    /// Resolve unification variable to a given type.
    ///
    /// Panics if variable was already resolved or isn't a representative.
    pub fn resolve(&self, tp: Type) {
        let u = self.find();
        match &mut *u.0.borrow_mut() {
            UVarData::Resolved(_) => panic!("unif variable already resolved"),
            UVarData::Link(_) => panic!("cant resolve non-root unif variables"),
            u => *u = UVarData::Resolved(tp),
        }
    }

    /// Returns type of resolved unification variable.
    pub(crate) fn try_resolved(&self) -> Option<Type> {
        match &*self.0.borrow() {
            UVarData::Resolved(tp) => Some(tp.clone()),
            _ => None,
        }
    }

    /// Checks if unification variable occurs inside of a given type.
    pub fn occurs(&self, other: &Type) -> bool {
        match other.view() {
            TypeView::Var(_) | TypeView::NamedVar(_, _) => false,
            TypeView::UVar(uvar) | TypeView::NumericUVar(uvar) => Rc::ptr_eq(&self.0, &uvar.0),
            TypeView::Fun(args, ret) => {
                args.iter().any(|arg| self.occurs(arg)) || self.occurs(&ret)
            }
            TypeView::Ptr(tp) | TypeView::MutPtr(tp) | TypeView::Array(_, tp) => self.occurs(&tp),
            TypeView::Tuple(items) => items.iter().any(|item| self.occurs(item)),
            TypeView::Unknown => false,
            TypeView::TypeApp(_, _, items) => items.iter().any(|item| self.occurs(item)),
        }
    }
}
