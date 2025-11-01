use crate::common::NodeID;

#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Func>,
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug)]
pub struct Func {
    pub id: NodeID,
    pub name: String,
    pub args: Vec<(RegID, Type)>,
    pub returns: Vec<Type>,
    pub body: Vec<Block>,
}

#[derive(Debug)]
pub struct Block {
    pub id: BlockID,
    pub body: Vec<Inst>,
    pub term: Terminal,
}

#[derive(Debug)]
pub enum Inst {
    Load {
        v_out: RegID,
        v_in: Operand,
        offset: usize,
    },
    Store {
        v_in: RegID,
        v_out: Operand,
        offset: usize,
    },
}

#[derive(Debug)]
pub enum Terminal {
    Return,
    Jump(BlockID),
    JumpCond {
        cond: Operand,
        if_false: BlockID,
        if_true: BlockID,
    },
}

#[derive(Debug)]
pub enum Operand {
    IConst(usize, Type),
    Global(NodeID),
    Reg(RegID),
}

#[derive(Debug)]
pub struct RegID(usize);
#[derive(Debug)]
pub struct BlockID(usize);
