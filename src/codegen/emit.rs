use std::{
    fmt::Display,
    io::{self, Write},
};

use crate::{
    symtable::{SymKind, TypeKind},
    tp::{TVar, TypeView},
};

use super::*;

pub fn emit_code<W: Write>(prog: out_a::Program, w: &mut W) -> io::Result<()> {
    let st = prog.sym_table;
    // generate type declarations
    for tv in st.get_type_order() {
        let info = st.find_type_info(*tv);
        match &info.kind {
            TypeKind::Struct { params, fields } => write!(w, "typedef struct {} {};\n", tv, tv)?,
            TypeKind::Enum {
                params,
                constructors,
            } => write!(w, "typedef struct {} {};\n", tv, tv)?,
        }
    }
    // generate type definitions
    for tv in st.get_type_order() {
        let info = st.find_type_info(*tv);
        match &info.kind {
            TypeKind::Struct { params, fields } => {
                write!(w, "struct {} {{\n", tv)?;
                for (name, tp) in fields {
                    write!(w, "{};\n", tp.with_name(name))?
                }
                write!(w, "}};\n")?
            }
            TypeKind::Enum {
                params,
                constructors,
            } => {
                write!(w, "struct {} {{\n", tv)?;
                write!(w, "int __id;\n")?;
                write!(w, "union {{\n")?;
                for (name, node_id) in constructors {
                    let info = st.find_sym_info(*node_id);
                    match &info.kind {
                        SymKind::Func { .. } | SymKind::Struct(_) | SymKind::Enum(_) => panic!(),
                        SymKind::EnumCons { id, args, parent } => {
                            write!(w, "struct {{\n")?;
                            for (id, tp) in args.iter().enumerate() {
                                write!(w, "{};\n", tp.with_name(&id.to_string()))?
                            }
                            write!(w, "}} __{};\n", id)?;
                        }
                    }
                }
                write!(w, "}} __data;\n")?;
                write!(w, "}};\n")?
            }
        }
    }
    Ok(())
}

impl Display for TVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "__tv_{}", self.id())
    }
}

impl Type {
    pub fn with_name(&self, name: &str) -> String {
        match self.view() {
            TypeView::Unknown | TypeView::UVar(_) | TypeView::NumericUVar(_) => {
                panic!("invalid type")
            }
            TypeView::NamedVar(tvar, _) | TypeView::Var(tvar) => format!("{} {}", tvar, name),
            TypeView::Tuple(items) => todo!(),
            TypeView::Array(_, _) => todo!(),
            TypeView::Fun(args, ret) => {
                let mut buf = String::new();
                buf += &ret.with_name("");
                buf += &format!("(*{})(", name);
                let args_str = args
                    .iter()
                    .map(|a| a.with_name(""))
                    .collect::<Vec<_>>()
                    .join(", ");
                buf += &args_str;
                buf += ")";
                buf
            }
            TypeView::Ptr(tp) | TypeView::MutPtr(tp) => format!("{} *{}", tp.with_name(""), name),
            TypeView::TypeApp(tvar, _, items) => {
                todo!()
            }
        }
    }
}
