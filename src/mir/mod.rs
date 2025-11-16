pub mod ast;
mod env;
use std::collections::HashMap;

use crate::{
    error::InternalError,
    mir::{
        ast::{TypeLayout, VarID, VarRef},
        env::Env,
    },
    symtable::SymTable,
    tp::Type,
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
                    let layout = get_layout(&st, tp.clone());
                    let tp = match layout.layout {
                        TypeLayout::Simple { tp } => tp,
                        TypeLayout::Array { .. } | TypeLayout::Tuple { .. } => out_a::Type::Tusize,
                    };
                    args.push(tp)
                }
                {
                    let layout = get_layout(&st, ret.clone());
                    match layout.layout {
                        TypeLayout::Simple { tp } => returns.push(tp),
                        TypeLayout::Array { .. } | TypeLayout::Tuple { .. } => {
                            args.push(out_a::Type::Tusize)
                        }
                    };
                }
                if let Some(n) = &info.builtin_name {
                    out_a::SymKind::BuiltinFunc {
                        args,
                        returns,
                        item_name: n.clone(),
                    }
                } else {
                    out_a::SymKind::Func { args, returns }
                }
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
        let layout = get_layout(st, tp);
        let var_id = env.add_var(name);
        let tp = match layout.layout {
            TypeLayout::Simple { tp } => tp,
            TypeLayout::Array { .. } | TypeLayout::Tuple { .. } => {
                var_needs_stack.insert(var_id, true);
                out_a::Type::Tusize
            }
        };
        args.push((var_id, is_mut, tp))
    }
    {
        let layout = get_layout(st, f.ret_type);
        match layout.layout {
            TypeLayout::Simple { tp } => returns.push(tp),
            TypeLayout::Array { .. } | TypeLayout::Tuple { .. } => {
                let name = "__ret_var".into();
                let id = env.add_var(name);
                var_needs_stack.insert(id, true);
                args.push((id, false, out_a::Type::Tusize))
            }
        };
    }

    let body = tr_expr(&mut env, &mut var_needs_stack, st, f.body)?;

    let body = deblock(body, None);

    let func = out_a::Func {
        id: f.id,
        args,
        returns,
        body,
        var_needs_stack,
    };
    Ok(func)
}

fn deblock(e: out_a::Expr, acc: Option<out_a::Expr>) -> out_a::Expr {
    match e {
        ast::Expr::Block {
            exprs,
            last_expr,
            block_tp,
        } => {
            let last_expr = deblock(*last_expr, acc);
            exprs
                .into_iter()
                .rfold(last_expr, |acc, e| deblock(e, Some(acc)))
        }
        ast::Expr::Tuple { fields, layout } => ast::Expr::Tuple { fields, layout },
        ast::Expr::FunCall {
            expr,
            args,
            args_tp,
            ret_tp,
        } => todo!(),
        ast::Expr::FieldAccess {
            object,
            field_id,
            struct_layout,
            element_layout,
        } => {
            let e1 = ast::Expr::FieldAccess {
                object,
                field_id,
                struct_layout,
                element_layout,
            };
            if let Some(e2) = acc {
                out_a::Expr::Ignore {
                    e1: Box::new(e1),
                    e2: Box::new(e2),
                }
            } else {
                e1
            }
        }
        ast::Expr::Return { expr, ret_tp } => ast::Expr::Return { expr, ret_tp },
        ast::Expr::Let {
            id,
            layout,
            is_mut,
            expr,
        } => ast::Expr::LetIn {
            id,
            layout,
            is_mut,
            expr,
            e2: Box::new(acc.unwrap()),
        },
        ast::Expr::LetIn {
            id,
            layout,
            is_mut,
            expr,
            e2,
        } => todo!(),
        ast::Expr::Assign {
            lval,
            rval,
            assign_tp,
        } => out_a::Expr::Assign {
            lval: Box::new(deblock(*lval, None)),
            rval: Box::new(deblock(*rval, None)),
            assign_tp,
        },
        ast::Expr::Ref { var, tp } => todo!(),
        ast::Expr::RefMut { var, tp } => todo!(),
        ast::Expr::Deref { expr, in_tp } => todo!(),
        ast::Expr::ArrayInitRepeat(expr, _, layout) => todo!(),
        ast::Expr::ArrayInitExact(exprs, layout) => todo!(),
        ast::Expr::While { pred, block } => todo!(),
        ast::Expr::IndexAccess {
            arr,
            index,
            arr_layout,
            elem_layout,
        } => todo!(),
        ast::Expr::StringLit(_, layout) => todo!(),
        ast::Expr::Char(_) => todo!(),
        ast::Expr::NumLit(n, tp) => ast::Expr::NumLit(n, tp),
        ast::Expr::Var(var_ref) => ast::Expr::Var(var_ref),
        ast::Expr::Ignore { e1, e2 } => todo!(),
    }
}

fn tr_expr(
    env: &mut Env,
    vns: &mut HashMap<VarID, bool>,
    st: &SymTable,
    e: in_a::Expr,
) -> Result<ast::Expr, InternalError> {
    Ok(match e {
        in_a::Expr::NumLit(n, tp) => {
            let tp = match get_layout(st, tp).layout {
                TypeLayout::Simple { tp } => tp,
                TypeLayout::Array { elem_layout, elems } => unreachable!(),
                TypeLayout::Tuple {
                    field_count,
                    fields,
                } => unreachable!(),
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
        in_a::Expr::Tuple(exprs, items) => {
            let mut fields = vec![];
            let tp = Type::tuple(items);
            let layout = get_layout(st, tp);
            let mut id = 0;
            for expr in exprs.into_iter() {
                let expr = tr_expr(env, vns, st, expr)?;
                let layout = match &layout.layout {
                    TypeLayout::Simple { tp } => todo!(),
                    TypeLayout::Array { elem_layout, elems } => todo!(),
                    TypeLayout::Tuple {
                        field_count,
                        fields,
                    } => fields[id].clone(),
                };
                id += 1;
                fields.push((expr, layout))
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
            let args_tp = args_tp.into_iter().map(|tp| get_layout(st, tp)).collect();
            let ret_tp = get_layout(st, ret_tp);
            out_a::Expr::FunCall {
                expr: Box::new(callee),
                args,
                args_tp,
                ret_tp,
            }
        }
        in_a::Expr::FieldAccess {
            object,
            field_name,
            struct_tp,
            field_tp,
        } => {
            let object = Box::new(tr_expr(env, vns, st, *object)?);
            let struct_layout = get_layout(st, struct_tp);
            let element_layout = get_layout(st, field_tp);
            out_a::Expr::FieldAccess {
                object,
                // TODO: calculate actual id XD
                field_id: 0,
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
            let block_tp = get_layout(st, block_tp);
            out_a::Expr::Block {
                exprs,
                last_expr,
                block_tp,
            }
        }
        in_a::Expr::Return { expr, ret_tp } => {
            let layout = get_layout(st, ret_tp);
            let expr = tr_expr(env, vns, st, *expr)?;
            match &layout.layout {
                TypeLayout::Simple { tp } => out_a::Expr::Return {
                    expr: Box::new(expr),
                    ret_tp: tp.clone(),
                },
                TypeLayout::Array { elem_layout, elems } => todo!(),
                TypeLayout::Tuple {
                    field_count,
                    fields,
                } => {
                    let ret_v = env.lookup("__ret_var");
                    let lval = Box::new(out_a::Expr::Var(VarRef::Local(ret_v)));
                    out_a::Expr::Assign {
                        lval,
                        rval: Box::new(expr),
                        assign_tp: (layout),
                    }
                }
            }
        }
        in_a::Expr::Let {
            name,
            tp,
            is_mut,
            expr,
        } => {
            let id = env.add_var(name);
            let layout = get_layout(st, tp);
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
            let layout = get_layout(st, tp);
            let mut id = 0;
            // TODO: care about order!
            for (_, expr) in initializers {
                let expr = tr_expr(env, vns, st, expr)?;
                let layout = match &layout.layout {
                    TypeLayout::Simple { tp } => todo!(),
                    TypeLayout::Array { elem_layout, elems } => todo!(),
                    TypeLayout::Tuple {
                        field_count,
                        fields,
                    } => fields[id].clone(),
                };
                id += 1;
                fields.push((expr, layout))
            }
            out_a::Expr::Tuple { fields, layout }
        }
        in_a::Expr::Assign {
            lval,
            rval,
            assign_tp,
        } => {
            let lval = Box::new(tr_expr(env, vns, st, *lval)?);
            let rval = Box::new(tr_expr(env, vns, st, *rval)?);
            let layout = get_layout(st, assign_tp);
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
            let layout = get_layout(st, tp);
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
    })
}

fn get_layout(st: &SymTable, tp: crate::tp::Type) -> out_a::Layout {
    match tp.view() {
        crate::tp::TypeView::Unknown => panic!(),
        crate::tp::TypeView::UVar(uvar) => panic!(),
        crate::tp::TypeView::NumericUVar(uvar) => panic!(),

        crate::tp::TypeView::TypeApp(tvar, _, _)
        | crate::tp::TypeView::Var(tvar)
        | crate::tp::TypeView::NamedVar(tvar, _) => {
            match tvar.builtin_size() {
                Some(s) => {
                    let tp = tvar.builtin_as_mir_type().unwrap();
                    out_a::Layout {
                        layout: TypeLayout::Simple { tp },
                        size: s,
                        offset: 0,
                        align: 3,
                    }
                }
                None => {
                    let info = st.find_type_info(tvar);
                    match &info.kind {
                        crate::symtable::TypeKind::Builtin(_) => todo!(),
                        crate::symtable::TypeKind::Struct {
                            params,
                            fields: field_map,
                        } => {
                            let mut id = 0;
                            let mut fields = vec![];
                            let mut offset = 0;
                            // TODO: the order might be arbitrary
                            for (_, tp) in field_map {
                                let mut layout = get_layout(st, tp.clone());
                                layout.offset = offset;
                                id += 1;
                                offset += layout.size;
                                fields.push(layout)
                            }
                            out_a::Layout {
                                layout: out_a::TypeLayout::Tuple {
                                    field_count: id,
                                    fields,
                                },
                                size: offset,
                                offset: 0,
                                align: 3,
                            }
                        }
                        crate::symtable::TypeKind::Enum {
                            params,
                            constructors,
                        } => todo!(),
                    }
                }
            }
        }

        crate::tp::TypeView::Tuple(items) => {
            let mut id = 0;
            let mut fields = vec![];
            let mut offset = 0;
            for tp in items {
                let mut layout = get_layout(st, tp.clone());
                layout.offset = offset;
                id += 1;
                offset += layout.size;
                fields.push(layout)
            }
            out_a::Layout {
                layout: out_a::TypeLayout::Tuple {
                    field_count: id,
                    fields,
                },
                size: offset,
                offset: 0,
                align: 3,
            }
        }
        crate::tp::TypeView::Array(n, tp) => {
            let layout = get_layout(st, *tp);
            out_a::Layout {
                layout: out_a::TypeLayout::Array {
                    elem_layout: Box::new(layout),
                    elems: n,
                },
                size: 0,
                offset: 0,
                align: 3,
            }
        }
        crate::tp::TypeView::Fun(_, _)
        | crate::tp::TypeView::MutPtr(_)
        | crate::tp::TypeView::Ptr(_) => out_a::Layout {
            layout: TypeLayout::Simple {
                tp: ast::Type::Tusize,
            },
            size: 8,
            offset: 0,
            align: 3,
        },
    }
}
