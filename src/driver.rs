use crate::{
    Cli, codegen, core,
    error::{InternalError, ariadne_renderer::AriadneRenderer, context::Context},
    mir, mod_tree,
    parser::parse_project,
    resolve, typecheck,
};

/// Run the compiler.
pub fn run(config: Cli) -> Result<(), InternalError> {
    let mut ctx = Context::init(Box::new(AriadneRenderer::new()));

    let prog = parse_project(&mut ctx)?;

    if config.print_input_ast {
        println!("{:#?}", prog);
        ctx.finish()?;
        return Ok(());
    }

    let prog = mod_tree::translate(&mut ctx, prog)?;

    let prog = resolve::translate(&mut ctx, prog)?;

    let prog = typecheck::translate(&mut ctx, prog)?;

    let error_count = ctx.finish()?;

    if error_count != 0 {
        println!("{} errors occurred, compilation aborted.", error_count);
        return Ok(());
    }

    if config.typecheck_only {
        return Ok(());
    }

    let prog = mir::translate(prog)?;

    let prog = core::translate(prog);

    if config.core_dump {
        println!("{:#?}", prog);
        return Ok(());
    }

    let obj = codegen::translate(prog)?;

    let obj_bytes = obj.emit().unwrap();
    std::fs::write("output.o", obj_bytes).unwrap();

    Ok(())
}
