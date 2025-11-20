pub mod ast;
mod env;

use std::mem::transmute;

use crate::{
    core::env::Env,
    mir::ast as in_a,
    symtable::layout::{Layout, LayoutKind},
};
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
        args.push((id, tp));
    }
    let returns = f.returns;

    let body = tr_expr(&mut env, f.body);

    out_a::Func {
        id: f.id,
        args,
        returns,
        body,
    }
}

fn tr_expr(env: &mut Env, e: in_a::Expr) -> out_a::Expr {
    match e {
        in_a::Expr::NumLit(n, tp) => out_a::Expr::Value(ast::Value::Const(n, tp)),
        in_a::Expr::StringLit(_, layout) => todo!(),
        in_a::Expr::Tuple { fields, layout } => {
            let ss = out_a::Expr::StackSlot {
                size: layout.size as u32,
            };
            let id = env.fresh_var();
            let var = out_a::VarRef::Local(id);
            let s_v = ast::Value::Var(var);
            let e = out_a::Expr::Value(s_v.clone());
            let mut exprs = vec![out_a::Expr::Let {
                id,
                e1: Box::new(ss),
            }];
            let layouts = match layout.kind {
                LayoutKind::Primitive(_) => todo!(),
                LayoutKind::Struct(items) => items,
                LayoutKind::Union(layouts) => todo!(),
            };
            for (id, field) in fields.into_iter().enumerate() {
                let field = tr_expr(env, field);
                let (layout, offset) = layouts[id].clone();
                let st = out_a::Expr::Store {
                    ptr: Box::new(ast::Expr::Value(s_v.clone())),
                    val: Box::new(field),
                    offset,
                };
                exprs.push(st);
            }
            out_a::Expr::Block {
                exprs,
                last_expr: Box::new(e),
            }
        }
        in_a::Expr::FunCall {
            expr,
            args,
            args_tp,
            ret_tp,
        } => {
            let expr = tr_expr(env, *expr);

            match &ret_tp.kind {
                LayoutKind::Primitive(tp) => {
                    let args = args.into_iter().map(|a| tr_expr(env, a)).collect();
                    let sig = make_sig(args_tp, ret_tp);
                    out_a::Expr::FunCall {
                        expr: Box::new(expr),
                        args,
                        sig,
                    }
                }
                LayoutKind::Struct(_) => {
                    todo!("sret is not implemented yet")
                }
                LayoutKind::Union(layouts) => todo!(),
            }
        }
        in_a::Expr::FieldAccess {
            object,
            field_id,
            struct_layout,
            element_layout,
        } => match struct_layout.kind {
            LayoutKind::Primitive(tp) => todo!(),
            LayoutKind::Struct(items) => {
                let (layout, offset) = items[field_id].clone();
                let ptr = Box::new(tr_expr(env, *object));
                match layout.kind {
                    LayoutKind::Primitive(tp) => out_a::Expr::Load {
                        tp,
                        ptr,
                        offset: offset,
                    },
                    LayoutKind::Struct(items) => todo!(),
                    LayoutKind::Union(layouts) => todo!(),
                }
            }
            LayoutKind::Union(layouts) => todo!(),
        },
        in_a::Expr::Block {
            exprs,
            last_expr,
            block_tp,
        } => {
            let exprs = exprs.into_iter().map(|e| tr_expr(env, e)).collect();
            let last_expr = tr_expr(env, *last_expr);
            out_a::Expr::Block {
                exprs,
                last_expr: Box::new(last_expr),
            }
        }
        in_a::Expr::Return { expr, ret_tp } => {
            let expr = tr_expr(env, *expr);
            out_a::Expr::Return {
                expr: Box::new(expr),
            }
        }
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
        in_a::Expr::Let {
            id,
            layout,
            is_mut,
            expr,
        } => {
            let e1 = tr_expr(env, *expr);
            let id = env.add_var(id);
            out_a::Expr::Let {
                id,
                e1: Box::new(e1),
            }
        }
        in_a::Expr::Builtin(name, exprs) => {
            let args = exprs.into_iter().map(|a| tr_expr(env, a)).collect();
            out_a::Expr::Builtin { name, args }
        }
    }
}

fn make_sig(args_tp: Vec<Layout>, ret_tp: Layout) -> ast::FnSig {
    let mut params = vec![];
    let mut returns = vec![];
    match ret_tp.kind {
        LayoutKind::Primitive(tp) => returns.push(tp),
        LayoutKind::Struct(items) => todo!(),
        LayoutKind::Union(layouts) => todo!(),
    }
    for arg in args_tp {
        match arg.kind {
            LayoutKind::Primitive(tp) => params.push(tp),
            LayoutKind::Struct(items) => todo!(),
            LayoutKind::Union(layouts) => todo!(),
        }
    }
    ast::FnSig { params, returns }
}
