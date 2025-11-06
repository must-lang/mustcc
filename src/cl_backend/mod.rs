use std::collections::HashMap;
use std::env::var;

use crate::common::NodeID;
use crate::error::InternalError;
use crate::symtable::SymTable;
use cranelift_codegen::Context;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{
    Block, FuncRef, InstBuilder, MemFlags, SigRef, Signature, StackSlot, StackSlotData,
    StackSlotKind, Value, types::*,
};
use cranelift_codegen::settings::Configurable;
use cranelift_codegen::{ir::AbiParam, isa, settings};

use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Switch, Variable};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::object::write::elf::Sym;
use cranelift_object::{ObjectModule, ObjectProduct};

use crate::mir::ast::{self, Layout};

pub fn translate(prog: ast::Program) -> Result<ObjectProduct, InternalError> {
    let mut settings_builder = settings::builder();
    settings_builder.set("opt_level", "speed").unwrap();
    let flags = settings::Flags::new(settings_builder);
    let isa = isa::lookup_by_name("x86_64-linux-elf")
        .unwrap()
        .finish(flags)
        .unwrap();

    let module_builder = cranelift_object::ObjectBuilder::new(
        isa,
        "output",
        cranelift_module::default_libcall_names(),
    )
    .unwrap();

    let mut module = ObjectModule::new(module_builder);

    let mut l = Lowerer::new(&mut module);

    for (id, sym) in &prog.symbols {
        l.declare_sym(*id, sym);
    }

    for f in prog.functions {
        l.emit_func(f);
    }

    println!("{:#?}", module.declarations());

    let obj = module.finish();
    Ok(obj)
}

struct Lowerer<'ctx> {
    m: &'ctx mut ObjectModule,
    id_fn_map: HashMap<NodeID, FuncId>,
    variables: HashMap<ast::VarID, Var>,
    needs_stack: HashMap<ast::VarID, bool>,
}

pub enum Var {
    Reg(Variable),
    FnArg(Value),
    Stack {
        ss: StackSlot,
        layout: Layout,
        offset: usize,
    },
}

const BUILTIN_FUNCTIONS: [&'static str; 1] = ["i32_add"];

#[derive(Debug)]
pub enum LRes {
    Val(Value),
    Vals(Vec<Value>),
    Func(FuncRef),
    Builtin(String),
    StackS(StackSlot),
    Unit,
}

fn make_sig(sig: &mut Signature, args: &[ast::Type], rets: &[ast::Type]) {
    for arg in args {
        sig.params.push(AbiParam::new(arg.to_cl_type()));
    }
    for ret in rets {
        sig.returns.push(AbiParam::new(ret.to_cl_type()));
    }
}

fn vals_to_lres(vals: &[Value]) -> LRes {
    match vals.len() {
        0 => LRes::Unit,
        1 => LRes::Val(vals[0]),
        _ => LRes::Vals(vals.to_owned()),
    }
}

impl<'ctx> Lowerer<'ctx> {
    pub fn new(m: &'ctx mut ObjectModule) -> Self {
        Self {
            m,
            id_fn_map: HashMap::new(),
            variables: HashMap::new(),
            needs_stack: HashMap::new(),
        }
    }

    fn get_func_id(&self, id: NodeID) -> FuncId {
        *self.id_fn_map.get(&id).unwrap()
    }

    pub fn emit_func(&mut self, f: ast::Func) {
        let func = self.get_func_id(f.id);

        let mut ctx = self.m.make_context();
        let mut fn_ctx = FunctionBuilderContext::new();

        ctx.func.signature = self
            .m
            .declarations()
            .get_function_decl(func)
            .signature
            .clone();

        self.needs_stack = f.var_needs_stack;

        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fn_ctx);

        let block = b.create_block();
        b.append_block_params_for_function_params(block);
        b.switch_to_block(block);
        b.seal_block(block);

        let fn_args = b.block_params(block);

        for (val, (var, _, _)) in fn_args.iter().zip(f.args) {
            self.variables.insert(var, Var::FnArg(*val));
        }

        let val = self.lower_expr(&mut b, f.body);

        if let Some(v) = val {
            match v {
                LRes::Val(value) => {
                    b.ins().return_(&[value]);
                }
                LRes::Vals(values) => todo!(),
                LRes::Func(func_ref) => todo!(),
                LRes::Builtin(_) => todo!(),
                LRes::StackS(stack_slot) => todo!(),
                LRes::Unit => {
                    b.ins().return_(&[]);
                }
            };
        } else {
            // b.ins().return_(&[]);
        }

        println!("{}", b.func.display());

        b.finalize();

        match self.m.define_function(func, &mut ctx) {
            Ok(o) => (),
            Err(e) => println!("{:#?}", e),
        }

        self.variables.clear();
        self.needs_stack.clear();
        self.m.clear_context(&mut ctx);
    }

    pub fn lres_to_val(&mut self, b: &mut FunctionBuilder, lr: LRes) -> Value {
        match lr {
            LRes::Val(value) => value,
            LRes::Func(func_ref) => b.ins().func_addr(I64, func_ref),
            LRes::Builtin(name) => todo!(),
            LRes::Vals(values) => todo!(),
            LRes::Unit => todo!(),
            LRes::StackS(_) => todo!(),
        }
    }

    /// Returns None on control statements, eg return.
    #[must_use]
    pub fn lower_expr(&mut self, b: &mut FunctionBuilder, e: ast::Expr) -> Option<LRes> {
        // println!("{}", b.func.display());
        match e {
            ast::Expr::NumLit(n, tp) => Some(LRes::Val(b.ins().iconst(tp.to_cl_type(), n as i64))),
            ast::Expr::StringLit(_, _) => todo!(),
            ast::Expr::FunCall {
                expr,
                args,
                args_tp,
                ret_tp,
            } => {
                let callee = self.lower_expr(b, *expr)?;

                match callee {
                    LRes::Val(value) => {
                        // let mut args_val = vec![];
                        // for arg in args {
                        //     let arg = self.lower_expr(b, arg)?;
                        //     args_val.push(self.lres_to_val(b, arg));
                        // }
                        // let mut sig = self.m.make_signature();
                        // make_sig(&mut sig, &args_tp, &[ret_tp]);
                        // let sig = b.import_signature(sig);
                        // let inst = b.ins().call_indirect(sig, value, &args_val);
                        // let vals = b.inst_results(inst);
                        // let lres = vals_to_lres(vals);
                        // Some(lres)
                        todo!()
                    }
                    LRes::Func(func_ref) => {
                        let mut args_val = vec![];
                        for arg in args {
                            let arg = self.lower_expr(b, arg)?;
                            args_val.push(self.lres_to_val(b, arg));
                        }
                        let inst = b.ins().call(func_ref, &args_val);
                        let vals = b.inst_results(inst);
                        let lres = vals_to_lres(vals);
                        Some(lres)
                    }
                    LRes::Builtin(name) => match name.as_str() {
                        "i32_add" => {
                            let mut args_val = vec![];
                            for arg in args {
                                let arg = self.lower_expr(b, arg)?;
                                args_val.push(self.lres_to_val(b, arg));
                            }
                            let res = b.ins().iadd(args_val[0], args_val[1]);
                            Some(LRes::Val(res))
                        }
                        _ => panic!(),
                    },
                    LRes::Vals(values) => {
                        panic!()
                    }
                    LRes::Unit => panic!(),
                    LRes::StackS(stack_slot) => todo!(),
                }
            }
            ast::Expr::Block {
                exprs,
                last_expr,
                block_tp,
            } => {
                // let block = b.create_block();
                // b.switch_to_block(block);
                // b.seal_block(block);
                for e in exprs {
                    self.lower_expr(b, e)?;
                }
                self.lower_expr(b, *last_expr)
            }
            ast::Expr::Return { expr, ret_tp } => {
                match self.lower_expr(b, *expr)? {
                    LRes::Val(value) => {
                        b.ins().return_(&[value]);
                    }
                    LRes::Func(func_ref) => todo!(),
                    LRes::Vals(vals) => {
                        b.ins().return_(&vals);
                    }
                    _ => todo!(),
                };
                None
            }
            ast::Expr::Let {
                id,
                layout,
                is_mut,
                expr,
            } => {
                let var = if *self.needs_stack.get(&id).unwrap() {
                    let ss = b.create_sized_stack_slot(StackSlotData {
                        kind: StackSlotKind::ExplicitSlot,
                        size: layout.size as u32,
                        align_shift: layout.align as u8,
                    });
                    self.write_to_ss(b, *expr, ss, 0);
                    Var::Stack {
                        ss,
                        layout,
                        offset: 0,
                    }
                } else {
                    let tp = match layout.layout {
                        ast::TypeLayout::Simple { tp } => tp,
                        _ => unreachable!("other types require stack"),
                    };
                    let var = b.declare_var(tp.to_cl_type());
                    let val = self.lower_expr(b, *expr)?;
                    let val = self.lres_to_val(b, val);
                    b.def_var(var, val);
                    Var::Reg(var)
                };
                self.variables.insert(id, var);
                Some(LRes::Unit)
            }
            ast::Expr::Assign {
                lval,
                rval,
                assign_tp,
            } => {
                let lres = self.lower_expr(b, *lval)?;
                match lres {
                    LRes::Val(value) => {
                        self.write_to_addr(b, *rval, value, 0);
                    }
                    LRes::Vals(values) => todo!(),
                    LRes::Func(func_ref) => todo!(),
                    LRes::Builtin(_) => todo!(),
                    LRes::Unit => todo!(),
                    LRes::StackS(ss) => {
                        self.write_to_ss(b, *rval, ss, 0);
                    }
                }
                Some(LRes::Unit)
            }
            ast::Expr::Ref { var, tp } => match var {
                ast::VarRef::Local(var_id) => {
                    let ss = match self.variables.get(&var_id).unwrap() {
                        Var::Reg(variable) => panic!("cannot assign to virtual register"),
                        Var::Stack { ss, layout, offset } => *ss,
                        Var::FnArg(value) => todo!(),
                    };
                    let v = b.ins().stack_addr(I64, ss, 0);
                    Some(LRes::Val(v))
                }
                ast::VarRef::Global(node_id) => todo!(),
            },
            ast::Expr::RefMut { var, tp } => todo!(),
            ast::Expr::Deref { expr, in_tp } => todo!(),
            ast::Expr::Char(_) => todo!(),
            ast::Expr::ArrayInitRepeat(expr, _, _) => todo!(),
            ast::Expr::ArrayInitExact(exprs, _) => todo!(),
            ast::Expr::While { pred, block } => {
                let cond_block = b.create_block();
                let end_block = b.create_block();
                let start_block = b.create_block();

                b.ins().jump(cond_block, vec![]);
                b.switch_to_block(cond_block);
                let val = self.lower_expr(b, *pred)?;
                let val = self.lres_to_val(b, val);
                b.ins().brif(val, start_block, vec![], end_block, vec![]);

                b.switch_to_block(start_block);
                b.seal_block(start_block);
                self.lower_expr(b, *block)?;
                b.ins().jump(cond_block, vec![]);

                b.switch_to_block(end_block);
                b.seal_block(cond_block);
                b.seal_block(end_block);
                Some(LRes::Unit)
            }
            ast::Expr::IndexAccess {
                arr,
                index,
                arr_layout,
                elem_layout,
            } => todo!(),
            ast::Expr::Tuple { fields, layout } => {
                panic!("cannot produce a value from tuple, use write_to_ss instead")
            }
            ast::Expr::FieldAccess {
                object,
                field_id,
                struct_layout,
                element_layout,
            } => {
                let lres = self.lower_expr(b, *object)?;
                match lres {
                    LRes::Val(value) => todo!(),
                    LRes::Vals(values) => todo!(),
                    LRes::Func(func_ref) => todo!(),
                    LRes::Builtin(_) => todo!(),
                    LRes::StackS(stack_slot) => {
                        let layout = match struct_layout.layout {
                            ast::TypeLayout::Simple { tp } => todo!(),
                            ast::TypeLayout::Array { elem_layout, elems } => todo!(),
                            ast::TypeLayout::Tuple {
                                field_count,
                                fields,
                            } => fields[field_id].clone(),
                        };
                        match &layout.layout {
                            ast::TypeLayout::Simple { tp } => {
                                let v = b.ins().stack_load(
                                    tp.to_cl_type(),
                                    stack_slot,
                                    layout.offset as i32,
                                );
                                Some(LRes::Val(v))
                            }
                            ast::TypeLayout::Array { elem_layout, elems } => todo!(),
                            ast::TypeLayout::Tuple {
                                field_count,
                                fields,
                            } => todo!(),
                        }
                    }
                    LRes::Unit => todo!(),
                }
            }
            ast::Expr::Var(var_ref) => match var_ref {
                ast::VarRef::Local(var_id) => match self.variables.get(&var_id).unwrap() {
                    Var::Reg(var) => {
                        let val = b.use_var(*var);
                        Some(LRes::Val(val))
                    }
                    Var::Stack { ss, layout, offset } => Some(LRes::StackS(*ss)),
                    Var::FnArg(value) => Some(LRes::Val(*value)),
                },
                ast::VarRef::Global(node_id) => {
                    let f = *self.id_fn_map.get(&node_id).unwrap();
                    let f = self.m.declare_func_in_func(f, b.func);
                    Some(LRes::Func(f))
                }
            },
        }
    }

    fn write_to_ss(
        &mut self,
        b: &mut FunctionBuilder,
        e: ast::Expr,
        ss: StackSlot,
        offset: usize,
    ) -> Option<()> {
        match e {
            ast::Expr::NumLit(n, tp) => {
                let v = b.ins().iconst(tp.to_cl_type(), n as i64);
                b.ins().stack_store(v, ss, offset as i32);
                Some(())
            }
            ast::Expr::StringLit(_, layout) => todo!(),
            ast::Expr::Tuple { fields, layout } => {
                for (expr, f_layout) in fields {
                    self.write_to_ss(b, expr, ss, offset + f_layout.offset);
                }
                Some(())
            }
            ast::Expr::FunCall {
                expr,
                args,
                args_tp,
                ret_tp,
            } => {
                let ret_addr = b.ins().stack_addr(I64, ss, offset as i32);
                let mut fn_args = vec![ret_addr];
                for arg in args {
                    let v = self.lower_expr(b, arg)?;
                    match v {
                        LRes::Val(value) => fn_args.push(value),
                        LRes::Vals(values) => todo!(),
                        LRes::Func(func_ref) => todo!(),
                        LRes::Builtin(_) => todo!(),
                        LRes::StackS(stack_slot) => {
                            let addr = b.ins().stack_addr(I64, stack_slot, 0);
                            fn_args.push(addr)
                        }
                        LRes::Unit => todo!(),
                    }
                }
                let callee = self.lower_expr(b, *expr)?;
                match callee {
                    LRes::Val(value) => todo!(),
                    LRes::Vals(values) => todo!(),
                    LRes::Func(func_ref) => {
                        b.ins().call(func_ref, &fn_args);
                    }
                    LRes::Builtin(_) => todo!(),
                    LRes::StackS(stack_slot) => todo!(),
                    LRes::Unit => todo!(),
                }
                Some(())
            }
            ast::Expr::FieldAccess {
                object,
                field_id,
                struct_layout,
                element_layout,
            } => todo!(),
            ast::Expr::Block {
                exprs,
                last_expr,
                block_tp,
            } => todo!(),
            ast::Expr::Return { expr, ret_tp } => todo!(),
            ast::Expr::Let {
                id,
                layout,
                is_mut,
                expr,
            } => todo!(),
            ast::Expr::Assign {
                lval,
                rval,
                assign_tp,
            } => todo!(),
            ast::Expr::Ref { var, tp } => todo!(),
            ast::Expr::RefMut { var, tp } => todo!(),
            ast::Expr::Deref { expr, in_tp } => todo!(),
            ast::Expr::Char(_) => todo!(),
            ast::Expr::ArrayInitRepeat(expr, _, _) => todo!(),
            ast::Expr::ArrayInitExact(exprs, _) => todo!(),
            ast::Expr::While { pred, block } => todo!(),
            ast::Expr::IndexAccess {
                arr,
                index,
                arr_layout,
                elem_layout,
            } => todo!(),
            ast::Expr::Var(var_ref) => todo!(),
        }
    }

    fn declare_sym(&mut self, id: NodeID, f: &ast::Symbol) {
        match &f.kind {
            ast::SymKind::Func { args, returns } => {
                let mut sig = self.m.make_signature();

                for tp in args {
                    let param = AbiParam::new(tp.to_cl_type());
                    sig.params.push(param);
                }
                for tp in returns {
                    let param = AbiParam::new(tp.to_cl_type());
                    sig.returns.push(param);
                }

                let name = if f.mangle {
                    &format!("id_{}", id.get())
                } else {
                    &f.name
                };

                let link = if f.is_extern {
                    Linkage::Export
                } else {
                    Linkage::Local
                };

                let func_id = self.m.declare_function(name, link, &sig).unwrap();

                self.id_fn_map.insert(id, func_id);
            }
            ast::SymKind::BuiltinFunc {
                args,
                returns,
                item_name,
            } => {
                let mut sig = self.m.make_signature();

                for tp in args {
                    let param = AbiParam::new(tp.to_cl_type());
                    sig.params.push(param);
                }
                for tp in returns {
                    let param = AbiParam::new(tp.to_cl_type());
                    sig.returns.push(param);
                }

                let name = if f.mangle {
                    &format!("id_{}", id.get())
                } else {
                    &f.name
                };

                let link = if f.is_extern {
                    Linkage::Export
                } else {
                    Linkage::Local
                };

                let func_id = self.m.declare_function(name, link, &sig).unwrap();

                self.id_fn_map.insert(id, func_id);

                let mut ctx = self.m.make_context();
                let mut fn_ctx = FunctionBuilderContext::new();

                match item_name.as_str() {
                    "i32_add" => {
                        ctx.func.signature = sig.clone();
                        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fn_ctx);
                        let block = b.create_block();
                        b.append_block_params_for_function_params(block);
                        b.switch_to_block(block);
                        b.seal_block(block);
                        let vals = b.block_params(block);
                        let v1 = vals[0];
                        let v2 = vals[1];
                        let val = b.ins().iadd(v1, v2);
                        b.ins().return_(&[val]);
                        // println!("{}", b.func.display());
                        b.finalize();
                        match self.m.define_function(func_id, &mut ctx) {
                            Ok(o) => (),
                            Err(e) => println!("{:#?}", e),
                        }
                    }
                    _ => todo!(),
                }
                self.m.clear_context(&mut ctx);
                self.m.clear_signature(&mut sig);
            }
        }
    }

    fn write_to_addr(&self, b: &mut FunctionBuilder<'_>, e: ast::Expr, addr: Value, offset: usize) {
        match e {
            ast::Expr::NumLit(n, tp) => {
                let v = b.ins().iconst(tp.to_cl_type(), n as i64);
                b.ins().store(MemFlags::new(), v, addr, offset as i32);
            }
            ast::Expr::StringLit(_, layout) => todo!(),
            ast::Expr::Tuple { fields, layout } => {
                for (expr, f_layout) in fields {
                    self.write_to_addr(b, expr, addr, offset + f_layout.offset);
                }
            }
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
            } => todo!(),
            ast::Expr::Block {
                exprs,
                last_expr,
                block_tp,
            } => todo!(),
            ast::Expr::Return { expr, ret_tp } => todo!(),
            ast::Expr::Let {
                id,
                layout,
                is_mut,
                expr,
            } => todo!(),
            ast::Expr::Assign {
                lval,
                rval,
                assign_tp,
            } => todo!(),
            ast::Expr::Ref { var, tp } => todo!(),
            ast::Expr::RefMut { var, tp } => todo!(),
            ast::Expr::Deref { expr, in_tp } => todo!(),
            ast::Expr::Char(_) => todo!(),
            ast::Expr::ArrayInitRepeat(expr, _, _) => todo!(),
            ast::Expr::ArrayInitExact(exprs, _) => todo!(),
            ast::Expr::While { pred, block } => todo!(),
            ast::Expr::IndexAccess {
                arr,
                index,
                arr_layout,
                elem_layout,
            } => todo!(),
            ast::Expr::Var(var_ref) => match var_ref {
                ast::VarRef::Local(var_id) => match self.variables.get(&var_id).unwrap() {
                    Var::Reg(var) => {
                        let val = b.use_var(*var);
                        b.ins().store(MemFlags::new(), val, addr, offset as i32);
                    }
                    Var::Stack {
                        ss,
                        layout,
                        offset: ss_offset,
                    } => store_from_layout(b, addr, offset, *ss, layout, *ss_offset),
                    Var::FnArg(val) => {
                        b.ins().store(MemFlags::new(), *val, addr, offset as i32);
                    }
                },
                ast::VarRef::Global(node_id) => todo!(),
            },
        }
    }
}

fn store_from_layout(
    b: &mut FunctionBuilder<'_>,
    addr: Value,
    offset: usize,
    ss: StackSlot,
    layout: &Layout,
    ss_offset: usize,
) {
    match &layout.layout {
        ast::TypeLayout::Simple { tp } => {
            let val = b.ins().stack_load(tp.to_cl_type(), ss, ss_offset as i32);
            b.ins().store(MemFlags::new(), val, addr, offset as i32);
        }
        ast::TypeLayout::Array { elem_layout, elems } => todo!(),
        ast::TypeLayout::Tuple {
            field_count,
            fields,
        } => {
            for layout in fields {
                store_from_layout(b, addr, offset + layout.offset, ss, &layout, ss_offset);
            }
        }
    }
}
