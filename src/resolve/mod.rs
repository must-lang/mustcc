pub mod ast;
mod env;

use std::collections::HashMap;

use crate::common::NodeID;
use crate::error::InternalError;
use crate::error::context::Context;
use crate::error::diagnostic::{Diagnostic, Label};
use crate::resolve::env::Env;
use crate::symtable::{SymInfo, SymKind, TypeInfo, TypeKind};
use crate::tp::{TVar, Type};

use crate::mod_tree::ast as in_a;
use ast as out_a;

pub fn translate(ctx: &mut Context, prog: in_a::Program) -> Result<out_a::Program, InternalError> {
    let mut tvar_map: HashMap<NodeID, TVar> = HashMap::new();
    generate_tvars(&mut tvar_map, &prog.ast);
    let mut env = Env::init(prog.scope_info, tvar_map);
    let functions = tr_module(ctx, &mut env, prog.ast)?;
    let sym_table = env.finish();
    let prog = out_a::Program {
        functions,
        sym_table,
    };
    Ok(prog)
}

fn generate_tvars(tvar_map: &mut HashMap<NodeID, TVar>, ast: &in_a::Module) {
    for item in &ast.items {
        match item {
            in_a::ModuleItem::Module(module) => generate_tvars(tvar_map, module),
            in_a::ModuleItem::Func(_) => continue,
            in_a::ModuleItem::Struct(s) => {
                let tvar = get_tvar_maybe_builtin(&s.attributes);
                tvar_map.insert(s.id, tvar);
            }
            in_a::ModuleItem::Enum(e) => {
                let tvar = get_tvar_maybe_builtin(&e.attributes);
                tvar_map.insert(e.id, tvar);
            }
        }
    }
}

fn get_tvar_maybe_builtin(attributes: &Vec<crate::common::RAttribute>) -> TVar {
    let mut builtin_name = None;

    for attribute in attributes {
        match attribute.name.data.as_str() {
            "builtin" => {
                builtin_name = Some(attribute.args[0].clone());
            }
            _ => (),
        }
    }

    let tvar = match builtin_name {
        Some(name) => TVar::of_builtin(name),
        None => TVar::new(),
    };

    tvar
}

fn tr_module(
    ctx: &mut Context,
    env: &mut Env,
    ast: in_a::Module,
) -> Result<Vec<out_a::Func>, InternalError> {
    let mut functions = vec![];
    for item in ast.items {
        env.current_module = ast.id;
        match item {
            in_a::ModuleItem::Module(module) => {
                let mut mod_functions = tr_module(ctx, env, module)?;
                functions.append(&mut mod_functions);
            }
            in_a::ModuleItem::Func(func) => {
                let func = tr_func(ctx, env, func, None)?;
                if let Some(func) = func {
                    functions.push(func);
                }
            }
            in_a::ModuleItem::Struct(s) => {
                let tvar = env.get_tvar(s.id)?;
                let mut fields = HashMap::new();
                for (name, tp) in s.fields {
                    let tp = env.resolve_type(ctx, tp)?;
                    let name = name.data;
                    match fields.insert(name, tp) {
                        Some(_) => panic!("field already defined"),
                        None => (),
                    }
                }
                let methods = s
                    .methods
                    .iter()
                    .map(|func| (func.name.name_str(), func.id))
                    .collect();

                let kind = TypeKind::Struct { fields };

                let type_info = TypeInfo {
                    name: s.name.name_str(),
                    pos: s.pos.clone(),
                    methods,
                    kind,
                };

                env.add_type_info(tvar, type_info);
                let sym_info = SymInfo::build(s.name.data.clone(), s.pos, SymKind::Struct(tvar))
                    .with_attributes(s.attributes);
                env.add_sym_info(s.id, sym_info);
                for method in s.methods {
                    let func = tr_func(ctx, env, method, Some((tvar, s.name.data.clone())))?;
                    if let Some(func) = func {
                        functions.push(func);
                    }
                }
            }
            in_a::ModuleItem::Enum(e) => {
                let tvar = env.get_tvar(e.id)?;

                let mut constructors = vec![];
                for cons in e.constructors {
                    match cons {
                        in_a::Constructor::Tuple {
                            attributes,
                            id,
                            name,
                            pos,
                            params,
                        } => {
                            let args = params
                                .into_iter()
                                .map(|param| env.resolve_type(ctx, param))
                                .collect::<Result<_, _>>()?;
                            constructors.push(id);
                            let sym_info = SymInfo::build(
                                name.name_str(),
                                pos,
                                SymKind::EnumCons { args, parent: e.id },
                            )
                            .with_attributes(attributes);
                            env.add_sym_info(id, sym_info);
                            id
                        }
                        in_a::Constructor::Struct {
                            attributes,
                            id,
                            name,
                            pos,
                            params,
                        } => todo!(),
                    };
                }
                let methods = e
                    .methods
                    .iter()
                    .map(|func| (func.name.name_str(), func.id))
                    .collect();

                let kind = TypeKind::Enum { constructors };

                let type_info = TypeInfo {
                    name: e.name.name_str(),
                    pos: e.pos.clone(),
                    kind,
                    methods,
                };

                env.add_type_info(tvar, type_info);
                let sym_info = SymInfo::build(e.name.data.clone(), e.pos, SymKind::Enum(tvar))
                    .with_attributes(e.attributes);
                env.add_sym_info(e.id, sym_info);
                for method in e.methods {
                    let func = tr_func(ctx, env, method, Some((tvar, e.name.data.clone())))?;
                    if let Some(func) = func {
                        functions.push(func);
                    }
                }
            }
        }
    }
    Ok(functions)
}

fn tr_func(
    ctx: &mut Context,
    env: &mut Env,
    func: in_a::Func,
    parent: Option<(TVar, String)>,
) -> Result<Option<ast::Func>, InternalError> {
    env.new_scope();
    let ret_type = match func.ret_type {
        Some(tp) => env.resolve_type(ctx, tp)?,
        None => Type::unit(),
    };
    let mut args_tp = vec![];
    let mut args = vec![];
    for arg in func.args {
        let arg = match arg {
            in_a::FnArg::Named {
                is_mut,
                name,
                tp,
                pos,
            } => {
                env.add_local(name.name_str());
                let tp = env.resolve_type(ctx, tp)?;
                args_tp.push(tp.clone());
                out_a::FnArg {
                    is_mut,
                    name: name.name_str(),
                    tp,
                    pos,
                }
            }
            in_a::FnArg::NSelf { is_mut, pos } => {
                let name = "self".to_string();
                env.add_local(name.clone());
                let tp = match &parent {
                    Some(p) => Type::named_var(p.0, p.1.clone()),
                    None => {
                        ctx.report(Diagnostic::error(&pos));
                        return Ok(None);
                    }
                };
                args_tp.push(tp.clone());
                out_a::FnArg {
                    is_mut,
                    name,
                    tp,
                    pos,
                }
            }
            in_a::FnArg::PtrSelf(pos) => {
                let name = "self".to_string();
                env.add_local(name.clone());
                let tp = match &parent {
                    Some(p) => Type::ptr(Type::named_var(p.0, p.1.clone())),
                    None => {
                        ctx.report(Diagnostic::error(&pos));
                        return Ok(None);
                    }
                };
                args_tp.push(tp.clone());
                out_a::FnArg {
                    is_mut: false,
                    name,
                    tp,
                    pos,
                }
            }
            in_a::FnArg::MutPtrSelf(pos) => {
                let name = "self".to_string();
                env.add_local(name.clone());
                let tp = match &parent {
                    Some(p) => Type::mut_ptr(Type::named_var(p.0, p.1.clone())),
                    None => {
                        ctx.report(Diagnostic::error(&pos));
                        return Ok(None);
                    }
                };
                args_tp.push(tp.clone());
                out_a::FnArg {
                    is_mut: false,
                    name,
                    tp,
                    pos,
                }
            }
        };
        args.push(arg);
    }

    let sym_kind = SymKind::Func {
        args: args_tp,
        ret: ret_type.clone(),
    };

    let sym_info = SymInfo::build(func.name.name_str(), func.pos.clone(), sym_kind)
        .with_attributes(func.attributes);

    let is_extern = sym_info.is_extern;

    env.add_sym_info(func.id, sym_info);

    let body = match func.body {
        Some(body) => tr_expr(ctx, env, body)?,
        None => {
            if !is_extern {
                ctx.report(Diagnostic::error(&func.pos));
            }
            env.leave_scope();
            return Ok(None);
        }
    };

    let func = out_a::Func {
        id: func.id,
        name: func.name.name_str(),
        args,
        ret_type,
        body,
        pos: func.pos,
    };
    env.leave_scope();
    Ok(Some(func))
}

fn tr_expr(
    ctx: &mut Context,
    env: &mut Env,
    expr: in_a::ExprNode,
) -> Result<out_a::ExprNode, InternalError> {
    let pos = expr.pos;
    let err_node = Ok(out_a::ExprNode {
        data: out_a::ExprData::Error,
        pos: pos.clone(),
    });
    let data = match expr.data {
        in_a::ExprData::Var(path) => match env.find_symbol(path) {
            Ok(sym) => out_a::ExprData::Var(sym),
            Err(diag) => {
                ctx.report(diag);
                out_a::ExprData::Error
            }
        },
        in_a::ExprData::FieldAccess(expr_node, ident) => {
            let expr_node = tr_expr(ctx, env, *expr_node)?;
            out_a::ExprData::FieldAccess(Box::new(expr_node), ident.name_str())
        }
        in_a::ExprData::ClosedBlock(expr_nodes) => {
            env.new_scope();
            let expr_nodes = expr_nodes
                .into_iter()
                .map(|expr| tr_expr(ctx, env, expr))
                .collect::<Result<_, _>>()?;
            env.leave_scope();
            let last = out_a::ExprNode {
                data: out_a::ExprData::Tuple(vec![]),
                pos: pos.clone(),
            };
            out_a::ExprData::Block(expr_nodes, Box::new(last))
        }
        in_a::ExprData::OpenBlock(expr_nodes, expr_node) => {
            env.new_scope();
            let expr_nodes = expr_nodes
                .into_iter()
                .map(|expr| tr_expr(ctx, env, expr))
                .collect::<Result<_, _>>()?;
            let expr_node = tr_expr(ctx, env, *expr_node)?;
            env.leave_scope();
            out_a::ExprData::Block(expr_nodes, Box::new(expr_node))
        }
        in_a::ExprData::Return(expr_node) => {
            let expr_node = match expr_node {
                Some(expr_node) => tr_expr(ctx, env, *expr_node)?,
                None => out_a::ExprNode {
                    data: out_a::ExprData::Tuple(vec![]),
                    pos: pos.clone(),
                },
            };
            out_a::ExprData::Return(Box::new(expr_node))
        }
        in_a::ExprData::Let {
            name,
            is_mut,
            tp,
            expr,
        } => {
            env.add_local(name.name_str());
            let tp = match tp {
                Some(tp) => Some(env.resolve_type(ctx, tp)?),
                None => None,
            };
            let expr = Box::new(tr_expr(ctx, env, *expr)?);
            out_a::ExprData::Let {
                name: name.name_str(),
                is_mut,
                tp,
                expr,
            }
        }
        in_a::ExprData::If(pr, th, el) => {
            let pr = tr_expr(ctx, env, *pr)?;
            let th = tr_expr(ctx, env, *th)?;
            let el = match el {
                Some(expr_node) => tr_expr(ctx, env, *expr_node)?,
                None => out_a::ExprNode {
                    data: out_a::ExprData::Tuple(vec![]),
                    pos: pos.clone(),
                },
            };
            out_a::ExprData::If(Box::new(pr), Box::new(th), Box::new(el))
        }
        in_a::ExprData::StructCons(path, items) => {
            let sym_ref = match env.find_symbol(path) {
                Ok(sym) => sym,
                Err(diag) => {
                    ctx.report(diag);
                    return err_node;
                }
            };
            let id = match sym_ref {
                out_a::SymRef::Local(_) => panic!("local type definitons not supported"),
                out_a::SymRef::Global(id) => id,
            };
            let mut tr_items = HashMap::new();
            for (ident, expr) in items {
                let name = ident.name_str();
                if let Some(_) = tr_items.insert(name.clone(), tr_expr(ctx, env, expr)?) {
                    ctx.report(
                        Diagnostic::error(&pos).with_label(
                            Label::new(&pos)
                                .with_msg(format!("field `{}` initialized more than once", name)),
                        ),
                    );
                }
            }
            out_a::ExprData::StructCons(id, tr_items)
        }
        in_a::ExprData::Assign(lexpr, rexpr) => {
            let lexpr = tr_expr(ctx, env, *lexpr)?;
            let rexpr = tr_expr(ctx, env, *rexpr)?;
            out_a::ExprData::Assign(Box::new(lexpr), Box::new(rexpr))
        }
        in_a::ExprData::FunCall(expr_node, expr_nodes) => {
            let expr_node = tr_expr(ctx, env, *expr_node)?;
            let expr_nodes = expr_nodes
                .into_iter()
                .map(|expr| tr_expr(ctx, env, expr))
                .collect::<Result<_, _>>()?;
            out_a::ExprData::FunCall(Box::new(expr_node), expr_nodes)
        }
        in_a::ExprData::Ref(expr_node) => {
            let expr_node = tr_expr(ctx, env, *expr_node)?;
            out_a::ExprData::Ref(Box::new(expr_node))
        }
        in_a::ExprData::RefMut(expr_node) => {
            let expr_node = tr_expr(ctx, env, *expr_node)?;
            out_a::ExprData::RefMut(Box::new(expr_node))
        }
        in_a::ExprData::Deref(expr_node) => {
            let expr_node = tr_expr(ctx, env, *expr_node)?;
            out_a::ExprData::Deref(Box::new(expr_node))
        }
        in_a::ExprData::Number(num) => out_a::ExprData::NumLit(num),
        in_a::ExprData::Error => out_a::ExprData::Error,
        in_a::ExprData::Char(c) => out_a::ExprData::Char(c),
        in_a::ExprData::String(s) => out_a::ExprData::String(s),
        in_a::ExprData::Tuple(expr_nodes) => {
            let expr_nodes = expr_nodes
                .into_iter()
                .map(|expr| tr_expr(ctx, env, expr))
                .collect::<Result<_, _>>()?;
            out_a::ExprData::Tuple(expr_nodes)
        }
        in_a::ExprData::Match(expr_node, match_clauses) => {
            let expr_node = tr_expr(ctx, env, *expr_node)?;
            let match_clauses = match_clauses
                .into_iter()
                .map(|cl| tr_clause(ctx, env, cl))
                .collect::<Result<_, _>>()?;
            out_a::ExprData::Match(Box::new(expr_node), match_clauses)
        }
        in_a::ExprData::While(expr, block) => {
            let expr = tr_expr(ctx, env, *expr)?;
            let block = tr_expr(ctx, env, *block)?;
            out_a::ExprData::While(Box::new(expr), Box::new(block))
        }
        in_a::ExprData::MethodCall(expr_node, ident, expr_nodes) => {
            let expr_node = tr_expr(ctx, env, *expr_node)?;
            let expr_nodes = expr_nodes
                .into_iter()
                .map(|expr| tr_expr(ctx, env, expr))
                .collect::<Result<_, _>>()?;
            out_a::ExprData::MethodCall(Box::new(expr_node), ident.data, expr_nodes)
        }
        in_a::ExprData::ArrayInitExact(expr_nodes) => {
            let expr_nodes = expr_nodes
                .into_iter()
                .map(|expr| tr_expr(ctx, env, expr))
                .collect::<Result<_, _>>()?;
            out_a::ExprData::ArrayInitExact(expr_nodes)
        }
        in_a::ExprData::ArrayInitRepeat(expr_node, size) => {
            let expr_node = tr_expr(ctx, env, *expr_node)?;
            out_a::ExprData::ArrayInitRepeat(Box::new(expr_node), size)
        }
        in_a::ExprData::IndexAccess(expr1, expr2) => {
            let expr1 = tr_expr(ctx, env, *expr1)?;
            let expr2 = tr_expr(ctx, env, *expr2)?;
            out_a::ExprData::IndexAccess(Box::new(expr1), Box::new(expr2))
        }
        in_a::ExprData::Cast(expr_node, rtype_node) => {
            let expr_node = tr_expr(ctx, env, *expr_node)?;
            let tp = env.resolve_type(ctx, rtype_node)?;
            out_a::ExprData::Cast(Box::new(expr_node), tp)
        }
    };
    let expr = out_a::ExprNode { data, pos };
    Ok(expr)
}

fn tr_clause(
    ctx: &mut Context,
    env: &mut Env,
    cl: in_a::MatchClause,
) -> Result<out_a::MatchClause, InternalError> {
    env.new_scope();

    let pattern = tr_pattern(ctx, env, cl.pattern)?;

    let expr = tr_expr(ctx, env, cl.expr)?;

    let cl = out_a::MatchClause {
        pattern,
        expr,
        pos: cl.pos,
    };
    env.leave_scope();
    Ok(cl)
}

fn tr_pattern(
    ctx: &mut Context,
    env: &mut Env,
    pattern: in_a::PatternNode,
) -> Result<out_a::PatternNode, InternalError> {
    let pos = pattern.pos;
    let data = match pattern.data {
        in_a::PatternData::Wildcard => out_a::PatternData::Wildcard,
        in_a::PatternData::Number(n) => out_a::PatternData::Number(n),
        in_a::PatternData::Var(ident) => {
            let name = ident.data;
            env.add_local(name.clone());
            out_a::PatternData::Var(name)
        }
        in_a::PatternData::Tuple(pattern_nodes) => {
            let pattern_nodes = pattern_nodes
                .into_iter()
                .map(|pat| tr_pattern(ctx, env, pat))
                .collect::<Result<_, _>>()?;
            out_a::PatternData::Tuple(pattern_nodes)
        }
        in_a::PatternData::TupleCons(path, pattern_nodes) => match env.find_symbol(path) {
            Ok(sym) => match sym {
                ast::SymRef::Local(_) => {
                    ctx.report(Diagnostic::error(&pos));
                    out_a::PatternData::Error
                }
                ast::SymRef::Global(id) => {
                    let pattern_nodes = pattern_nodes
                        .into_iter()
                        .map(|pat| tr_pattern(ctx, env, pat))
                        .collect::<Result<_, _>>()?;
                    out_a::PatternData::TupleCons(id, pattern_nodes)
                }
            },
            Err(diag) => {
                ctx.report(diag);
                out_a::PatternData::Error
            }
        },
    };
    let node = out_a::PatternNode { data, pos };
    Ok(node)
}
