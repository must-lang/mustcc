use std::collections::HashMap;

use crate::{common::NodeID, symtable::SymTable, tp::Type};

#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Func>,
    pub sym_table: SymTable,
}

#[derive(Debug)]
pub struct Func {
    pub id: NodeID,
    pub name: String,
    pub args: Vec<(String, bool, Type)>,
    pub ret_type: Type,
    pub body: Expr,
}

// ==== Expr ===================================================================

#[derive(Debug)]
pub enum Expr {
    NumLit(usize, Type),
    StringLit(String, Type),
    LocalVar {
        name: String,
        tp: Type,
    },
    GlobalVar {
        id: NodeID,
        tp: Type,
    },
    Tuple(Vec<Expr>, Type),
    FunCall {
        expr: Box<Expr>,
        args: Vec<Expr>,
        args_tp: Vec<Type>,
        ret_tp: Type,
    },
    FieldAccess {
        object: Box<Expr>,
        field_id: usize,
        struct_tp: Type,
        field_tp: Type,
    },
    Block {
        exprs: Vec<Expr>,
        last_expr: Box<Expr>,
        block_tp: Type,
    },
    Return {
        expr: Box<Expr>,
        ret_tp: Type,
    },
    Let {
        name: String,
        tp: Type,
        is_mut: bool,
        expr: Box<Expr>,
    },
    If {
        pred: Box<Expr>,
        th: Box<Expr>,
        el: Box<Expr>,
        block_tp: Type,
    },
    StructCons {
        id: NodeID,
        initializers: HashMap<String, (usize, Expr)>,
        tp: Type,
    },
    Assign {
        lval: Box<Expr>,
        rval: Box<Expr>,
        assign_tp: Type,
    },
    Ref {
        expr: Box<Expr>,
        tp: Type,
    },
    RefMut {
        expr: Box<Expr>,
        tp: Type,
    },
    Deref {
        expr: Box<Expr>,
        in_tp: Type,
    },
    Error,
    Char(u8),
    ArrayInitRepeat(Box<Expr>, usize, Type),
    ArrayInitExact(Vec<Expr>, Type),
    While {
        pred: Box<Expr>,
        block: Box<Expr>,
    },
    IndexAccess {
        arr: Box<Expr>,
        index: Box<Expr>,
        tp: Type,
    },
    Builtin(String, Vec<Expr>),
}
