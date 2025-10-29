use std::collections::{HashMap, HashSet};

mod error;
mod type_sort;

use crate::{
    common::{NodeID, Position, RAttribute},
    error::context::Context,
    symtable::type_sort::{calculate_size, make_dep_tree, topo_sort},
    tp::{TVar, Type},
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
        println!("{:#?}", dep_tree);
        let (tvar_order, cyclic) = topo_sort(dep_tree);
        println!("order: {:?}", tvar_order);
        for tv in cyclic {
            let info = tvar_map.get(&tv).unwrap();
            ctx.report(error::resursive_types(&info.pos));
        }
        let tvar_size = calculate_size(ctx, &tvar_map, &node_map, &tvar_order);
        println!("{:#?}", tvar_size);
        Self {
            node_map,
            tvar_map,
            tvar_order,
            tvar_size,
        }
    }

    pub(crate) fn find_sym_info(&self, node_id: NodeID) -> &SymInfo {
        self.node_map.get(&node_id).unwrap()
    }

    pub(crate) fn find_type_info(&self, tvar: TVar) -> &TypeInfo {
        self.tvar_map.get(&tvar).unwrap()
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
        for mut attr in attributes {
            match attr.name.data.as_str() {
                "builtin" => match attr.args.len().cmp(&1) {
                    std::cmp::Ordering::Equal => unsafe {
                        let name = attr.args.pop().unwrap_unchecked();
                        self.builtin_name = Some(name);
                        self.is_extern = true;
                    },
                    std::cmp::Ordering::Less => {
                        panic!("expected one argument for attribute `builtin`")
                    }
                    std::cmp::Ordering::Greater => panic!("unexpected argument for `builtin`"),
                },
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
    LocalVar,
    Primitive {
        size: usize,
    },
    Struct {
        params: HashSet<TVar>,
        fields: HashMap<String, Type>,
    },
    Enum {
        params: HashSet<TVar>,
        constructors: HashMap<String, NodeID>,
    },
}
