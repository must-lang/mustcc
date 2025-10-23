use crate::{
    Cli,
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

    let prog = match mod_tree::translate(&mut ctx, prog) {
        Ok(prog) => prog,
        Err(err) => {
            ctx.finish()?;
            return Err(err);
        }
    };

    let prog = match resolve::translate(&mut ctx, prog) {
        Ok(prog) => prog,
        Err(err) => {
            ctx.finish()?;
            return Err(err);
        }
    };

    let prog = match typecheck::translate(&mut ctx, prog) {
        Ok(prog) => prog,
        Err(err) => {
            ctx.finish()?;
            return Err(err);
        }
    };

    // println!("{prog:#?}");

    ctx.finish()?;

    Ok(())
}
