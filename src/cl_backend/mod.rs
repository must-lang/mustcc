use crate::cfg::ast;
use crate::cl_backend::env::Env;
use crate::error::InternalError;
use cranelift_codegen::ir::types::*;
use cranelift_codegen::{
    ir::{AbiParam, Type},
    isa, settings,
};
mod env;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module};
use cranelift_object::{ObjectModule, ObjectProduct};
impl ast::Type {
    pub(crate) fn to_abi_param(&self) -> cranelift_codegen::ir::AbiParam {
        todo!()
    }
}

pub fn translate(prog: ast::Program) -> Result<ObjectProduct, InternalError> {
    let flags = settings::Flags::new(settings::builder());
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

    let mut sig = module.make_signature();

    let mut env = Env::new();

    for f in prog.functions {
        for (_, tp) in f.args {
            sig.params.push(tp.to_abi_param());
        }
        for tp in f.returns {
            sig.returns.push(tp.to_abi_param());
        }

        let func_id = module
            .declare_function(&f.name, Linkage::Local, &sig)
            .unwrap();

        env.add_func(f.id, func_id);
        module.clear_signature(&mut sig);
    }

    let obj = module.finish();
    Ok(obj)
}

pub fn emit_func(module: &mut ObjectModule, f: ast::Func) {
    let mut ctx = module.make_context();
    let mut fn_builder_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fn_builder_ctx);

    // translate

    builder.finalize();
}
