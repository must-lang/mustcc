pub mod ast;
mod env;
mod error;
mod import_solve;
pub mod scope;
pub mod scope_info;

use std::collections::BTreeMap;

pub use scope_info::ScopeInfo;

use crate::common::{NodeID, Path, Visibility};
use crate::error::InternalError;
use crate::error::context::Context;
use crate::error::diagnostic::Diagnostic;

use crate::mod_tree::env::Env;
use crate::mod_tree::scope::{Binding, Import, Scope, ScopeKind};
use crate::parser::ast as in_a;
use ast as out_a;

/// Translate the file tree into a single module, resolving imports.
pub fn translate(ctx: &mut Context, prog: in_a::Program) -> Result<out_a::Program, InternalError> {
    let mut env = Env::init(prog.file_map);

    let module = match env.remove_module(&vec!["src".into()]) {
        Some(m) => m,
        None => {
            return Err(InternalError::AnyMsg("failed to load root module".into()));
        }
    };

    let ast = tr_module(ctx, &mut env, module)?;

    let scope_info = env.solve_imports(ctx)?;

    let prog = out_a::Program { scope_info, ast };

    Ok(prog)
}

fn tr_module(
    ctx: &mut Context,
    env: &mut Env,
    module: in_a::Module,
) -> Result<ast::Module, InternalError> {
    let id = NodeID::new_global();

    let binding = Binding {
        vis: module.visibility,
        kind: scope::Kind::Module,
        sym: scope::Symbol::Local(id),
    };

    let mut mod_path = env.get_current_name_path().clone();

    mod_path.push(module.name.name_str());

    let mod_info = Scope {
        items: BTreeMap::new(),
        kind: ScopeKind::Module {
            imports: vec![],
            parent: env.get_current_module_id(),
        },
    };

    // add this module to its parent
    if let Err(diag) = env.add_item(module.name.clone(), binding) {
        ctx.report(diag);
        return Ok(out_a::Module::empty());
    }

    // register this new module in mod tree
    env.add_mod_info(id, mod_info);

    // enter the scope of new module
    env.enter(module.name.name_str());

    let mut items = vec![];

    for item in module.items {
        let item = match item {
            in_a::ModuleItem::Module(m) => out_a::ModuleItem::Module(tr_module(ctx, env, m)?),
            in_a::ModuleItem::ModuleDecl(m) => {
                let mut path = env.get_current_name_path().clone();
                path.push(m.name.name_str());
                let module = match env.remove_module(&path) {
                    Some(m) => m,
                    None => {
                        ctx.report(error::missing_module(&m.pos, m.name.name_str()));
                        continue;
                    }
                };
                out_a::ModuleItem::Module(tr_module(ctx, env, module)?)
            }
            in_a::ModuleItem::Import(it) => {
                let imports = generate_imports(it);
                for import in imports {
                    env.add_import(import)
                }
                continue;
            }
            in_a::ModuleItem::Func(it) => match tr_func(env, it) {
                Ok(f) => out_a::ModuleItem::Func(f),
                Err(diag) => {
                    ctx.report(diag);
                    continue;
                }
            },
            in_a::ModuleItem::Struct(it) => {
                if let Some(it) = tr_struct(ctx, env, it)? {
                    out_a::ModuleItem::Struct(it)
                } else {
                    continue;
                }
            }
            in_a::ModuleItem::Enum(it) => {
                if let Some(it) = tr_enum(ctx, env, it)? {
                    out_a::ModuleItem::Enum(it)
                } else {
                    continue;
                }
            }
            in_a::ModuleItem::Error => continue,
        };
        items.push(item)
    }

    let module = ast::Module {
        attributes: module.attributes,
        visibility: module.visibility,
        id,
        name: module.name,
        items,
        pos: module.pos,
    };

    // remember to leave to the parent
    env.leave();

    Ok(module)
}

fn generate_imports(it: in_a::Import) -> Vec<Import> {
    let mut imports = vec![];
    let vis = it.visibility;
    let mut path = Path {
        data: vec![].into(),
    };
    tr_import_path(it.path, &mut imports, &mut path, vis);
    imports
}

fn tr_import_path(
    import_path: in_a::ImportPathNode,
    imports: &mut Vec<Import>,
    path: &mut Path,
    vis: Visibility,
) {
    match import_path.data {
        in_a::ImportPathData::Exact(ident, alias) => {
            path.push_inplace(ident);
            let import = Import {
                path: path.clone(),
                alias,
                is_glob: false,
                vis,
            };
            imports.push(import);
            path.pop_inplace();
        }
        in_a::ImportPathData::All => {
            let import = Import {
                path: path.clone(),
                alias: None,
                is_glob: true,
                vis,
            };
            imports.push(import)
        }
        in_a::ImportPathData::Path(ident, import_path) => {
            path.push_inplace(ident);
            tr_import_path(*import_path, imports, path, vis);
            path.pop_inplace();
        }
        in_a::ImportPathData::Many(import_paths) => {
            for import_path in import_paths {
                tr_import_path(import_path, imports, path, vis.clone());
            }
        }
    };
}

fn tr_enum(
    ctx: &mut Context,
    env: &mut Env,
    it: in_a::Enum,
) -> Result<Option<out_a::Enum>, InternalError> {
    let id = NodeID::new_global();

    let vis = it.visibility;

    let binding = Binding {
        vis,
        kind: scope::Kind::Enum,
        sym: scope::Symbol::Local(id),
    };

    let mod_info = Scope {
        items: BTreeMap::new(),
        kind: ScopeKind::Enum {
            parent: env.get_current_module_id(),
        },
    };

    if let Err(diag) = env.add_item(it.name.clone(), binding) {
        ctx.report(diag);
        return Ok(None);
    }

    // register this new module in mod tree
    env.add_mod_info(id, mod_info);

    env.enter(it.name.name_str());

    let mut constructors = vec![];

    for cons in it.constructors {
        match tr_cons(env, cons, vis) {
            Ok(c) => constructors.push(c),
            Err(diag) => {
                ctx.report(diag);
                continue;
            }
        }
    }

    env.leave();

    let it = out_a::Enum {
        attributes: it.attributes,
        visibility: vis,
        id,
        name: it.name,
        type_params: it.type_params,
        constructors,
        pos: it.pos,
    };

    Ok(Some(it))
}

fn tr_cons(
    env: &mut Env,
    it: in_a::Constructor,
    vis: Visibility,
) -> Result<ast::Constructor, Diagnostic> {
    match it {
        in_a::Constructor::Tuple {
            attributes,
            name,
            pos,
            params,
        } => {
            let id = NodeID::new_global();

            let binding = Binding {
                vis,
                kind: scope::Kind::Cons,
                sym: scope::Symbol::Local(id),
            };

            // add this module to its parent
            env.add_item(name.clone(), binding)?;

            let it = out_a::Constructor::Tuple {
                attributes,
                id,
                name,
                pos,
                args: params,
            };

            Ok(it)
        }
        in_a::Constructor::Struct {
            attributes,
            name,
            pos,
            params,
        } => todo!(),
    }
}

fn tr_struct(
    ctx: &mut Context,
    env: &mut Env,
    it: in_a::Struct,
) -> Result<Option<out_a::Struct>, InternalError> {
    let id = NodeID::new_global();

    let vis = it.visibility;

    let binding = Binding {
        vis,
        kind: scope::Kind::Struct,
        sym: scope::Symbol::Local(id),
    };

    if let Err(diag) = env.add_item(it.name.clone(), binding) {
        ctx.report(diag);
        return Ok(None);
    }

    let it = out_a::Struct {
        attributes: it.attributes,
        visibility: vis,
        id,
        name: it.name,
        type_params: it.type_params,
        pos: it.pos,
        fields: it.fields,
    };

    Ok(Some(it))
}

fn tr_func(env: &mut Env, it: in_a::Func) -> Result<out_a::Func, Diagnostic> {
    let id = NodeID::new_global();

    let binding = Binding {
        vis: it.visibility,
        kind: scope::Kind::Func,
        sym: scope::Symbol::Local(id),
    };

    env.add_item(it.name.clone(), binding)?;

    let it = out_a::Func {
        attributes: it.attributes,
        visibility: it.visibility,
        id,
        name: it.name,
        type_params: it.type_params,
        args: it.args,
        ret_type: it.ret_type,
        body: it.body,
        pos: it.pos,
    };

    Ok(it)
}
