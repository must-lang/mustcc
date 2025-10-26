use std::collections::HashMap;

use crate::{
    common::{NodeID, Position},
    symtable::SymTable,
    tp::{TVar, Type},
};

#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Func>,
    pub sym_table: SymTable,
}

#[derive(Debug)]
pub struct Func {
    pub id: NodeID,
    pub args: Vec<FnArg>,
    pub params: Vec<(String, TVar)>,
    pub name: String,
    pub ret_type: Type,
    pub body: ExprNode,
    pub pos: Position,
}

#[derive(Debug)]
pub struct FnArg {
    pub is_mut: bool,
    pub name: String,
    pub tp: Type,
    pub pos: Position,
}

// ==== Expr ===================================================================

#[derive(Debug)]
pub struct ExprNode {
    pub data: ExprData,
    pub pos: Position,
}

#[derive(Debug)]
pub enum SymRef {
    Local(String),
    Global(NodeID),
}

#[derive(Debug)]
pub enum ExprData {
    Var(SymRef),
    NumLit(usize),
    String(String),
    Tuple(Vec<ExprNode>),
    FunCall(Box<ExprNode>, Vec<ExprNode>),
    MethodCall(Box<ExprNode>, String, Vec<ExprNode>),
    FieldAccess(Box<ExprNode>, String),
    Block(Vec<ExprNode>, Box<ExprNode>),
    Return(Box<ExprNode>),
    Let {
        name: String,
        is_mut: bool,
        tp: Option<Type>,
        expr: Box<ExprNode>,
    },
    Ref(Box<ExprNode>),
    RefMut(Box<ExprNode>),
    Deref(Box<ExprNode>),
    If(Box<ExprNode>, Box<ExprNode>, Box<ExprNode>),
    StructCons(NodeID, HashMap<String, ExprNode>),
    Assign(Box<ExprNode>, Box<ExprNode>),
    Error,
    IndexAccess(Box<ExprNode>, Box<ExprNode>),
    Match(Box<ExprNode>, Vec<MatchClause>),
    While(Box<ExprNode>, Box<ExprNode>),
    Cast(Box<ExprNode>, Type),
    ArrayInitExact(Vec<ExprNode>),
    ArrayInitRepeat(Box<ExprNode>, usize),
    Char(u8),
}

// ==== Pattern matching =======================================================

#[derive(Debug)]
pub struct MatchClause {
    pub pattern: PatternNode,
    pub expr: ExprNode,
    pub pos: Position,
}

#[derive(Debug)]
pub struct PatternNode {
    pub data: PatternData,
    pub pos: Position,
}

#[derive(Debug)]
pub enum PatternData {
    Error,
    Wildcard,
    Number(usize),
    Var(String),
    Tuple(Vec<PatternNode>),
    TupleCons(NodeID, Vec<PatternNode>),
}
