mod command_definition;
mod engine;
mod for_loop_stack;
mod line_reader;
mod opcode;
mod program_load;
mod reference_memory;
mod string_memory;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(about = "Execute a Simpla program")]
struct CLIArguments {
    #[structopt(name = "Bytecode File", help = "Simpla bytecode file")]
    file: PathBuf,
}

fn main() {
    let args = CLIArguments::from_args();
    let (prog, str_mem) = program_load::load_program(&args.file).unwrap();
    match engine::run_program(prog, str_mem) {
        Ok(()) => {}
        Err(err) => println!("{}", err),
    };
}
