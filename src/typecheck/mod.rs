use std::collections::HashMap;

use crate::error::context::Context;

pub mod ast;
mod env;
mod error;

use crate::error::InternalError;
use crate::resolve::ast as in_a;
use crate::symtable::{SymKind, SymTable, TypeKind, TypeSize};
use crate::tp::{TVar, Type, TypeView, unify};
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
        .collect::<Result<_, InternalError>>()?;

    let body = check_expr(ctx, sym_table, &mut env, func.body, &func.ret_type, false)?;

    env.finish(ctx)?;

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
    let pos = &expr.pos;
    Ok(match expr.data {
        in_a::ExprData::Var(sym_ref) => match sym_ref {
            in_a::SymRef::Local(name) => {
                let (is_mut, tp) = env.lookup(&name);
                if exp_mut && !is_mut {
                    ctx.report(error::expected_mutable(pos));
                }
                if !unify(exp_tp, tp) {
                    ctx.report(error::type_mismatch(pos, exp_tp.clone(), tp.clone()));
                }
                out_a::Expr::LocalVar {
                    name,
                    tp: tp.clone(),
                }
            }
            in_a::SymRef::Global(node_id) => {
                let sym = sym_table.find_sym_info(node_id);
                match &sym.kind {
                    SymKind::Func { params, args, ret } => {
                        let subst: HashMap<TVar, Type> = params
                            .iter()
                            .map(|tv| (*tv, env.fresh_uvar(&pos)))
                            .collect();
                        let tp = Type::fun(args.clone(), ret.clone()).substitute(&subst);
                        if !unify(exp_tp, &tp) {
                            ctx.report(error::type_mismatch(pos, exp_tp.clone(), tp.clone()));
                        }
                        out_a::Expr::GlobalVar {
                            id: node_id,
                            tp: tp.clone(),
                        }
                    }
                    SymKind::EnumCons { id, args, parent } => {
                        let sym_info = sym_table.find_sym_info(*parent);
                        let (params, tvar, name) = match &sym_info.kind {
                            SymKind::Func { params, args, ret } => todo!(),
                            SymKind::Enum(tvar) => {
                                let type_info = sym_table.find_type_info(*tvar);
                                match &type_info.kind {
                                    TypeKind::Enum {
                                        params,
                                        constructors,
                                    } => (params, tvar, type_info.name.clone()),
                                    _ => unreachable!("this is 100% a struct"),
                                }
                            }
                            SymKind::EnumCons { id, args, parent } => todo!(),
                            SymKind::Struct(tvar) => todo!(),
                        };
                        let subst: HashMap<TVar, Type> = params
                            .iter()
                            .map(|tv| (*tv, env.fresh_uvar(&pos)))
                            .collect();
                        let tp = unsafe {
                            if params.len() == 0 {
                                Type::named_var(*tvar, &name, &pos).unwrap_unchecked()
                            } else {
                                Type::type_app(
                                    *tvar,
                                    &name,
                                    subst.values().map(|tp| tp.clone()).collect(),
                                    &pos,
                                )
                                .unwrap_unchecked()
                            }
                        };
                        let tp = if args.is_empty() {
                            tp
                        } else {
                            Type::fun(args.clone(), tp).substitute(&subst)
                        };
                        if !unify(exp_tp, &tp) {
                            ctx.report(error::type_mismatch(pos, exp_tp.clone(), tp.clone()));
                        }
                        out_a::Expr::GlobalVar { id: node_id, tp }
                    }
                    SymKind::Struct(tvar) => todo!(),
                    SymKind::Enum(tvar) => todo!(),
                }
            }
        },
        in_a::ExprData::FunCall(expr, expr_nodes) => {
            let fn_tp = env.fresh_uvar(&pos);
            let ref expr_pos = expr.pos.clone();
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
                        ctx.report(error::missing_argument(pos, id, arg.clone()));
                        Ok(out_a::Expr::Error)
                    }
                })
                .collect::<Result<_, _>>()?;
            while let Some(arg) = args_iter.next() {
                id += 1;
                ctx.report(error::unexpected_argument(id, &arg.pos));
            }
            if !unify(exp_tp, &ret) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), *ret.clone()));
            }
            out_a::Expr::FunCall {
                expr: Box::new(ch_expr),
                args,
                args_tp,
                ret_tp: *ret,
            }
        }
        in_a::ExprData::FieldAccess(expr, field_name) => {
            let tp = env.fresh_uvar(&pos);
            let expr = check_expr(ctx, sym_table, env, *expr, &tp, exp_mut)?;
            let field_tp = match tp.view() {
                TypeView::NamedVar(tvar, _) | TypeView::Var(tvar) => {
                    let type_info = sym_table.find_type_info(tvar);
                    match &type_info.kind {
                        TypeKind::Struct { params, fields } => match fields.get(&field_name) {
                            Some(tp) => tp,
                            None => {
                                ctx.report(error::no_such_field(field_name, tp, &pos));
                                return Ok(out_a::Expr::Error);
                            }
                        },
                        _ => {
                            ctx.report(error::no_such_field(field_name, tp, &pos));
                            return Ok(out_a::Expr::Error);
                        }
                    }
                }
                _ => {
                    ctx.report(error::no_such_field(field_name, tp, &pos));
                    return Ok(out_a::Expr::Error);
                }
            };
            if !unify(exp_tp, field_tp) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), field_tp.clone()));
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
            if !unify(exp_tp, &Type::builtin("never")) {
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
                None => env.fresh_uvar(&pos),
            };
            env.new_scope();
            let expr = check_expr(ctx, sym_table, env, *expr, &tp, false)?;
            env.leave_scope();
            env.add_var(name.clone(), is_mut, tp.clone());
            if !unify(exp_tp, &Type::unit()) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), Type::unit()));
            };
            out_a::Expr::Let {
                name,
                is_mut,
                tp,
                expr: Box::new(expr),
            }
        }
        in_a::ExprData::If(pr, th, el) => {
            let tp = env.fresh_uvar(&pos);
            let pr = check_expr(ctx, sym_table, env, *pr, &Type::builtin("bool"), false)?;
            let el = check_expr(ctx, sym_table, env, *el, &tp, exp_mut)?;
            let th = check_expr(ctx, sym_table, env, *th, &tp, exp_mut)?;
            if !unify(exp_tp, &tp) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), tp.clone()));
            };
            out_a::Expr::If {
                pred: Box::new(pr),
                th: Box::new(th),
                el: Box::new(el),
                block_tp: tp,
            }
        }
        in_a::ExprData::Assign(lval, rval) => {
            let tp = env.fresh_uvar(&pos);
            let lval = check_expr(ctx, sym_table, env, *lval, &tp, true)?;
            let rval = check_expr(ctx, sym_table, env, *rval, &tp, false)?;
            if !unify(exp_tp, &Type::unit()) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), Type::unit()));
            }
            out_a::Expr::Assign {
                lval: Box::new(lval),
                rval: Box::new(rval),
                assign_tp: tp,
            }
        }
        in_a::ExprData::Ref(expr_node) => {
            let tp = env.fresh_uvar(&pos);
            let expr = check_expr(ctx, sym_table, env, *expr_node, &tp, false)?;
            let tp = Type::ptr(tp);
            if !unify(exp_tp, &tp) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), tp.clone()));
            }
            out_a::Expr::Ref {
                expr: Box::new(expr),
                tp: tp,
            }
        }
        in_a::ExprData::RefMut(expr_node) => {
            let tp = env.fresh_uvar(&pos);
            let expr = check_expr(ctx, sym_table, env, *expr_node, &tp, true)?;
            let tp = Type::mut_ptr(tp);
            if !unify(exp_tp, &tp) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), tp.clone()));
            }
            out_a::Expr::RefMut {
                expr: Box::new(expr),
                tp: tp,
            }
        }
        in_a::ExprData::Deref(expr_node) => {
            let in_tp = env.fresh_uvar(&pos);
            let tp = if exp_mut {
                Type::mut_ptr(in_tp.clone())
            } else {
                Type::ptr(in_tp.clone())
            };
            let expr = check_expr(ctx, sym_table, env, *expr_node, &tp, false)?;
            if !unify(exp_tp, &in_tp) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), in_tp.clone()));
            }
            match sym_table.sizeof(&in_tp) {
                TypeSize::Sized(_) => (),
                TypeSize::Unsized => {
                    ctx.report(error::unsized_type(&pos));
                }
                TypeSize::Unknown => (),
                TypeSize::NotUnified => {
                    ctx.report(error::cannot_infer_type(&pos));
                }
            }
            out_a::Expr::Deref {
                expr: Box::new(expr),
                in_tp: in_tp,
            }
        }
        in_a::ExprData::NumLit(lit) => {
            let tp = env.numeric_uvar(&pos);
            if !unify(exp_tp, &tp) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), tp.clone()));
            }
            out_a::Expr::NumLit(lit, tp)
        }
        in_a::ExprData::Tuple(exprs) => {
            let mut tps = vec![];
            let mut ch_exprs = vec![];
            for expr in exprs {
                let tp = env.fresh_uvar(&pos);
                let expr = check_expr(ctx, sym_table, env, expr, &tp, exp_mut)?;
                tps.push(tp);
                ch_exprs.push(expr);
            }
            let tp = Type::tuple(tps);
            if !unify(exp_tp, &tp) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), tp.clone()));
            }
            out_a::Expr::Tuple(ch_exprs)
        }
        in_a::ExprData::String(s) => {
            let size = s.as_bytes().len();
            let tp = Type::ptr(Type::array(size, Type::builtin("u8")));
            if !unify(exp_tp, &tp) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), tp));
            }
            out_a::Expr::String(s)
        }
        in_a::ExprData::MethodCall(expr, method_name, exprs) => {
            let tp = env.fresh_uvar(&pos);
            let expr = check_expr(ctx, sym_table, env, *expr, &tp, false)?;
            let method_id = match tp.view() {
                TypeView::Var(tvar) | TypeView::NamedVar(tvar, _) => {
                    let type_info = sym_table.find_type_info(tvar);
                    match type_info.methods.get(&method_name) {
                        Some(m) => *m,
                        None => {
                            ctx.report(error::unbound_method(pos, method_name));
                            return Ok(out_a::Expr::Error);
                        }
                    }
                }
                TypeView::Unknown => return Ok(out_a::Expr::Error),
                TypeView::UVar(uvar) | TypeView::NumericUVar(uvar) => {
                    ctx.report(error::unsolved_uvar(pos, tp));
                    return Ok(out_a::Expr::Error);
                }
                TypeView::Tuple(items) => todo!(),
                TypeView::Array(_, _) => todo!(),
                TypeView::Fun(items, _) => todo!(),
                TypeView::Ptr(tp) | TypeView::MutPtr(tp) => match tp.view() {
                    TypeView::Var(tvar) | TypeView::NamedVar(tvar, _) => {
                        let type_info = sym_table.find_type_info(tvar);
                        match type_info.methods.get(&method_name) {
                            Some(m) => *m,
                            None => {
                                ctx.report(error::unbound_method(pos, method_name));
                                return Ok(out_a::Expr::Error);
                            }
                        }
                    }
                    TypeView::Unknown => return Ok(out_a::Expr::Error),
                    TypeView::UVar(uvar) | TypeView::NumericUVar(uvar) => {
                        ctx.report(error::unsolved_uvar(pos, *tp));
                        return Ok(out_a::Expr::Error);
                    }
                    TypeView::Tuple(items) => todo!(),
                    TypeView::Array(_, _) => todo!(),
                    TypeView::Fun(items, _) => todo!(),
                    TypeView::Ptr(tp) | TypeView::MutPtr(tp) => todo!(),
                    TypeView::TypeApp(tvar, _, items) => todo!(),
                },
                TypeView::TypeApp(tvar, _, items) => todo!(),
            };
            let method_info = sym_table.find_sym_info(method_id);
            let (mut args_tp, ret_tp) = match &method_info.kind {
                SymKind::Func { params, args, ret } => (args.clone(), ret.clone()),
                _ => panic!("not a function"),
            };
            let first_arg = match args_tp.get(0) {
                Some(tp) => tp,
                None => {
                    ctx.report(error::unexpected_argument(1, pos));
                    return Ok(out_a::Expr::Error);
                }
            };
            if !unify(first_arg, &tp) {
                ctx.report(error::type_mismatch(pos, first_arg.clone(), tp));
            }
            let mut args_iter = exprs.into_iter();
            let mut id = 1;
            let mut args = vec![expr];
            for arg in args_tp[1..].iter() {
                id += 1;
                let arg = if let Some(expr) = args_iter.next() {
                    check_expr(ctx, sym_table, env, expr, arg, false)?
                } else {
                    ctx.report(error::missing_argument(pos, id, arg.clone()));
                    out_a::Expr::Error
                };
                args.push(arg)
            }
            while let Some(arg) = args_iter.next() {
                id += 1;
                ctx.report(error::unexpected_argument(id, &arg.pos));
            }
            if !unify(exp_tp, &ret_tp) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), ret_tp.clone()));
            }
            let method_tp = Type::fun(args_tp.clone(), ret_tp.clone());
            let callee = out_a::Expr::GlobalVar {
                id: method_id,
                tp: method_tp,
            };

            out_a::Expr::FunCall {
                expr: Box::new(callee),
                args,
                args_tp: args_tp.clone(),
                ret_tp: ret_tp.clone(),
            }
        }
        in_a::ExprData::StructCons(id, mut items) => {
            let sym_info = sym_table.find_sym_info(id);
            let (params, tvar, name, fields) = match &sym_info.kind {
                SymKind::Func { params, args, ret } => todo!(),
                SymKind::Enum(tvar) => todo!(),
                SymKind::EnumCons { id, args, parent } => todo!(),
                SymKind::Struct(tvar) => {
                    let type_info = sym_table.find_type_info(*tvar);
                    match &type_info.kind {
                        TypeKind::Struct { params, fields } => {
                            (params, tvar, type_info.name.clone(), fields)
                        }
                        _ => unreachable!("this is 100% a struct"),
                    }
                }
            };
            let subst: HashMap<TVar, Type> = params
                .iter()
                .map(|tv| (*tv, env.fresh_uvar(&pos)))
                .collect();
            let mut initializers = HashMap::new();
            for (f_name, f_type) in fields {
                let tp = f_type.substitute(&subst);
                match items.remove(f_name) {
                    Some(expr) => {
                        let expr = check_expr(ctx, sym_table, env, expr, &tp, false)?;
                        initializers.insert(f_name.clone(), expr);
                    }
                    None => {
                        ctx.report(error::missing_field(pos, f_name.clone(), tp));
                    }
                }
            }
            for (f_name, expr) in items {
                // check anyways to report errors
                let tp = env.fresh_uvar(&pos);
                let _ = check_expr(ctx, sym_table, env, expr, &tp, false)?;
                ctx.report(error::unbound_field(pos, f_name));
            }
            let tp = unsafe {
                if params.len() == 0 {
                    Type::named_var(*tvar, &name, &pos).unwrap_unchecked()
                } else {
                    Type::type_app(
                        *tvar,
                        &name,
                        subst.values().map(|tp| tp.clone()).collect(),
                        &pos,
                    )
                    .unwrap_unchecked()
                }
            };
            if !unify(exp_tp, &tp) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), tp.clone()));
            }
            out_a::Expr::StructCons {
                id,
                initializers,
                tp,
            }
        }
        in_a::ExprData::Error => out_a::Expr::Error,
        in_a::ExprData::IndexAccess(arr, index) => {
            let tp = env.fresh_uvar(&pos);
            let arr = check_expr(ctx, sym_table, env, *arr, &tp, exp_mut)?;
            let index = check_expr(ctx, sym_table, env, *index, &Type::builtin("usize"), false)?;
            let tp = match tp.view() {
                TypeView::Array(_, tp) => *tp,
                TypeView::Unknown => todo!(),
                TypeView::UVar(uvar) | TypeView::NumericUVar(uvar) => {
                    todo!()
                    // cannot infer type
                }
                TypeView::Var(_)
                | TypeView::NamedVar(_, _)
                | TypeView::Tuple(_)
                | TypeView::Fun(_, _)
                | TypeView::Ptr(_)
                | TypeView::MutPtr(_) => {
                    todo!()
                    // type mismatch, expected array
                }
                TypeView::TypeApp(tvar, _, items) => todo!(),
            };
            if !unify(exp_tp, &tp) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), tp));
            }
            out_a::Expr::While {
                pred: Box::new(arr),
                block: Box::new(index),
            }
        }
        in_a::ExprData::Match(expr_node, match_clauses) => {
            ctx.report(error::not_yet_supported(&pos));
            out_a::Expr::Error
        }
        in_a::ExprData::While(pred, block) => {
            if exp_mut {
                ctx.report(error::expected_mutable(pos));
            }
            let pred = check_expr(ctx, sym_table, env, *pred, &Type::builtin("bool"), false)?;
            let block = check_expr(ctx, sym_table, env, *block, &Type::unit(), false)?;
            if !unify(exp_tp, &Type::unit()) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), Type::unit()));
            }
            out_a::Expr::While {
                pred: Box::new(pred),
                block: Box::new(block),
            }
        }
        in_a::ExprData::Cast(expr, to_type) => {
            let tp = env.fresh_uvar(&pos);
            let expr = check_expr(ctx, sym_table, env, *expr, &tp, exp_mut)?;
            if !unify(exp_tp, &to_type) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), to_type));
            }
            expr
        }
        in_a::ExprData::ArrayInitExact(exprs) => {
            if exp_mut {
                ctx.report(error::expected_mutable(pos));
            }
            let size = exprs.len();
            let tp = env.fresh_uvar(&pos);
            let exprs = exprs
                .into_iter()
                .map(|expr| check_expr(ctx, sym_table, env, expr, &tp, false))
                .collect::<Result<_, _>>()?;
            let arr_tp = Type::array(size, tp.clone());
            if !unify(exp_tp, &arr_tp) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), arr_tp));
            }
            out_a::Expr::ArrayInitExact(exprs, tp)
        }
        in_a::ExprData::ArrayInitRepeat(expr, size) => {
            if exp_mut {
                ctx.report(error::expected_mutable(pos));
            }
            let tp = env.fresh_uvar(&pos);
            let expr = check_expr(ctx, sym_table, env, *expr, &tp, false)?;
            let arr_tp = Type::array(size, tp.clone());
            if !unify(exp_tp, &arr_tp) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), arr_tp));
            }
            out_a::Expr::ArrayInitRepeat(Box::new(expr), size, tp)
        }
        in_a::ExprData::Char(c) => {
            let tp = Type::builtin("u8");
            if !unify(exp_tp, &tp) {
                ctx.report(error::type_mismatch(pos, exp_tp.clone(), tp));
            }
            out_a::Expr::Char(c)
        }
    })
}
