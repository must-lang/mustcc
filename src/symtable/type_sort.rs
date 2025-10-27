use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    common::NodeID,
    symtable::{SymInfo, SymKind, TypeInfo, TypeKind},
    tp::{TVar, Type},
};

pub fn calculate_size(
    tvar_map: &HashMap<TVar, TypeInfo>,
    node_map: &HashMap<NodeID, SymInfo>,
    tvar_order: &[TVar],
) -> HashMap<TVar, usize> {
    let mut tvar_size = HashMap::new();
    for tvar in tvar_order {
        // if tvar.is_builtin() {
        //     let size = tvar.builtin_size().unwrap();
        //     tvar_size.insert(*tvar, size);
        // } else {
        //     // calculate_type_size()
        // }
    }
    tvar_size
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

pub fn topo_sort(dep_tree: HashMap<TVar, HashSet<TVar>>) -> Vec<TVar> {
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

    if order.len() != n {
        let left: Vec<_> = indeg
            .into_iter()
            .filter_map(|(k, v)| if v > 0 { Some(k) } else { None })
            .collect();
        println!("LEFT (CYCLIC) TVARS: {:#?}", left);
    }

    order
}

pub fn make_dep_tree(
    tvar_map: &HashMap<TVar, TypeInfo>,
    node_map: &HashMap<NodeID, SymInfo>,
) -> HashMap<TVar, HashSet<TVar>> {
    let mut dep_tree = HashMap::new();
    for (tv, info) in tvar_map {
        let tvars = get_tvars(info, node_map);
        if let Some(_) = dep_tree.insert(*tv, tvars) {
            panic!()
        }
    }
    dep_tree
}

fn get_tvars(info: &TypeInfo, node_map: &HashMap<NodeID, SymInfo>) -> HashSet<TVar> {
    let mut set = HashSet::new();
    match &info.kind {
        TypeKind::Struct { params, fields } => {
            for (_, field) in fields {
                set.extend(get_tvars_of_type(&field))
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
                                set.extend(get_tvars_of_type(&arg))
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

fn get_tvars_of_type(tp: &Type) -> HashSet<TVar> {
    match &tp.view() {
        crate::tp::TypeView::Unknown => HashSet::new(),
        crate::tp::TypeView::UVar(uvar) | crate::tp::TypeView::NumericUVar(uvar) => panic!(),
        crate::tp::TypeView::Var(tvar) | crate::tp::TypeView::NamedVar(tvar, _) => {
            let mut set = HashSet::new();
            set.insert(*tvar);
            set
        }
        crate::tp::TypeView::Tuple(items) => {
            let mut set = HashSet::new();
            for tp in items {
                set.extend(get_tvars_of_type(tp));
            }
            set
        }
        crate::tp::TypeView::Array(_, tp) => get_tvars_of_type(tp),
        crate::tp::TypeView::Fun(items, ret) => {
            let mut set = HashSet::new();
            for tp in items {
                set.extend(get_tvars_of_type(tp));
            }
            set.extend(get_tvars_of_type(&ret));
            set
        }
        crate::tp::TypeView::Ptr(_) => HashSet::new(),
        crate::tp::TypeView::MutPtr(_) => HashSet::new(),
        crate::tp::TypeView::TypeApp(tvar, _, items) => todo!(),
    }
}
