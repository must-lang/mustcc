use std::fs::File;

use crate::{
    Cli, codegen,
    error::{InternalError, ariadne_renderer::AriadneRenderer, context::Context},
    mod_tree,
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

    let prog = codegen::translate(prog)?;

    let mut output = File::create("output.c")?;

    codegen::emit_code(prog, &mut output)?;

    Ok(())
}
