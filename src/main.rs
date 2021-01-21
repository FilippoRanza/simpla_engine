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

fn compile_and_run(file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let (prog, prog_mem, str_mem) = program_load::load_program(file)?;
    engine::run_program(prog, prog_mem, str_mem)?;
    Ok(())
}

fn main() {
    let args = CLIArguments::from_args();
    let status = compile_and_run(&args.file);
    match status {
        Ok(()) => {},
        Err(err) => eprintln!("{}", err)
    }
}
