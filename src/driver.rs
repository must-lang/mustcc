use crate::{
    Cli,
    error::{InternalError, ariadne_renderer::AriadneRenderer, context::Context},
    parser::parse_project,
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

    Ok(())
}
