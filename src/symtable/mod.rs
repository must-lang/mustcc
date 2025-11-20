use std::collections::{HashMap, HashSet};

mod error;
pub mod layout;
mod type_sort;

use crate::{
    common::{NodeID, Position, RAttribute},
    error::context::Context,
    symtable::{
        layout::{Layout, LayoutKind},
        type_sort::{make_dep_tree, topo_sort},
    },
    tp::{TVar, Type, TypeView},
};

#[derive(Debug)]
pub struct SymTable {
    node_map: HashMap<NodeID, SymInfo>,
    tvar_map: HashMap<TVar, TypeInfo>,
}

impl SymTable {
    pub(crate) fn init(
        ctx: &mut Context,
        node_map: HashMap<NodeID, SymInfo>,
        tvar_map: HashMap<TVar, TypeInfo>,
    ) -> SymTable {
        let dep_tree: HashMap<TVar, HashSet<TVar>> = make_dep_tree(&tvar_map, &node_map);
        let (_, cyclic) = topo_sort(dep_tree);
        for tv in cyclic {
            let info = tvar_map.get(&tv).unwrap();
            ctx.report(error::resursive_types(&info.pos));
        }
        Self { node_map, tvar_map }
    }

    pub fn get_items(&self) -> &HashMap<NodeID, SymInfo> {
        &self.node_map
    }

    pub fn destroy_for_items(self) -> HashMap<NodeID, SymInfo> {
        self.node_map
    }

    pub(crate) fn find_sym_info(&self, node_id: NodeID) -> &SymInfo {
        self.node_map.get(&node_id).unwrap()
    }

    pub(crate) fn find_type_info(&self, tvar: TVar) -> &TypeInfo {
        self.tvar_map.get(&tvar).unwrap()
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

    pub(crate) fn get_layout(&self, tp: &Type) -> Layout {
        match tp.view() {
            TypeView::Unknown => todo!(),
            TypeView::UVar(uvar) => todo!(),
            TypeView::NumericUVar(uvar) => todo!(),
            TypeView::NamedVar(tvar, _) | TypeView::Var(tvar) => {
                let t_info = self.find_type_info(tvar);
                match &t_info.kind {
                    TypeKind::Builtin => {
                        let size = tvar.builtin_size().unwrap();
                        let tp = tvar.builtin_as_primitive().unwrap();
                        Layout {
                            size,
                            align: 3,
                            kind: LayoutKind::Primitive(tp),
                        }
                    }
                    TypeKind::Struct { params, fields } => {
                        let mut v: Vec<_> = fields.into_iter().map(|(_, v)| v).collect();
                        v.sort_by_key(|(k, _)| k);
                        let mut layouts = vec![];
                        let mut curr_offset = 0;
                        for (_, tp) in v {
                            let layout = self.get_layout(tp);
                            // TODO: align size with layout.align
                            let total_size = layout.size;
                            layouts.push((layout, curr_offset as i32));
                            curr_offset += total_size;
                        }
                        Layout {
                            size: curr_offset,
                            align: 4,
                            kind: LayoutKind::Struct(layouts),
                        }
                    }
                    TypeKind::Enum {
                        params,
                        constructors,
                    } => todo!(),
                }
            }
            TypeView::Tuple(items) => todo!(),
            TypeView::Array(_, _) => todo!(),
            TypeView::Fun(_, _) | TypeView::Ptr(_) | TypeView::MutPtr(_) => Layout {
                size: 8,
                align: 3,
                kind: LayoutKind::Primitive(layout::Type::Tusize),
            },
            TypeView::TypeApp(tvar, _, items) => todo!(),
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
    pub kind: TypeKind,
}

#[derive(Debug)]
pub enum TypeKind {
    Builtin,
    Struct {
        params: HashSet<TVar>,
        fields: HashMap<String, (usize, Type)>,
    },
    Enum {
        params: HashSet<TVar>,
        constructors: HashMap<String, NodeID>,
    },
}
