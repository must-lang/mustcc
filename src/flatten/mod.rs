pub mod ast;

mod env;

use crate::{
    error::InternalError, flatten::env::Env, symtable::SymTable, tp::Type, typecheck::ast as in_a,
};
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

fn tr_func(sym_table: &SymTable, func: in_a::Func) -> Result<out_a::Func, InternalError> {
    let mut env = Env::new();

    let mut args = vec![];

    for (name, _, tp) in func.args {
        let var = env.add_var(name);
        args.push((var, tp))
    }

    let body = tr_expr(&mut env, &sym_table, func.body, &|v| vec![]);

    let func = out_a::Func {
        id: func.id,
        name: func.name,
        args,
        ret_type: func.ret_type,
        body,
    };
    Ok(func)
}

#[must_use]
fn tr_expr(
    env: &mut Env,
    st: &SymTable,
    body: in_a::Expr,
    cont: &dyn Fn(out_a::VarRef) -> Vec<ast::Stmt>,
) -> Vec<ast::Stmt> {
    match body {
        in_a::Expr::NumLit(_, _) => todo!(),
        in_a::Expr::StringLit(_, _) => todo!(),
        in_a::Expr::LocalVar { name, tp } => todo!(),
        in_a::Expr::GlobalVar { id, tp } => todo!(),
        in_a::Expr::Tuple(exprs) => todo!(),
        in_a::Expr::FunCall {
            expr,
            args,
            args_tp,
            ret_tp,
        } => todo!(),
        in_a::Expr::FieldAccess {
            object,
            field_name,
            field_tp,
        } => todo!(),
        in_a::Expr::Block {
            exprs,
            last_expr,
            block_tp,
        } => todo!(),
        in_a::Expr::Return { expr, ret_tp } => todo!(),
        in_a::Expr::Let {
            name,
            tp,
            is_mut,
            expr,
        } => todo!(),
        in_a::Expr::Match { expr, clauses } => todo!(),
        in_a::Expr::StructCons {
            id,
            initializers,
            tp,
        } => todo!(),
        in_a::Expr::Assign {
            lval,
            rval,
            assign_tp,
        } => todo!(),
        in_a::Expr::Ref { expr, tp } => todo!(),
        in_a::Expr::RefMut { expr, tp } => todo!(),
        in_a::Expr::Deref { expr, in_tp } => todo!(),
        in_a::Expr::Error => todo!(),
        in_a::Expr::Char(_) => todo!(),
        in_a::Expr::String(_) => todo!(),
        in_a::Expr::ArrayInitRepeat(expr, _, _) => todo!(),
        in_a::Expr::ArrayInitExact(exprs, _) => todo!(),
        in_a::Expr::While { pred, block } => todo!(),
    }
}
