use std::collections::{HashMap, HashSet};

mod error;
mod type_sort;

use crate::{
    common::{NodeID, Position, RAttribute},
    error::context::Context,
    symtable::type_sort::{calculate_size, make_dep_tree, topo_sort},
    tp::{TVar, Type, TypeView},
};

#[derive(Debug)]
pub struct SymTable {
    node_map: HashMap<NodeID, SymInfo>,
    tvar_map: HashMap<TVar, TypeInfo>,
    tvar_order: Vec<TVar>,
    tvar_size: HashMap<TVar, usize>,
}

impl SymTable {
    pub(crate) fn init(
        ctx: &mut Context,
        node_map: HashMap<NodeID, SymInfo>,
        tvar_map: HashMap<TVar, TypeInfo>,
    ) -> SymTable {
        let dep_tree: HashMap<TVar, HashSet<TVar>> = make_dep_tree(&tvar_map, &node_map);
        let (tvar_order, cyclic) = topo_sort(dep_tree);
        for tv in cyclic {
            let info = tvar_map.get(&tv).unwrap();
            ctx.report(error::resursive_types(&info.pos));
        }
        let tvar_size = calculate_size(ctx, &tvar_map, &node_map, &tvar_order);
        let st = Self {
            node_map,
            tvar_map,
            tvar_order,
            tvar_size,
        };
        st.check_sizes(ctx);
        st
    }

    pub fn get_type_order(&self) -> &Vec<TVar> {
        &self.tvar_order
    }

    pub fn get_items(&self) -> &HashMap<NodeID, SymInfo> {
        &self.node_map
    }

    pub fn destroy_for_items(self) -> HashMap<NodeID, SymInfo> {
        self.node_map
    }

    fn check_sizes(&self, ctx: &mut Context) {
        for (_, sym) in &self.node_map {
            match &sym.kind {
                SymKind::Func { params, args, ret } => {
                    for arg in args {
                        match self.sizeof(arg) {
                            TypeSize::Sized(_) => (),
                            TypeSize::Unknown => (),
                            TypeSize::Unsized => {
                                ctx.report(error::unsized_type(&sym.pos));
                            }
                            TypeSize::NotUnified => panic!(),
                        }
                    }
                    match self.sizeof(ret) {
                        TypeSize::Sized(_) => (),
                        TypeSize::Unknown => (),
                        TypeSize::Unsized => {
                            ctx.report(error::unsized_type(&sym.pos));
                        }
                        TypeSize::NotUnified => panic!(),
                    }
                }
                SymKind::Struct(tvar) => (),
                SymKind::Enum(tvar) => (),
                SymKind::EnumCons { id, args, parent } => {
                    for arg in args {
                        match self.sizeof(arg) {
                            TypeSize::Sized(_) => (),
                            TypeSize::Unknown => (),
                            TypeSize::Unsized => {
                                ctx.report(error::unsized_type(&sym.pos));
                            }
                            TypeSize::NotUnified => panic!(),
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn find_sym_info(&self, node_id: NodeID) -> &SymInfo {
        self.node_map.get(&node_id).unwrap()
    }

    pub(crate) fn find_type_info(&self, tvar: TVar) -> &TypeInfo {
        self.tvar_map.get(&tvar).unwrap()
    }

    pub fn sizeof(&self, tp: &Type) -> TypeSize {
        match tp.view() {
            TypeView::Unknown => TypeSize::Unknown,
            TypeView::UVar(uvar) | TypeView::NumericUVar(uvar) => TypeSize::NotUnified,
            TypeView::TypeApp(tvar, _, _) | TypeView::Var(tvar) | TypeView::NamedVar(tvar, _) => {
                match self.tvar_size.get(&tvar) {
                    Some(n) => TypeSize::Sized(*n),
                    None => TypeSize::Unsized,
                }
            }
            TypeView::Tuple(items) => {
                let mut size = 0;
                for tp in items {
                    match self.sizeof(&tp) {
                        TypeSize::Sized(n) => size += n,
                        TypeSize::Unsized => return TypeSize::Unsized,
                        TypeSize::Unknown => return TypeSize::Unknown,
                        TypeSize::NotUnified => return TypeSize::NotUnified,
                    }
                }
                TypeSize::Sized(size)
            }
            TypeView::Array(size, tp) => match self.sizeof(&tp) {
                TypeSize::Sized(n) => TypeSize::Sized(n * size),
                TypeSize::Unsized => TypeSize::Unsized,
                TypeSize::Unknown => TypeSize::Unknown,
                TypeSize::NotUnified => TypeSize::NotUnified,
            },
            TypeView::Fun(items, ret) => {
                let mut size = 0;
                for tp in items {
                    match self.sizeof(&tp) {
                        TypeSize::Sized(n) => size += n,
                        TypeSize::Unsized => return TypeSize::Unsized,
                        TypeSize::Unknown => return TypeSize::Unknown,
                        TypeSize::NotUnified => return TypeSize::NotUnified,
                    }
                }
                match self.sizeof(&ret) {
                    TypeSize::Sized(n) => size += n,
                    TypeSize::Unsized => return TypeSize::Unsized,
                    TypeSize::Unknown => return TypeSize::Unknown,
                    TypeSize::NotUnified => return TypeSize::NotUnified,
                }
                TypeSize::Sized(size)
            }
            TypeView::Ptr(_) | TypeView::MutPtr(_) => TypeSize::Sized(8),
        }
    }

    pub(crate) fn get_builtin_id(&self, name: &str) -> Option<NodeID> {
        for (id, info) in &self.node_map {
            if let Some(n) = &info.builtin_name {
                if n == name {
                    return Some(*id);
                }
            }
        }
        None
    }
}

#[derive(Debug)]
pub enum TypeSize {
    Sized(usize),
    Unsized,
    Unknown,
    NotUnified,
}
impl TypeSize {
    pub(crate) fn as_usize(&self) -> usize {
        match self {
            TypeSize::Sized(n) => *n,
            TypeSize::Unsized => panic!(),
            TypeSize::Unknown => panic!(),
            TypeSize::NotUnified => panic!(),
        }
    }
}

#[derive(Debug)]
pub struct SymInfo {
    pub name: String,
    pub pos: Position,
    pub kind: SymKind,
    pub builtin_name: Option<String>,
    pub is_extern: bool,
    pub mangle: bool,
}

impl SymInfo {
    pub(crate) fn build(name: String, pos: Position, kind: SymKind) -> SymInfo {
        Self {
            name,
            pos,
            kind,
            builtin_name: None,
            is_extern: false,
            mangle: true,
        }
    }

    /// Set symbol flags according to given attributes.
    pub(crate) fn with_attributes(mut self, attributes: Vec<RAttribute>) -> SymInfo {
        for attr in attributes {
            match attr.name.data.as_str() {
                "extern" => self.is_extern = true,
                "no_mangle" => self.mangle = false,
                _ => continue,
            }
        }
        self
    }
}

#[derive(Debug)]
pub enum SymKind {
    Func {
        params: HashSet<TVar>,
        args: Vec<Type>,
        ret: Type,
    },
    Struct(TVar),
    Enum(TVar),
    EnumCons {
        id: usize,
        args: Vec<Type>,
        parent: NodeID,
    },
}

#[derive(Debug)]
pub struct TypeInfo {
    pub name: String,
    pub pos: Position,
    pub methods: HashMap<String, NodeID>,
    pub kind: TypeKind,
}

#[derive(Debug)]
pub enum TypeKind {
    Builtin,
    Struct {
        params: HashSet<TVar>,
        fields: HashMap<String, Type>,
    },
    Enum {
        params: HashSet<TVar>,
        constructors: HashMap<String, NodeID>,
    },
}
