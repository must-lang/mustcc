use crate::common::{Ident, NodeID, Position, RAttribute, Visibility};

pub use super::ScopeInfo;
pub use crate::parser::ast::{ExprNode, FnArg, RTypeNode};

#[derive(Debug)]
pub struct Program {
    pub scope_info: ScopeInfo,
    pub ast: Module,
}

// ==== Top level ==============================================================

#[derive(Debug)]
pub struct Module {
    pub attributes: Vec<RAttribute>,
    pub visibility: Visibility,
    pub id: NodeID,
    pub name: Ident,
    pub items: Vec<ModuleItem>,
    pub pos: Position,
}

#[derive(Debug)]
pub enum ModuleItem {
    Module(Module),
    Func(Func),
    Struct(Struct),
    Enum(Enum),
}

// ==== Module items ===========================================================

#[derive(Debug)]
pub struct Func {
    pub attributes: Vec<RAttribute>,
    pub visibility: Visibility,
    pub id: NodeID,
    pub name: Ident,
    pub args: Vec<FnArg>,
    pub ret_type: Option<RTypeNode>,
    pub body: Option<ExprNode>,
    pub pos: Position,
}

#[derive(Debug)]
pub struct Struct {
    pub attributes: Vec<RAttribute>,
    pub visibility: Visibility,
    pub id: NodeID,
    pub name: Ident,
    pub fields: Vec<(Ident, RTypeNode)>,
    pub pos: Position,
    pub methods: Vec<Func>,
}

#[derive(Debug)]
pub struct Enum {
    pub attributes: Vec<RAttribute>,
    pub visibility: Visibility,
    pub id: NodeID,
    pub name: Ident,
    pub constructors: Vec<Constructor>,
    pub pos: Position,
    pub methods: Vec<Func>,
}

// ==== Others =================================================================

#[derive(Debug)]
pub enum Constructor {
    Tuple {
        attributes: Vec<RAttribute>,
        id: NodeID,
        name: Ident,
        pos: Position,
        params: Vec<RTypeNode>,
    },
    Struct {
        attributes: Vec<RAttribute>,
        id: NodeID,
        name: Ident,
        pos: Position,
        params: Vec<(Ident, RTypeNode)>,
    },
}

// ==== Utility functions ======================================================

impl Module {
    pub fn empty() -> Self {
        Module {
            attributes: vec![],
            visibility: crate::common::Visibility::Private,
            id: NodeID::new_global(),
            name: crate::common::Ident {
                data: "<unknown>".into(),
                pos: Position::nowhere(),
            },
            items: vec![],
            pos: Position::nowhere(),
        }
    }
}
