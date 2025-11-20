pub mod ast;
mod env;
use std::collections::HashMap;

use crate::{
    error::InternalError,
    mir::{
        ast::{VarID, VarRef},
        env::Env,
    },
    symtable::{
        SymTable,
        layout::{LayoutKind, Type},
    },
    typecheck::ast as in_a,
};
use ast as out_a;

pub(crate) fn translate(prog: in_a::Program) -> Result<out_a::Program, InternalError> {
    let st = prog.sym_table;

    let functions = prog
        .functions
        .into_iter()
        .map(|f| tr_func(&st, f))
        .collect::<Result<_, _>>()?;

    let prog = out_a::Program {
        symbols: make_symtable(st),
        functions,
    };
    Ok(prog)
}

fn make_symtable(st: SymTable) -> HashMap<crate::common::NodeID, ast::Symbol> {
    let mut map = HashMap::new();
    for (id, info) in st.get_items() {
        let kind = match &info.kind {
            crate::symtable::SymKind::Func {
                params,
                args: old_args,
                ret,
            } => {
                let mut args = vec![];
                let mut returns = vec![];
                for tp in old_args {
                    let layout = st.get_layout(&tp.clone());
                    let tp = match layout.kind {
                        LayoutKind::Primitive(tp) => tp,
                        LayoutKind::Struct(items) => Type::Tusize,
                        LayoutKind::Union(layouts) => Type::Tusize,
                    };
                    args.push(tp)
                }
                {
                    let layout = st.get_layout(&ret.clone());
                    match layout.kind {
                        LayoutKind::Primitive(tp) => returns.push(tp),
                        LayoutKind::Struct(items) => args.push(Type::Tusize),
                        LayoutKind::Union(layouts) => args.push(Type::Tusize),
                    };
                }
                out_a::SymKind::Func { args, returns }
            }
            crate::symtable::SymKind::Struct(tvar) => continue,
            crate::symtable::SymKind::Enum(tvar) => continue,
            crate::symtable::SymKind::EnumCons { id, args, parent } => continue,
        };
        let new_info = out_a::Symbol {
            name: info.name.clone(),
            kind,
            is_extern: info.is_extern,
            mangle: info.mangle,
        };
        map.insert(*id, new_info);
    }
    map
}

fn tr_func(st: &SymTable, f: in_a::Func) -> Result<out_a::Func, InternalError> {
    let mut args = vec![];
    let mut returns = vec![];
    let mut env = Env::new();
    let mut var_needs_stack = HashMap::new();
    for (name, is_mut, tp) in f.args {
        let layout = st.get_layout(&tp);
        let var_id = env.add_var(name);
        let tp = match layout.kind {
            LayoutKind::Primitive(tp) => tp,
            _ => {
                var_needs_stack.insert(var_id, true);
                Type::Tusize
            }
        };
        args.push((var_id, is_mut, tp))
    }
    {
        let layout = st.get_layout(&f.ret_type);
        match layout.kind {
            LayoutKind::Primitive(tp) => returns.push(tp),
            LayoutKind::Struct(items) => {
                let name = "__ret_var".into();
                let id = env.add_var(name);
                var_needs_stack.insert(id, true);
                args.push((id, false, Type::Tusize))
            }
            LayoutKind::Union(layouts) => todo!(),
        };
    }

    let body = tr_expr(&mut env, &mut var_needs_stack, st, f.body)?;

    let func = out_a::Func {
        id: f.id,
        args,
        returns,
        body,
        var_needs_stack,
    };
    Ok(func)
}

fn tr_expr(
    env: &mut Env,
    vns: &mut HashMap<VarID, bool>,
    st: &SymTable,
    e: in_a::Expr,
) -> Result<ast::Expr, InternalError> {
    Ok(match e {
        in_a::Expr::NumLit(n, tp) => {
            let tp = match st.get_layout(&tp).kind {
                LayoutKind::Primitive(tp) => tp,
                LayoutKind::Struct(items) => unreachable!(),
                LayoutKind::Union(layouts) => unreachable!(),
            };
            out_a::Expr::NumLit(n, tp)
        }
        in_a::Expr::StringLit(_, _) => todo!(),
        in_a::Expr::LocalVar { name, tp } => {
            let id = env.lookup(&name);
            let var = out_a::VarRef::Local(id);
            out_a::Expr::Var(var)
        }
        in_a::Expr::GlobalVar { id, tp } => {
            let var = out_a::VarRef::Global(id);
            out_a::Expr::Var(var)
        }
        in_a::Expr::Tuple(exprs, tp) => {
            let mut fields = vec![];
            let layout = st.get_layout(&tp);
            let mut id = 0;
            for expr in exprs.into_iter() {
                let expr = tr_expr(env, vns, st, expr)?;
                let layout = match &layout.kind {
                    LayoutKind::Primitive(_) => todo!(),
                    LayoutKind::Struct(items) => items[id].clone(),
                    LayoutKind::Union(layouts) => todo!(),
                };
                id += 1;
                fields.push(expr)
            }
            out_a::Expr::Tuple { fields, layout }
        }
        in_a::Expr::FunCall {
            expr,
            args,
            args_tp,
            ret_tp,
        } => {
            let callee = tr_expr(env, vns, st, *expr)?;
            let args = args
                .into_iter()
                .map(|e| tr_expr(env, vns, st, e))
                .collect::<Result<_, _>>()?;
            let args_tp = args_tp.into_iter().map(|tp| st.get_layout(&tp)).collect();
            let ret_tp = st.get_layout(&ret_tp);
            out_a::Expr::FunCall {
                expr: Box::new(callee),
                args,
                args_tp,
                ret_tp,
            }
        }
        in_a::Expr::FieldAccess {
            object,
            field_id,
            struct_tp,
            field_tp,
        } => {
            let object = Box::new(tr_expr(env, vns, st, *object)?);
            let struct_layout = st.get_layout(&struct_tp);
            let element_layout = st.get_layout(&field_tp);
            out_a::Expr::FieldAccess {
                object,
                field_id,
                struct_layout,
                element_layout,
            }
        }
        in_a::Expr::Block {
            exprs,
            last_expr,
            block_tp,
        } => {
            let exprs = exprs
                .into_iter()
                .map(|e| tr_expr(env, vns, st, e))
                .collect::<Result<_, _>>()?;
            let last_expr = Box::new(tr_expr(env, vns, st, *last_expr)?);
            let block_tp = st.get_layout(&block_tp);
            out_a::Expr::Block {
                exprs,
                last_expr,
                block_tp,
            }
        }
        in_a::Expr::Return { expr, ret_tp } => {
            let layout = st.get_layout(&ret_tp);
            let expr = tr_expr(env, vns, st, *expr)?;
            match &layout.kind {
                LayoutKind::Primitive(tp) => out_a::Expr::Return {
                    expr: Box::new(expr),
                    ret_tp: tp.clone(),
                },
                LayoutKind::Struct(items) => {
                    let ret_v = env.lookup("__ret_var");
                    let lval = Box::new(out_a::Expr::Var(VarRef::Local(ret_v)));
                    out_a::Expr::Assign {
                        lval,
                        rval: Box::new(expr),
                        assign_tp: (layout),
                    }
                }
                LayoutKind::Union(layouts) => todo!(),
            }
        }
        in_a::Expr::Let {
            name,
            tp,
            is_mut,
            expr,
        } => {
            let id = env.add_var(name);
            let layout = st.get_layout(&tp);
            let expr = tr_expr(env, vns, st, *expr)?;
            vns.insert(id, layout.require_stack());
            out_a::Expr::Let {
                id,
                layout,
                is_mut,
                expr: Box::new(expr),
            }
        }
        in_a::Expr::StructCons {
            id,
            initializers,
            tp,
        } => {
            let mut fields = vec![];
            let layout = st.get_layout(&tp);
            for (_, (id, expr)) in initializers {
                let expr = tr_expr(env, vns, st, expr)?;
                fields.push((id, expr))
            }
            fields.sort_by_key(|(k, _)| *k);
            let fields = fields.into_iter().map(|(_, e)| e).collect();
            out_a::Expr::Tuple { fields, layout }
        }
        in_a::Expr::Assign {
            lval,
            rval,
            assign_tp,
        } => {
            let lval = Box::new(tr_expr(env, vns, st, *lval)?);
            let rval = Box::new(tr_expr(env, vns, st, *rval)?);
            let layout = st.get_layout(&assign_tp);
            out_a::Expr::Assign {
                lval: lval,
                rval: rval,
                assign_tp: layout,
            }
        }
        in_a::Expr::Ref { expr, tp } => todo!(),
        in_a::Expr::RefMut { expr, tp } => todo!(),
        in_a::Expr::Deref { expr, in_tp } => todo!(),
        in_a::Expr::Error => todo!(),
        in_a::Expr::Char(_) => todo!(),
        in_a::Expr::ArrayInitRepeat(expr, n, tp) => {
            let e = tr_expr(env, vns, st, *expr)?;
            let layout = st.get_layout(&tp);
            out_a::Expr::ArrayInitRepeat(Box::new(e), n, layout)
        }
        in_a::Expr::ArrayInitExact(exprs, _) => todo!(),
        in_a::Expr::While { pred, block } => {
            let pred = tr_expr(env, vns, st, *pred)?;
            let block = tr_expr(env, vns, st, *block)?;
            out_a::Expr::While {
                pred: Box::new(pred),
                block: Box::new(block),
            }
        }
        in_a::Expr::IndexAccess { arr, index, tp } => todo!(),
        in_a::Expr::If {
            pred,
            th,
            el,
            block_tp,
        } => todo!(),
        in_a::Expr::Builtin(name, args) => {
            let args = args
                .into_iter()
                .map(|e| tr_expr(env, vns, st, e))
                .collect::<Result<_, _>>()?;
            out_a::Expr::Builtin(name, args)
        }
    })
}
