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
    // TODO
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
    },
}

#[derive(Debug)]
pub struct StructField {
    pub visibility: Visibility,
    pub name: String,
    pub tp: Type,
    pub pos: Position,
}
