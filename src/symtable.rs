use std::collections::HashMap;

use crate::{
    common::{BuiltinName, NodeID, Position, Visibility},
    tp::{TVar, Type},
};

#[derive(Debug)]
pub struct SymTable {
    node_map: HashMap<NodeID, SymInfo>,
    tvar_map: HashMap<TVar, TypeInfo>,
}

impl SymTable {
    pub(crate) fn add_sym_info(&mut self, id: NodeID, sym_info: SymInfo) {
        self.node_map.insert(id, sym_info);
    }

    pub(crate) fn add_type_info(&mut self, id: TVar, type_info: TypeInfo) {
        self.tvar_map.insert(id, type_info);
    }

    pub(crate) fn init(tvar_map: HashMap<TVar, NodeID>) -> SymTable {
        Self {
            node_map: HashMap::new(),
            tvar_map: HashMap::new(),
        }
    }

    pub(crate) fn find(&self, node_id: NodeID) -> &SymInfo {
        self.node_map.get(&node_id).unwrap()
    }

    pub fn destroy(self) -> HashMap<NodeID, SymInfo> {
        self.node_map
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Origin {
    Local,
    External,
    Builtin(BuiltinName),
    NoMangle,
}

#[derive(Debug)]
pub enum SymInfo {
    BuiltinFunc {},
    Func {
        origin: Origin,
        name: String,
        pos: Position,
        args: Vec<Type>,
        ret: Type,
    },
    Struct(TVar),
    Enum(TVar),
    EnumCons {
        name: String,
        pos: Position,
        args: Vec<Type>,
        parent: TVar,
    },
}

#[derive(Debug)]
pub enum TypeInfo {
    Struct {
        name: String,
        pos: Position,
        fields: Vec<StructField>,
        methods: HashMap<String, NodeID>,
    },
    Enum {
        name: String,
        pos: Position,
        constructors: Vec<NodeID>,
        methods: HashMap<String, NodeID>,
    },
}

#[derive(Debug)]
pub struct StructField {
    pub name: String,
    pub tp: Type,
}
