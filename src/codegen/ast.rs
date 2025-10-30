use std::fmt::Display;

use crate::{common::NodeID, symtable::SymTable, tp::Type};

#[derive(Debug)]
pub struct Program {
    pub sym_table: SymTable,
    pub functions: Vec<Func>,
}

#[derive(Debug)]
pub struct Func {
    pub id: NodeID,
    pub name: String,
    pub args: Vec<(VarID, Type)>,
    pub ret_type: Type,
    pub body: Vec<Stmt>,
}

// ==== Stmt ===================================================================

#[derive(Debug)]
pub enum Stmt {
    Return {
        expr: VarRef,
        ret_tp: Type,
    },
    VarDecl {
        id: VarID,
        tp: Type,
    },
    If {
        pred: VarRef,
        th: Vec<Stmt>,
        el: Vec<Stmt>,
        block_tp: Type,
    },
    Assign {
        lval: LValue,
        rval: RValue,
    },
    While {
        cond: VarRef,
        body: Vec<Stmt>,
    },
}

// ==== Values ==================================================================

#[derive(Debug)]
pub enum RValue {
    NumLit(usize, Type),
    FunCall {
        callee: VarRef,
        args: Vec<VarRef>,
        ret_tp: Type,
    },
    Ref {
        var: VarRef,
        tp: Type,
    },
    StructCons {
        id: NodeID,
        initializers: Vec<(String, RValue)>,
        tp: Type,
    },
    ArrayInit(Vec<RValue>),
    Value(LValue),
    Tuple(Vec<RValue>),
}

#[derive(Debug)]
pub enum LValue {
    VarRef(VarRef),
    FieldAccess {
        var: VarRef,
        field_id: String,
        field_tp: Type,
    },
    Deref {
        var: VarRef,
        in_tp: Type,
    },
}

// ==== Vars ===================================================================

#[derive(Debug, Clone, Copy)]
pub enum VarRef {
    LocalVar { id: VarID },
    GlobalVar { id: NodeID },
}

#[derive(Debug, Clone, Copy)]
pub struct VarID(usize);

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

impl Display for VarID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v_{}", self.0)
    }
}
