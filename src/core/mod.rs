pub mod ast;
mod env;

use std::mem::transmute;

use crate::{core::env::Env, mir::ast as in_a};
use ast as out_a;

pub fn translate(prog: in_a::Program) -> out_a::Program {
    let symbols = unsafe { transmute(prog.symbols) };
    let functions = prog.functions.into_iter().map(|f| tr_func(f)).collect();

    out_a::Program { symbols, functions }
}

fn tr_func(f: in_a::Func) -> out_a::Func {
    let mut args = vec![];
    let mut env = Env::new();
    for (id, _, tp) in f.args {
        let id = env.add_var(id);
        let tp = tr_type(tp);
        args.push((id, tp));
    }
    let returns = f.returns.into_iter().map(|tp| tr_type(tp)).collect();

    let (_, body) = tr_expr(env, f.body);

    out_a::Func {
        id: f.id,
        args,
        returns,
        body,
    }
}

fn tr_type(tp: in_a::Type) -> out_a::Type {
    unsafe { transmute(tp) }
}

fn tr_expr(mut env: Env, e: in_a::Expr) -> (Env, out_a::Expr) {
    match e {
        in_a::Expr::NumLit(n, tp) => (env, out_a::Expr::Value(ast::Value::Const(n, tr_type(tp)))),
        in_a::Expr::StringLit(_, layout) => todo!(),
        in_a::Expr::Tuple { fields, layout } => {
            let ss = out_a::Expr::StackSlot {
                size: layout.size as u32,
            };
            let id = env.fresh_var();
            let var = out_a::VarRef::Local(id);
            let e = out_a::Expr::Value(ast::Value::Var(var));
            let (env, e2) = fields
                .into_iter()
                .fold((env, e), |(env, e1), (e2, lt)| tr_expr(env, e2));
            (
                env,
                out_a::Expr::Let {
                    id,
                    e1: Box::new(ss),
                    e2: Box::new(e2),
                },
            )
        }
        in_a::Expr::FunCall {
            expr,
            args,
            args_tp,
            ret_tp,
        } => todo!(),
        in_a::Expr::FieldAccess {
            object,
            field_id,
            struct_layout,
            element_layout,
        } => todo!(),
        in_a::Expr::Block {
            exprs,
            last_expr,
            block_tp,
        } => unreachable!(),
        in_a::Expr::Return { expr, ret_tp } => todo!(),
        in_a::Expr::Let {
            id,
            layout,
            is_mut,
            expr,
        } => unreachable!(),
        in_a::Expr::Assign {
            lval,
            rval,
            assign_tp,
        } => todo!(),
        in_a::Expr::Ref { var, tp } => todo!(),
        in_a::Expr::RefMut { var, tp } => todo!(),
        in_a::Expr::Deref { expr, in_tp } => todo!(),
        in_a::Expr::Char(_) => todo!(),
        in_a::Expr::ArrayInitRepeat(expr, _, layout) => todo!(),
        in_a::Expr::ArrayInitExact(exprs, layout) => todo!(),
        in_a::Expr::While { pred, block } => todo!(),
        in_a::Expr::IndexAccess {
            arr,
            index,
            arr_layout,
            elem_layout,
        } => todo!(),
        in_a::Expr::Var(var_ref) => match var_ref {
            in_a::VarRef::Local(var_id) => {
                let id = env.lookup(var_id);
                (
                    env,
                    out_a::Expr::Value(ast::Value::Var(ast::VarRef::Local(id))),
                )
            }
            in_a::VarRef::Global(node_id) => todo!(),
        },
        in_a::Expr::LetIn {
            id,
            layout,
            is_mut,
            expr,
            e2,
        } => todo!(),
    }
}
