mod command_definition;
mod engine;
mod line_reader;
mod opcode;
mod program_load;
mod string_memory;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
struct CLIArguments {
    #[structopt(name = "FILE")]
    file: PathBuf,
}

fn main() {
    let args = CLIArguments::from_args();
    let (prog, str_mem) = program_load::load_program(&args.file).unwrap();
    println!("{:?}", prog);
    engine::run_program(prog, str_mem);
}
