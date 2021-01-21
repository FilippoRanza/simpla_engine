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


fn compile_and_run(file: &PathBuf) -> Result<(), String> {
    let res = program_load::load_program(file);
    let (prog, prog_mem, str_mem) = match res {
        Ok((prog, prog_mem, str_mem)) => (prog, prog_mem, str_mem),
        Err(err) => return Err(format!("Error while loading {:?}\n{}", file, err))
    };

    let run_stat = engine::run_program(prog, prog_mem, str_mem);
    match run_stat {
        Ok(()) => Ok(()),
        Err(err) => Err(format!("Error while running {:?}\n{}", file, err))
    }
}

fn main() {
    let args = CLIArguments::from_args();
    let status = compile_and_run(&args.file);
    match status {
        Ok(()) => {},
        Err(err) => eprintln!("{}", err)
    }
}
