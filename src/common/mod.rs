//! A module for common syntax/utilities.
use std::{collections::VecDeque, fmt::Display};

pub mod sources;

mod node_id;
mod position;

pub use node_id::NodeID;

pub use position::{Position, PositionGenerator};

#[derive(Debug, Clone)]
pub struct Path {
    pub data: VecDeque<Ident>,
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.data
                .iter()
                .map(|id| id.name_str())
                .collect::<Vec<String>>()
                .join("::")
        )
    }
}

impl Path {
    pub fn push_back(mut self, id: Ident) -> Self {
        self.data.push_back(id);
        self
    }

    pub fn pop_back(mut self) -> Self {
        self.data.pop_back();
        self
    }
}

#[derive(Debug, Clone)]
pub struct Ident {
    pub data: String,
    pub pos: Position,
}

impl Ident {
    pub fn name_str(&self) -> String {
        self.data.clone()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Public,
}

#[derive(Debug, Clone)]
pub enum Attribute {
    Builtin(BuiltinName),
    Extern,
    NoMangle,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum BuiltinName {
    TNever = 0,

    TUnit,
    CUnit,

    TBool,
    CTrue,
    CFalse,

    TOrder,
    CLt,
    CEq,
    CGt,

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

    Fu8Add,
    Fu16Add,
    Fu32Add,
    Fu64Add,
    FusizeAdd,
    Fi8Add,
    Fi16Add,
    Fi32Add,
    Fi64Add,
    FisizeAdd,

    Fu8Sub,
    Fu16Sub,
    Fu32Sub,
    Fu64Sub,
    FusizeSub,
    Fi8Sub,
    Fi16Sub,
    Fi32Sub,
    Fi64Sub,
    FisizeSub,

    Fu8Mul,
    Fu16Mul,
    Fu32Mul,
    Fu64Mul,
    FusizeMul,
    Fi8Mul,
    Fi16Mul,
    Fi32Mul,
    Fi64Mul,
    FisizeMul,

    Fu8Div,
    Fu16Div,
    Fu32Div,
    Fu64Div,
    FusizeDiv,
    Fi8Div,
    Fi16Div,
    Fi32Div,
    Fi64Div,
    FisizeDiv,

    Fu8Cmp,
    Fu16Cmp,
    Fu32Cmp,
    Fu64Cmp,
    FusizeCmp,
    Fi8Cmp,
    Fi16Cmp,
    Fi32Cmp,
    Fi64Cmp,
    FisizeCmp,
}
