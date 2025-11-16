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

    let body = tr_expr(&mut env, f.body);

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

fn tr_expr(env: &mut Env, e: in_a::Expr) -> out_a::Expr {
    match e {
        in_a::Expr::NumLit(n, tp) => out_a::Expr::Value(ast::Value::Const(n, tr_type(tp))),
        in_a::Expr::StringLit(_, layout) => todo!(),
        in_a::Expr::Tuple { fields, layout } => {
            let ss = out_a::Expr::StackSlot {
                size: layout.size as u32,
            };
            let id = env.fresh_var();
            let var = out_a::VarRef::Local(id);
            let s_v = ast::Value::Var(var);
            let e = out_a::Expr::Value(s_v.clone());
            let e2 = fields.into_iter().rfold(e, |acc, (field, layout)| {
                let field = tr_expr(env, field);
                let st = out_a::Expr::Store {
                    ptr: Box::new(ast::Expr::Value(s_v.clone())),
                    val: Box::new(field),
                    offset: layout.offset as i32,
                };
                out_a::Expr::Ignore {
                    e1: Box::new(st),
                    e2: Box::new(acc),
                }
            });
            out_a::Expr::Let {
                id,
                e1: Box::new(ss),
                e2: Box::new(e2),
            }
        }
        in_a::Expr::FunCall {
            expr,
            args,
            args_tp,
            ret_tp,
        } => {
            let expr = tr_expr(env, *expr);

            match &ret_tp.layout {
                in_a::TypeLayout::Simple { tp } => {
                    let args = args.into_iter().map(|a| tr_expr(env, a)).collect();
                    let sig = make_sig(args_tp, ret_tp);
                    out_a::Expr::FunCall {
                        expr: Box::new(expr),
                        args,
                        sig,
                    }
                }
                in_a::TypeLayout::Array { elem_layout, elems } => todo!(),
                in_a::TypeLayout::Tuple {
                    field_count,
                    fields,
                } => {
                    todo!("sret is not implemented yet")
                }
            }
        }
        in_a::Expr::FieldAccess {
            object,
            field_id,
            struct_layout,
            element_layout,
        } => match element_layout.layout {
            in_a::TypeLayout::Simple { tp } => {
                let ptr = Box::new(tr_expr(env, *object));
                out_a::Expr::Load {
                    tp: tr_type(tp),
                    ptr,
                    offset: element_layout.offset as i32,
                }
            }
            in_a::TypeLayout::Array { elem_layout, elems } => todo!(),
            in_a::TypeLayout::Tuple {
                field_count,
                fields,
            } => todo!(),
        },
        in_a::Expr::Block {
            exprs,
            last_expr,
            block_tp,
        } => unreachable!(),
        in_a::Expr::Return { expr, ret_tp } => {
            let expr = tr_expr(env, *expr);
            out_a::Expr::Return {
                expr: Box::new(expr),
            }
        }
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

                out_a::Expr::Value(ast::Value::Var(ast::VarRef::Local(id)))
            }
            in_a::VarRef::Global(id) => {
                out_a::Expr::Value(ast::Value::Var(ast::VarRef::Global(id)))
            }
        },
        in_a::Expr::LetIn {
            id,
            layout,
            is_mut,
            expr,
            e2,
        } => {
            let e1 = tr_expr(env, *expr);
            let id = env.add_var(id);
            let e2 = tr_expr(env, *e2);

            out_a::Expr::Let {
                id,
                e1: Box::new(e1),
                e2: Box::new(e2),
            }
        }
        in_a::Expr::Ignore { e1, e2 } => {
            let e1 = tr_expr(env, *e1);
            let e2 = tr_expr(env, *e2);
            out_a::Expr::Ignore {
                e1: Box::new(e1),
                e2: Box::new(e2),
            }
        }
    }
}

fn make_sig(args_tp: Vec<in_a::Layout>, ret_tp: in_a::Layout) -> ast::FnSig {
    let mut params = vec![];
    let mut returns = vec![];
    match ret_tp.layout {
        in_a::TypeLayout::Simple { tp } => returns.push(tr_type(tp)),
        in_a::TypeLayout::Array { elem_layout, elems } => todo!(),
        in_a::TypeLayout::Tuple {
            field_count,
            fields,
        } => params.push(ast::Type::Tusize),
    }
    for arg in args_tp {
        match arg.layout {
            in_a::TypeLayout::Simple { tp } => params.push(tr_type(tp)),
            in_a::TypeLayout::Array { elem_layout, elems } => todo!(),
            in_a::TypeLayout::Tuple {
                field_count,
                fields,
            } => todo!(),
        }
    }
    ast::FnSig { params, returns }
}
