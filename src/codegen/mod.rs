mod ast;

mod emit;
mod env;

pub use emit::emit_code;

use crate::{
    codegen::env::Env, error::InternalError, symtable::SymTable, tp::Type, typecheck::ast as in_a,
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

    let body = tr_expr(&mut env, &sym_table, func.body, &|v| {
        vec![out_a::Stmt::Return {
            expr: v,
            ret_tp: func.ret_type.clone(),
        }]
    });

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
        in_a::Expr::NumLit(n, tp) => {
            let (var, stmt) = env.var_decl(None, tp.clone());
            let mut stmts = vec![
                stmt,
                out_a::Stmt::Assign {
                    lval: out_a::LValue::VarRef(var),
                    rval: out_a::RValue::NumLit(n, tp),
                },
            ];
            stmts.extend(cont(var));
            stmts
        }
        in_a::Expr::StringLit(_, _) => vec![],
        in_a::Expr::LocalVar { name, tp } => {
            let var = env.lookup(name);
            cont(var)
        }
        in_a::Expr::GlobalVar { id, tp } => {
            let var = out_a::VarRef::GlobalVar { id };
            cont(var)
        }
        in_a::Expr::Tuple(exprs) => vec![],
        in_a::Expr::FunCall {
            expr,
            args,
            args_tp,
            ret_tp,
        } => vec![],
        in_a::Expr::FieldAccess {
            object,
            field_name,
            field_tp,
        } => vec![],
        in_a::Expr::Block {
            exprs,
            last_expr,
            block_tp,
        } => {
            let mut stmts = vec![];
            for e in exprs {
                stmts.extend(tr_expr(env, st, e, &|_| vec![]));
            }
            let last_stmts = tr_expr(env, st, *last_expr, cont);
            stmts.extend(last_stmts);
            stmts
        }
        in_a::Expr::Return { expr, ret_tp } => vec![],
        in_a::Expr::Let {
            name,
            tp,
            is_mut,
            expr,
        } => {
            let (var, stmt) = env.var_decl(Some(name), tp);
            let mut stmts = vec![stmt];
            let init_stmts = tr_expr(env, st, *expr, &|v| {
                vec![out_a::Stmt::Assign {
                    lval: out_a::LValue::VarRef(var),
                    rval: out_a::RValue::Value(out_a::LValue::VarRef(v)),
                }]
            });
            stmts.extend(init_stmts);
            stmts.extend(cont(var));
            stmts
        }
        in_a::Expr::If {
            pred,
            th,
            el,
            block_tp,
        } => vec![],
        in_a::Expr::StructCons {
            id,
            initializers,
            tp,
        } => vec![],
        in_a::Expr::Assign {
            lval,
            rval,
            assign_tp,
        } => vec![],
        in_a::Expr::Ref { expr, tp } => vec![],
        in_a::Expr::RefMut { expr, tp } => vec![],
        in_a::Expr::Deref { expr, in_tp } => vec![],
        in_a::Expr::Error => vec![],
        in_a::Expr::Char(_) => vec![],
        in_a::Expr::String(_) => vec![],
        in_a::Expr::ArrayInitRepeat(expr, size, tp) => {
            let (elem_var, stmt) = env.var_decl(None, tp.clone());
            let mut stmts = vec![stmt];
            let init_stmts = tr_expr(env, st, *expr, &|v| {
                vec![out_a::Stmt::Assign {
                    lval: out_a::LValue::VarRef(elem_var),
                    rval: out_a::RValue::Value(out_a::LValue::VarRef(v)),
                }]
            });
            stmts.extend(init_stmts);
            let (arr_var, stmt) = env.var_decl(None, Type::array(size, tp));

            stmts
        }
        in_a::Expr::ArrayInitExact(exprs, tp) => vec![],
        in_a::Expr::While { pred, block } => {
            let (cond_var, stmt) = env.var_decl(None, Type::builtin("bool"));
            let mut stmts = vec![stmt];

            stmts.extend(tr_expr(env, st, *pred, &|v| {
                vec![out_a::Stmt::Assign {
                    lval: out_a::LValue::VarRef(cond_var),
                    rval: out_a::RValue::Value(out_a::LValue::VarRef(v)),
                }]
            }));

            stmts.push(out_a::Stmt::While {
                cond: cond_var,
                body: tr_expr(env, st, *block, &|_| vec![]),
            });

            let (last_var, stmt) = env.var_decl(None, Type::unit());
            stmts.push(stmt);
            stmts.push(out_a::Stmt::Assign {
                lval: out_a::LValue::VarRef(last_var),
                rval: out_a::RValue::Tuple(vec![]),
            });
            let last_stmts = cont(last_var);
            stmts.extend(last_stmts);
            stmts
        }
    }
}
