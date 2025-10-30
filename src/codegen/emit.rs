use std::{
    fmt::Display,
    io::{self, Write},
};

use crate::{
    codegen::ast::VarID,
    common::NodeID,
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
            TypeKind::Builtin(str) => match str.as_str() {
                "never" => write!(w, "typedef struct {{}} {};\n", tv)?,
                "bool" => write!(w, "typedef unsigned char {};\n", tv)?,
                "order" => write!(w, "typedef unsigned char {};\n", tv)?,
                "u8" => write!(w, "typedef unsigned char {};\n", tv)?,
                "usize" => write!(w, "typedef unsigned long int {};\n", tv)?,
                "i32" => write!(w, "typedef int {};\n", tv)?,
                _ => panic!("{}", str),
            },
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
            TypeKind::Builtin(str) => (),
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
                                let name = format!("__{}", id);
                                write!(w, "{};\n", tp.with_name(&name))?
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

    // generate functions declarations
    for f in &prog.functions {
        let info = st.find_sym_info(f.id);
        let name = if info.mangle {
            format!("{}", f.id)
        } else {
            f.name.clone()
        };
        let args_str = f
            .args
            .iter()
            .map(|(_, a)| a.with_name(""))
            .collect::<Vec<_>>()
            .join(", ");
        write!(w, "{} {}({});\n", f.ret_type.with_name(""), name, args_str)?;
    }

    // generate functions implementations
    for func in prog.functions {
        let args_str = func
            .args
            .iter()
            .map(|a| a.1.with_name(&a.0))
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            w,
            "{} {}({}) {{\n",
            func.ret_type.with_name(""),
            func.name,
            args_str
        )?;
        for stmt in func.body {
            write!(w, "    {};\n", stmt)?
        }
        write!(w, "}}\n")?
    }
    Ok(())
}

impl Display for TVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tv_{}", self.id())
    }
}

impl Type {
    pub fn with_name<T: Display>(&self, name: T) -> String {
        match self.view() {
            TypeView::Unknown | TypeView::UVar(_) | TypeView::NumericUVar(_) => {
                panic!("invalid type")
            }
            TypeView::NamedVar(tvar, _) | TypeView::Var(tvar) => format!("{} {}", tvar, name),
            TypeView::Tuple(items) => {
                println!("{}", self);
                let mut buf = String::new();
                buf += "struct {";
                for (id, tp) in items.iter().enumerate() {
                    println!("{}", id);
                    buf += &tp.with_name(&format!("__{}", id));
                    buf += ";";
                }
                buf += "}";
                println!("{}", buf);
                buf
            }
            TypeView::Array(size, tp) => {
                format!("{}[{}]", tp.with_name(name), size)
            }
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

impl Display for out_a::Stmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            out_a::Stmt::Return { expr, ret_tp } => write!(f, "return {}", expr)?,
            out_a::Stmt::VarDecl { id, tp } => write!(f, "{}", tp.with_name(id))?,
            out_a::Stmt::If {
                pred,
                th,
                el,
                block_tp,
            } => {
                write!(f, "if ({}) {{\n", pred)?;
                for stmt in th {
                    write!(f, "    {};\n", stmt)?
                }
                write!(f, "}} else {{\n")?;
                for stmt in el {
                    write!(f, "    {};\n", stmt)?
                }
                write!(f, "}}\n")?;
            }
            out_a::Stmt::Assign { lval, rval } => write!(f, "{} = {}", lval, rval)?,
            out_a::Stmt::While { cond, body } => {
                write!(f, "while ({}) {{\n", cond)?;
                for stmt in body {
                    write!(f, "    {};\n", stmt)?
                }
                write!(f, "}};\n")?;
            }
        }
        Ok(())
    }
}

impl Display for out_a::LValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ast::LValue::VarRef(var_ref) => write!(f, "{}", var_ref),
            ast::LValue::FieldAccess {
                var,
                field_id,
                field_tp,
            } => todo!(),
            ast::LValue::Deref { var, in_tp } => todo!(),
        }
    }
}

impl Display for out_a::RValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ast::RValue::NumLit(n, _) => write!(f, "{}", n),
            ast::RValue::FunCall {
                callee,
                args,
                ret_tp,
            } => todo!(),
            ast::RValue::Ref { var, tp } => todo!(),
            ast::RValue::StructCons {
                id,
                initializers,
                tp,
            } => todo!(),
            ast::RValue::Value(lvalue) => write!(f, "{}", lvalue),
            ast::RValue::ArrayInit(rvalues) => {
                write!(f, "[]")
            }
            ast::RValue::Tuple(vals) => {
                write!(f, "{{")?;
                for (id, v) in vals.iter().enumerate() {
                    write!(f, ".__{} = {}", id, v)?
                }
                write!(f, "}}")
            }
        }
    }
}

impl Display for out_a::VarRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ast::VarRef::LocalVar { id } => write!(f, "{}", id),
            ast::VarRef::GlobalVar { id } => write!(f, "{}", id),
        }
    }
}

impl Display for NodeID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "id_{}", self.get())
    }
}
