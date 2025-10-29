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
    Value(LValue),
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

#[derive(Debug)]
pub enum VarRef {
    LocalVar { id: VarID },
    GlobalVar { id: NodeID },
}

#[derive(Debug)]
pub struct VarID(usize);

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
