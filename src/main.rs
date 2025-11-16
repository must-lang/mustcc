use std::path::PathBuf;

use clap::Parser;

mod codegen;
mod common;
mod core;
mod driver;
mod error;
mod mir;
mod mod_tree;
mod parser;
mod resolve;
mod symtable;
mod tp;
mod typecheck;

#[derive(Parser)]
#[command(name = "mustcc", version, about = "Must Compiler Compiler")]
pub struct Cli {
    /// Path to project root directory
    #[arg(value_name = "PATH", default_value = ".", value_hint = clap::ValueHint::DirPath)]
    dir: PathBuf,

    /// Only print parsed AST and exit
    #[arg(short, long, default_value_t = false)]
    print_input_ast: bool,

    /// Only check types and exit
    #[arg(short, long, default_value_t = false)]
    typecheck_only: bool,

    /// Print program in core IR
    #[arg(short, long, default_value_t = false)]
    core_dump: bool,
}

/// Entry point, parses command line arguments and starts the compiler pipeline.
pub fn main() {
    let cli = Cli::parse();
    std::env::set_current_dir(&cli.dir).unwrap();
    if let Err(e) = driver::run(cli) {
        eprintln!("Internal error: {:#?}", e);
    }
}
