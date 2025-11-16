use std::collections::HashMap;

use crate::common::NodeID;
use crate::error::InternalError;
use cranelift_codegen::ir::{
    InstBuilder, MemFlags, Signature, StackSlotData, StackSlotKind, Value, types::*,
};
use cranelift_codegen::settings::Configurable;
use cranelift_codegen::{ir::AbiParam, isa, settings};

use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};

use cranelift_object::{ObjectModule, ObjectProduct};

use crate::core::ast;

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
    variables: HashMap<ast::VarID, Value>,
}

impl<'ctx> Lowerer<'ctx> {
    pub fn new(m: &'ctx mut ObjectModule) -> Self {
        Self {
            m,
            id_fn_map: HashMap::new(),
            variables: HashMap::new(),
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

        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fn_ctx);

        let block = b.create_block();
        b.append_block_params_for_function_params(block);
        b.switch_to_block(block);
        b.seal_block(block);

        let fn_args = b.block_params(block);

        for (val, (var, _)) in fn_args.iter().zip(f.args) {
            self.variables.insert(var, *val);
        }

        let val = self.lower_expr(&mut b, f.body);

        if let Some(v) = val {
            b.ins().return_(&[v]);
        } else {
            b.ins().return_(&[]);
        }

        println!("{}", b.func.display());

        b.finalize();

        match self.m.define_function(func, &mut ctx) {
            Ok(o) => (),
            Err(e) => println!("{:#?}", e),
        }

        self.variables.clear();
        self.m.clear_context(&mut ctx);
    }

    pub fn lower_expr(&mut self, b: &mut FunctionBuilder, e: ast::Expr) -> Option<Value> {
        match e {
            ast::Expr::FunCall { expr, args, sig } => {
                let mut fn_args = vec![];
                for arg in args {
                    if let Some(v) = self.lower_expr(b, arg) {
                        fn_args.push(v)
                    }
                }
                let callee = self.lower_expr(b, *expr).unwrap();
                let sig = self.sig_from_core(sig);
                let sig_ref = b.import_signature(sig);
                let inst = b.ins().call_indirect(sig_ref, callee, &fn_args);
                let ret = b.inst_results(inst);
                ret.get(0).map(|f| *f)
            }
            ast::Expr::Return { expr } => {
                if let Some(v) = self.lower_expr(b, *expr) {
                    b.ins().return_(&[v]);
                } else {
                    b.ins().return_(&[]);
                }
                None
            }
            ast::Expr::Let { id, e1, e2 } => {
                if let Some(v) = self.lower_expr(b, *e1) {
                    self.variables.insert(id, v);
                }
                self.lower_expr(b, *e2)
            }
            ast::Expr::While { pred, block } => todo!(),
            ast::Expr::Value(value) => self.tr_value(b, value),
            ast::Expr::StackSlot { size } => {
                let ss = b.create_sized_stack_slot(StackSlotData {
                    kind: StackSlotKind::ExplicitSlot,
                    size,
                    align_shift: 0,
                });
                let v = b.ins().stack_addr(I64, ss, 0);
                Some(v)
            }
            ast::Expr::Store { ptr, val, offset } => {
                let x = self.lower_expr(b, *val)?;
                let p = self.lower_expr(b, *ptr)?;
                b.ins().store(MemFlags::new(), x, p, offset);
                None
            }
            ast::Expr::Load { tp, ptr, offset } => {
                let p = self.lower_expr(b, *ptr).unwrap();
                let v = b.ins().load(tp.to_cl_type(), MemFlags::new(), p, offset);
                Some(v)
            }
            ast::Expr::Ignore { e1, e2 } => {
                self.lower_expr(b, *e1);
                self.lower_expr(b, *e2)
            }
        }
    }

    pub fn tr_value(&mut self, b: &mut FunctionBuilder, v: ast::Value) -> Option<Value> {
        match v {
            ast::Value::Unit => None,
            ast::Value::Var(var_ref) => match var_ref {
                ast::VarRef::Local(var_id) => {
                    let v = *self.variables.get(&var_id).unwrap();
                    Some(v)
                }
                ast::VarRef::Global(node_id) => {
                    let f_id = *self.id_fn_map.get(&node_id).unwrap();
                    let f_ref = self.m.declare_func_in_func(f_id, b.func);
                    let v = b.ins().func_addr(I64, f_ref);
                    Some(v)
                }
            },
            ast::Value::Const(n, tp) => {
                let v = b.ins().iconst(tp.to_cl_type(), n as i64);
                Some(v)
            }
        }
    }

    fn sig_from_core(&self, fn_sig: ast::FnSig) -> Signature {
        let mut sig = self.m.make_signature();
        for param in fn_sig.params {
            sig.params.push(AbiParam::new(param.to_cl_type()));
        }
        for param in fn_sig.returns {
            sig.returns.push(AbiParam::new(param.to_cl_type()));
        }
        sig
    }
}
