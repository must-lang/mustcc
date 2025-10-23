use crate::error::context::Context;

mod ast;
mod env;
mod error;

use crate::error::InternalError;
use crate::resolve::ast as in_a;
use crate::symtable::{SymKind, SymTable};
use crate::tp::{Type, TypeView, unify};
use crate::typecheck::env::Env;
use ast as out_a;

pub fn translate(ctx: &mut Context, prog: in_a::Program) -> Result<out_a::Program, InternalError> {
    let sym_table = prog.sym_table;

    let functions = prog
        .functions
        .into_iter()
        .map(|func| tr_func(ctx, &sym_table, func))
        .collect::<Result<_, _>>()?;

    let prog = out_a::Program {
        functions,
        sym_table,
    };
    Ok(prog)
}

fn tr_func(
    ctx: &mut Context,
    sym_table: &SymTable,
    func: in_a::Func,
) -> Result<out_a::Func, InternalError> {
    let mut env = Env::new(func.ret_type.clone());

    let args = func
        .args
        .into_iter()
        .map(|arg| {
            env.add_var(arg.name.clone(), arg.is_mut, arg.tp.clone());
            Ok((arg.name, arg.is_mut, arg.tp))
        })
        .collect::<Result<_, _>>()?;

    let body = check_expr(ctx, sym_table, &mut env, func.body, &func.ret_type, false)?;

    env.finish()?;

    let func = out_a::Func {
        name: func.name,
        id: func.id,
        args,
        ret_type: func.ret_type,
        body,
    };

    Ok(func)
}

fn check_expr(
    ctx: &mut Context,
    sym_table: &SymTable,
    env: &mut Env,
    expr: in_a::ExprNode,
    exp_tp: &Type,
    exp_mut: bool,
) -> Result<out_a::Expr, InternalError> {
    let pos = expr.pos;
    Ok(match expr.data {
        in_a::ExprData::Var(sym_ref) => match sym_ref {
            in_a::SymRef::Local(name) => {
                let (is_mut, tp) = env.lookup(&name);
                if exp_mut && !is_mut {
                    ctx.report(error::expected_mutable(pos.clone()));
                }
                if !unify(exp_tp, tp) {
                    ctx.report(error::type_mismatch(pos, exp_tp, tp));
                }
                out_a::Expr::LocalVar {
                    name,
                    tp: tp.clone(),
                }
            }
            in_a::SymRef::Global(node_id) => {
                let sym = sym_table.find_sym_info(node_id);
                match &sym.kind {
                    SymKind::Func { args, ret } => {
                        let tp = Type::fun(args.clone(), ret.clone());
                        if !unify(exp_tp, &tp) {
                            ctx.report(error::type_mismatch(pos, exp_tp, &tp));
                        }
                        out_a::Expr::GlobalVar {
                            id: node_id,
                            tp: tp.clone(),
                        }
                    }
                    SymKind::EnumCons { args, parent } => {
                        let sym = sym_table.find_sym_info(*parent);
                        let tp = match sym.kind {
                            SymKind::Enum(tv) => {
                                let tp = Type::named_var(tv, sym.name.clone());
                                if args.is_empty() {
                                    tp
                                } else {
                                    Type::fun(args.clone(), tp)
                                }
                            }
                            _ => unreachable!("parent of an enum cons is enum"),
                        };
                        if !unify(exp_tp, &tp) {
                            ctx.report(error::type_mismatch(pos, exp_tp, &tp));
                        }
                        out_a::Expr::GlobalVar { id: node_id, tp }
                    }
                    SymKind::BuiltinFunc {} => todo!(),
                    SymKind::Struct(tvar) => todo!(),
                    SymKind::Enum(tvar) => todo!(),
                }
            }
        },

        in_a::ExprData::FunCall(expr, expr_nodes) => {
            let fn_tp = Type::fresh_uvar();
            let expr_pos = expr.pos.clone();
            let ch_expr = check_expr(ctx, sym_table, env, *expr, &fn_tp, false)?;
            let (args_tp, ret) = match fn_tp.view() {
                TypeView::Fun(args, ret) => (args, ret),
                _ => {
                    ctx.report(error::not_a_function(expr_pos));
                    return Ok(out_a::Expr::Error);
                }
            };
            let mut args_iter = expr_nodes.into_iter();
            let mut id = 0;
            let args = args_tp
                .clone()
                .iter_mut()
                .map(|arg| {
                    id += 1;
                    if let Some(expr) = args_iter.next() {
                        check_expr(ctx, sym_table, env, expr, arg, false)
                    } else {
                        ctx.report(error::missing_argument(id, arg));
                        Ok(out_a::Expr::Error)
                    }
                })
                .collect::<Result<_, _>>()?;
            while let Some(arg) = args_iter.next() {
                id += 1;
                ctx.report(error::unexpected_argument(id, arg.pos));
            }
            if !unify(exp_tp, &ret) {
                ctx.report(error::type_mismatch(pos, exp_tp, &ret));
            }
            out_a::Expr::FunCall {
                expr: Box::new(ch_expr),
                args,
                args_tp,
                ret_tp: *ret,
            }
        }

        in_a::ExprData::FieldAccess(expr, field_name) => {
            let tp = Type::fresh_uvar();
            let expr = check_expr(ctx, sym_table, env, *expr, &tp, exp_mut)?;
            let field_tp = match tp.view() {
                TypeView::NamedVar(tvar, _) | TypeView::Var(tvar) => {
                    let type_info = sym_table.find_type_info(tvar);
                    match type_info {
                        crate::symtable::TypeInfo::Struct {
                            name,
                            pos: _,
                            fields,
                            methods,
                        } => match fields.get(&field_name) {
                            Some(tp) => tp,
                            None => {
                                ctx.report(error::no_such_field(field_name, &tp, &pos));
                                return Ok(out_a::Expr::Error);
                            }
                        },
                        _ => {
                            ctx.report(error::no_such_field(field_name, &tp, &pos));
                            return Ok(out_a::Expr::Error);
                        }
                    }
                }
                _ => {
                    ctx.report(error::no_such_field(field_name, &tp, &pos));
                    return Ok(out_a::Expr::Error);
                }
            };
            if !unify(exp_tp, field_tp) {
                ctx.report(error::type_mismatch(pos, exp_tp, field_tp));
            }
            out_a::Expr::FieldAccess {
                object: Box::new(expr),
                field_name,
                field_tp: field_tp.clone(),
            }
        }

        in_a::ExprData::Return(expr) => {
            let tp = env.expected_ret();
            let expr = check_expr(ctx, sym_table, env, *expr, &tp, false)?;
            if !unify(exp_tp, &Type::never()) {
                unreachable!("never always coerces")
            };
            out_a::Expr::Return {
                expr: Box::new(expr),
                ret_tp: tp,
            }
        }

        in_a::ExprData::Block(expr_nodes, expr) => {
            env.new_scope();
            let exprs = expr_nodes
                .into_iter()
                .map(|expr| check_expr(ctx, sym_table, env, expr, &Type::unit(), false))
                .collect::<Result<_, _>>()?;
            let expr = check_expr(ctx, sym_table, env, *expr, exp_tp, exp_mut)?;
            env.leave_scope();
            out_a::Expr::Block {
                exprs,
                last_expr: Box::new(expr),
                block_tp: exp_tp.clone(),
            }
        }

        in_a::ExprData::Let {
            name,
            is_mut,
            tp,
            expr,
        } => {
            let tp = match tp {
                Some(tp) => tp,
                None => Type::fresh_uvar(),
            };
            env.new_scope();
            let expr = check_expr(ctx, sym_table, env, *expr, &tp, false)?;
            env.leave_scope();
            env.add_var(name.clone(), is_mut, tp.clone());
            if !unify(exp_tp, &Type::unit()) {
                ctx.report(error::type_mismatch(pos, exp_tp, &Type::unit()));
            };
            out_a::Expr::Let {
                name,
                is_mut,
                tp,
                expr: Box::new(expr),
            }
        }

        in_a::ExprData::If(pr, th, el) => {
            let tp = Type::fresh_uvar();
            let pr = check_expr(ctx, sym_table, env, *pr, &Type::bool(), false)?;
            let el = check_expr(ctx, sym_table, env, *el, &tp, exp_mut)?;
            let th = check_expr(ctx, sym_table, env, *th, &tp, exp_mut)?;
            if !unify(exp_tp, &tp) {
                ctx.report(error::type_mismatch(pos, exp_tp, &tp));
            };
            out_a::Expr::If {
                pred: Box::new(pr),
                th: Box::new(th),
                el: Box::new(el),
                block_tp: tp,
            }
        }

        in_a::ExprData::Assign(lval, rval) => {
            let tp = Type::fresh_uvar();
            let lval = check_expr(ctx, sym_table, env, *lval, &tp, true)?;
            let rval = check_expr(ctx, sym_table, env, *rval, &tp, false)?;
            if !unify(exp_tp, &Type::unit()) {
                ctx.report(error::type_mismatch(pos, exp_tp, &Type::unit()));
            }
            out_a::Expr::Assign {
                lval: Box::new(lval),
                rval: Box::new(rval),
                assign_tp: tp,
            }
        }

        in_a::ExprData::Ref(expr_node) => {
            let tp = Type::fresh_uvar();
            let expr = check_expr(ctx, sym_table, env, *expr_node, &tp, false)?;
            let tp = Type::ptr(tp);
            if !unify(exp_tp, &tp) {
                ctx.report(error::type_mismatch(pos, exp_tp, &tp));
            }
            out_a::Expr::Ref {
                expr: Box::new(expr),
                tp: tp,
            }
        }

        in_a::ExprData::RefMut(expr_node) => {
            let tp = Type::fresh_uvar();
            let expr = check_expr(ctx, sym_table, env, *expr_node, &tp, true)?;
            let tp = Type::mut_ptr(tp);
            if !unify(exp_tp, &tp) {
                ctx.report(error::type_mismatch(pos, exp_tp, &tp));
            }
            out_a::Expr::RefMut {
                expr: Box::new(expr),
                tp: tp,
            }
        }

        in_a::ExprData::Deref(expr_node) => {
            let in_tp = Type::fresh_uvar();
            let tp = if exp_mut {
                Type::mut_ptr(in_tp.clone())
            } else {
                Type::ptr(in_tp.clone())
            };
            let expr = check_expr(ctx, sym_table, env, *expr_node, &tp, false)?;
            if !unify(exp_tp, &in_tp) {
                ctx.report(error::type_mismatch(pos, exp_tp, &in_tp));
            }
            out_a::Expr::Deref {
                expr: Box::new(expr),
                in_tp: in_tp,
            }
        }

        in_a::ExprData::NumLit(lit) => {
            let tp = Type::numeric_uvar();
            if !unify(exp_tp, &tp) {
                ctx.report(error::type_mismatch(pos, exp_tp, &tp));
            }
            out_a::Expr::NumLit(lit, tp)
        }

        in_a::ExprData::Tuple(exprs) => {
            let mut tps = vec![];
            let mut ch_exprs = vec![];
            for expr in exprs {
                let tp = Type::fresh_uvar();
                let expr = check_expr(ctx, sym_table, env, expr, &tp, exp_mut)?;
                tps.push(tp);
                ch_exprs.push(expr);
            }
            out_a::Expr::Tuple(ch_exprs)
        }

        _ => todo!(),
    })
}
