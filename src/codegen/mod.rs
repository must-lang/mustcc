mod ast;

mod emit;
mod env;

pub use emit::emit_code;

use crate::{error::InternalError, symtable::SymTable, tp::Type, typecheck::ast as in_a};
use ast as out_a;

pub fn translate(prog: in_a::Program) -> Result<out_a::Program, InternalError> {
    let sym_table = prog.sym_table;

    let functions = prog
        .functions
        .into_iter()
        .map(|func| tr_func(&sym_table, func))
        .collect::<Result<_, _>>()?;

    let prog = out_a::Program {
        functions,
        sym_table,
    };
    Ok(prog)
}

fn tr_func(sym_table: &SymTable, func: in_a::Func) -> Result<ast::Func, InternalError> {
    let func = out_a::Func {
        id: func.id,
        name: func.name,
        args: vec![],
        ret_type: Type::unit(),
        body: vec![],
    };
    Ok(func)
}
