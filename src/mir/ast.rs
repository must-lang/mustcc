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

#[derive(Debug, Clone)]
pub enum TypeLayout {
    Simple {
        tp: Type,
    },
    Array {
        elem_layout: Box<Layout>,
        elems: usize,
    },
    Tuple {
        field_count: usize,
        fields: Vec<Layout>,
    },
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub layout: TypeLayout,
    pub size: usize,
    pub offset: usize,
    pub align: usize,
}
impl Layout {
    pub(crate) fn require_stack(&self) -> bool {
        match &self.layout {
            TypeLayout::Simple { tp } => false,
            TypeLayout::Array { elem_layout, elems } => true,
            TypeLayout::Tuple {
                field_count,
                fields,
            } => true,
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
    Func { args: Vec<Type>, returns: Vec<Type> },
}

#[derive(Debug)]
pub struct Func {
    pub id: NodeID,
    pub args: Vec<(VarID, bool, Type)>,
    pub returns: Vec<Type>,
    pub body: Expr,
    pub var_needs_stack: HashMap<VarID, bool>,
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
pub enum Expr {
    NumLit(usize, Type),
    StringLit(String, Layout),
    Tuple {
        fields: Vec<(Expr, Layout)>,
        layout: Layout,
    },
    FunCall {
        expr: Box<Expr>,
        args: Vec<Expr>,
        args_tp: Vec<Layout>,
        ret_tp: Layout,
    },
    FieldAccess {
        object: Box<Expr>,
        field_id: usize,
        struct_layout: Layout,
        element_layout: Layout,
    },
    Block {
        exprs: Vec<Expr>,
        last_expr: Box<Expr>,
        block_tp: Layout,
    },
    Return {
        expr: Box<Expr>,
        ret_tp: Type,
    },
    Let {
        id: VarID,
        layout: Layout,
        is_mut: bool,
        expr: Box<Expr>,
    },
    Assign {
        lval: Box<Expr>,
        rval: Box<Expr>,
        assign_tp: Layout,
    },
    Ref {
        var: VarRef,
        tp: Type,
    },
    RefMut {
        var: VarRef,
        tp: Type,
    },
    Deref {
        expr: Box<Expr>,
        in_tp: Layout,
    },
    Char(u8),
    ArrayInitRepeat(Box<Expr>, usize, Layout),
    ArrayInitExact(Vec<Expr>, Layout),
    While {
        pred: Box<Expr>,
        block: Box<Expr>,
    },
    IndexAccess {
        arr: Box<Expr>,
        index: Box<Expr>,
        arr_layout: Layout,
        elem_layout: Layout,
    },
    Var(VarRef),
    Builtin(String, Vec<Expr>),
}
