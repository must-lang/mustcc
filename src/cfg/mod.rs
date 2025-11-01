use crate::{error::InternalError, flatten::ast as in_a};

pub mod ast;

use ast as out_a;

pub fn translate(prog: in_a::Program) -> Result<out_a::Program, InternalError> {
    todo!()
}
