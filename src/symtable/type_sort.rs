use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    common::{NodeID, Position},
    error::context::Context,
    symtable::{SymInfo, SymKind, TypeInfo, TypeKind, error},
    tp::{TVar, Type, TypeView},
};

pub fn calculate_size(
    ctx: &mut Context,
    tvar_map: &HashMap<TVar, TypeInfo>,
    node_map: &HashMap<NodeID, SymInfo>,
    tvar_order: &Vec<TVar>,
) -> HashMap<TVar, usize> {
    let mut tvar_size = HashMap::new();
    for tvar in tvar_order {
        let size = if tvar.is_builtin() {
            tvar.builtin_size().unwrap()
        } else {
            let info = tvar_map.get(tvar).unwrap();
            match &info.kind {
                TypeKind::LocalVar => todo!(),
                TypeKind::Primitive { size } => todo!(),
                TypeKind::Struct { params, fields } => {
                    let mut size = 0;
                    for (_, f) in fields {
                        size += calculate_type_size(ctx, &info.pos, &tvar_size, f);
                    }
                    size
                }
                TypeKind::Enum {
                    params,
                    constructors,
                } => todo!(),
            }
        };
        tvar_size.insert(*tvar, size);
    }
    tvar_size
}

fn calculate_type_size(
    ctx: &mut Context,
    pos: &Position,
    tvar_size: &HashMap<TVar, usize>,
    tp: &Type,
) -> usize {
    match tp.view() {
        TypeView::Unknown => 0,
        TypeView::UVar(uvar) => todo!(),
        TypeView::NumericUVar(uvar) => todo!(),
        TypeView::TypeApp(tvar, _, _) | TypeView::Var(tvar) | TypeView::NamedVar(tvar, _) => {
            match tvar_size.get(&tvar) {
                Some(s) => *s,
                None => {
                    ctx.report(error::unsized_type(pos));
                    0
                }
            }
        }
        TypeView::Tuple(items) => items
            .iter()
            .map(|tp| calculate_type_size(ctx, pos, tvar_size, tp))
            .sum(),
        TypeView::Array(size, tp) => calculate_type_size(ctx, pos, tvar_size, &tp) * size,
        TypeView::Fun(_, _) | TypeView::Ptr(_) | TypeView::MutPtr(_) => 8,
    }
}

pub fn reverse_graph(graph: &HashMap<TVar, HashSet<TVar>>) -> HashMap<TVar, HashSet<TVar>> {
    let mut rev: HashMap<TVar, HashSet<TVar>> = HashMap::new();

    for node in graph.keys() {
        rev.entry(*node).or_default();
    }

    for (from, tos) in graph {
        for to in tos {
            rev.entry(*to).or_default().insert(*from);
        }
    }

    rev
}

pub fn topo_sort(dep_tree: HashMap<TVar, HashSet<TVar>>) -> (Vec<TVar>, Vec<TVar>) {
    let n = dep_tree.len();
    let mut indeg = HashMap::<TVar, usize>::new();

    for (tvar, set) in reverse_graph(&dep_tree) {
        indeg.insert(tvar, set.len());
    }

    let mut q = VecDeque::new();
    for i in dep_tree.keys() {
        if indeg[i] == 0 {
            q.push_back(i);
        }
    }

    let mut order = Vec::with_capacity(n);

    while let Some(node) = q.pop_front() {
        order.push(*node);
        for dependee in dep_tree.get(node).unwrap() {
            let indeg = indeg.get_mut(dependee).unwrap();
            *indeg -= 1;
            if *indeg == 0 {
                q.push_back(dependee);
            }
        }
    }

    order.reverse();

    if order.len() != n {
        let left: Vec<_> = indeg
            .into_iter()
            .filter_map(|(k, v)| if v > 0 { Some(k) } else { None })
            .collect();
        return (order, left);
    }

    (order, vec![])
}

pub fn make_dep_tree(
    tvar_map: &HashMap<TVar, TypeInfo>,
    node_map: &HashMap<NodeID, SymInfo>,
) -> HashMap<TVar, HashSet<TVar>> {
    let mut dep_tree = HashMap::new();
    for (tv, info) in tvar_map {
        let tvars = get_tvars(info, node_map);
        if let Some(_) = dep_tree.insert(*tv, tvars) {
            unreachable!("all tvars are unique")
        }
    }
    dep_tree
}

fn get_tvars(info: &TypeInfo, node_map: &HashMap<NodeID, SymInfo>) -> HashSet<TVar> {
    let mut set = HashSet::new();
    match &info.kind {
        TypeKind::Struct { params, fields } => {
            for (_, field) in fields {
                set.extend(field.get_size_dependencies())
            }
        }
        TypeKind::Enum {
            params,
            constructors,
        } => {
            for (_, cons) in constructors {
                match node_map.get(&cons) {
                    Some(info) => match &info.kind {
                        SymKind::EnumCons { id, args, parent } => {
                            for arg in args {
                                set.extend(arg.get_size_dependencies())
                            }
                        }
                        _ => panic!(),
                    },
                    None => panic!(),
                }
            }
        }
        _ => (),
    };
    set
}
