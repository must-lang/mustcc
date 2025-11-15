use std::collections::HashMap;

use crate::common::NodeID;

#[derive(Debug, Clone)]
pub enum Type {
    Tu8,
    Tu16,
    Tu32,
    Tu64,
    Tusize,
    Ti8,
    Ti16,
    Ti32,
    Ti64,
    Tisize,
}

impl Type {
    pub(crate) fn to_cl_type(&self) -> cranelift_codegen::ir::Type {
        use cranelift_codegen::ir::types::*;
        match self {
            Self::Tu8 => I8,
            Self::Tu16 => I16,
            Self::Tu32 => I32,
            Self::Tu64 => I64,
            Self::Tusize => I64,
            Self::Ti8 => I8,
            Self::Ti16 => I16,
            Self::Ti32 => I32,
            Self::Ti64 => I64,
            Self::Tisize => I64,
        }
    }
}

#[derive(Debug)]
pub struct Program {
    pub symbols: HashMap<NodeID, Symbol>,
    pub functions: Vec<Func>,
}

#[derive(Debug)]
pub struct Symbol {
    pub name: String,
    pub kind: SymKind,
    pub is_extern: bool,
    pub mangle: bool,
}

#[derive(Debug)]
pub enum SymKind {
    Func {
        args: Vec<Type>,
        returns: Vec<Type>,
    },
    BuiltinFunc {
        args: Vec<Type>,
        returns: Vec<Type>,
        item_name: String,
    },
}

#[derive(Debug)]
pub struct Func {
    pub id: NodeID,
    pub args: Vec<(VarID, Type)>,
    pub returns: Vec<Type>,
    pub body: Expr,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub struct VarID(usize);

impl VarID {
    pub fn get(&self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub struct VarSpawner(usize);
impl VarSpawner {
    pub fn new() -> Self {
        VarSpawner(0)
    }

    pub fn fresh(&mut self) -> VarID {
        self.0 += 1;
        VarID(self.0)
    }
}

#[derive(Debug)]
pub enum VarRef {
    Local(VarID),
    Global(NodeID),
}

#[derive(Debug)]
pub enum Value {
    Unit,
    Var(VarRef),
    Const(usize, Type),
}

#[derive(Debug)]
pub enum Expr {
    Value(Value),
    FunCall {
        expr: VarRef,
        args: Vec<Expr>,
    },
    Return {
        expr: Box<Expr>,
    },
    Let {
        id: VarID,
        e1: Box<Expr>,
        e2: Box<Expr>,
    },
    StackSlot {
        size: u32,
    },
    Store {
        ptr: Box<Expr>,
        val: Box<Expr>,
        offset: i32,
    },
    Load {
        tp: Type,
        ptr: Box<Expr>,
        offset: i32,
    },
    While {
        pred: Box<Expr>,
        block: Box<Expr>,
    },
}
