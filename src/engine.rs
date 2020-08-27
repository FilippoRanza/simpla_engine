use crate::command_definition::{
    Block, Command, Constant, ControlFlow, Kind, MathOperator, Program,
};
use crate::line_reader::LineReader;
use crate::string_memory::StringMemory;
use std::cmp::{PartialEq, PartialOrd};
use std::ops::{Add, Div, Mul, Sub};

pub fn run_program(prog: Program, mut string_memory: StringMemory) -> Result<(), RuntimeError> {
    let mut stack_vect: Vec<Record> = Vec::new();

    let mut curr_block = &prog.body;
    let mut index: usize = 0;

    let mut global_memory = EngineMemory::new();
    let mut engine_stack = EngineStack::new();

    let mut reader = LineReader::new(true);

    let mut next_record: Option<Record> = None;

    while index < curr_block.code.len() {
        let cmd = &curr_block.code[index];
        index += 1;
        match cmd {
            Command::Integer(cmd) => full_math_operation(
                &cmd,
                &mut engine_stack.int_stack,
                &mut engine_stack.bool_stack,
            ),
            Command::Real(cmd) => full_math_operation(
                &cmd,
                &mut engine_stack.real_stack,
                &mut engine_stack.bool_stack,
            ),
            Command::CastInt => {
                let n = engine_stack.real_stack.pop().unwrap();
                let i = n as i32;
                engine_stack.int_stack.push(i);
            }
            Command::CastReal => {
                let i = engine_stack.int_stack.pop().unwrap();
                let n = i as f64;
                engine_stack.real_stack.push(n);
            }
            Command::And | Command::Or => boolean_operation(cmd, &mut engine_stack.bool_stack),
            Command::MemoryLoad(load, add) => {
                let local = if let Some(last) = stack_vect.last_mut() {
                    Some(&last.func_mem)
                } else {
                    None
                };
                memory_load(load, *add, &mut engine_stack, &global_memory, local);
            }
            Command::MemoryStore(store, add) => {
                let local = if let Some(last) = stack_vect.last_mut() {
                    Some(&mut last.func_mem)
                } else {
                    None
                };
                memory_store(store, *add, &mut engine_stack, &mut global_memory, local)
            }
            Command::Control(ctrl, addr) => match ctrl {
                ControlFlow::Call => {
                    if let Some(block) = next_record {
                        let mut block = block;
                        block.return_index = index;
                        curr_block = &prog.func[*addr];
                        index = 0;
                        stack_vect.push(block);
                        next_record = None;
                    }
                }
                ControlFlow::Ret => {
                    if let Some(top) = stack_vect.pop() {
                        index = top.return_index;
                        curr_block = top.return_block;
                        string_memory.remove_strings(&top.func_mem.str_mem);
                    } else {
                        panic!("return outside function body");
                    }
                }
                ControlFlow::Label => {}
                jump => {
                    let next_addr = curr_block.labels[addr];
                    index = run_jump(jump, index, next_addr, &mut engine_stack.bool_stack);
                }
            },
            Command::Input(k) => input(k, &mut engine_stack, &mut reader, &mut string_memory),
            Command::Output(k) => {
                output(k, &mut engine_stack, OutputMode::SameLine, &string_memory)
            }
            Command::OutputLine(k) => {
                output(k, &mut engine_stack, OutputMode::NewLine, &string_memory)
            }
            Command::Exit => break,
            Command::ConstantLoad(load) => load_constant(load, &mut engine_stack),
            Command::StoreParam(k, addr) => {
                if let Some(ref mut record) = next_record {
                    let local_memory = Some(&mut record.func_mem);
                    memory_store(
                        k,
                        *addr,
                        &mut engine_stack,
                        &mut global_memory,
                        local_memory,
                    );
                } else {
                    panic!("cannot store parameter before initializing a new activation record");
                }
            }
            Command::NewRecord => {
                if next_record.is_none() {
                    next_record = Some(Record::new(curr_block));
                } else {
                    panic!("cannot initialize a new activation record")
                }
            }
        }
    }

    Ok(())
}

struct EngineStack {
    int_stack: Vec<i32>,
    real_stack: Vec<f64>,
    bool_stack: Vec<bool>,
    str_stack: Vec<usize>,
}

impl EngineStack {
    fn new() -> Self {
        Self {
            int_stack: vec![],
            real_stack: vec![],
            bool_stack: vec![],
            str_stack: vec![],
        }
    }
}

fn run_jump(j: &ControlFlow, curr: usize, next: usize, stack: &mut Vec<bool>) -> usize {
    match j {
        ControlFlow::Jump => next,
        ControlFlow::JumpTrue => {
            let b = stack.pop().unwrap();
            if b {
                next
            } else {
                curr
            }
        }
        ControlFlow::JumpFalse => {
            let b = stack.pop().unwrap();
            if !b {
                next
            } else {
                curr
            }
        }
        _ => unreachable!(),
    }
}

fn memory_load(
    k: &Kind,
    addr: usize,
    stack: &mut EngineStack,
    global: &EngineMemory,
    local: Option<&EngineMemory>,
) {
    match k {
        Kind::Bool => {
            let loc = if let Some(mem) = local {
                Some(&mem.bool_mem)
            } else {
                None
            };
            let b = get_value(&global.bool_mem, loc, addr);
            stack.bool_stack.push(*b);
        }
        Kind::Integer => {
            let loc = if let Some(mem) = local {
                Some(&mem.int_mem)
            } else {
                None
            };
            let i = get_value(&global.int_mem, loc, addr);
            stack.int_stack.push(*i);
        }
        Kind::Real => {
            let loc = if let Some(mem) = local {
                Some(&mem.real_mem)
            } else {
                None
            };
            let r = get_value(&global.real_mem, loc, addr);
            stack.real_stack.push(*r);
        }
        Kind::Str => {
            let loc = if let Some(mem) = local {
                Some(&mem.str_mem)
            } else {
                None
            };
            let s = get_value(&global.str_mem, loc, addr);
            stack.str_stack.push(*s)
        }
    }
}

fn memory_store(
    k: &Kind,
    addr: usize,
    stack: &mut EngineStack,
    global: &mut EngineMemory,
    local: Option<&mut EngineMemory>,
) {
    match k {
        Kind::Bool => {
            let loc = if let Some(mem) = local {
                Some(&mut mem.bool_mem)
            } else {
                None
            };
            let b = stack.bool_stack.pop().unwrap();
            set_value(&mut global.bool_mem, loc, addr, b);
        }
        Kind::Integer => {
            let loc = if let Some(mem) = local {
                Some(&mut mem.int_mem)
            } else {
                None
            };
            let b = stack.int_stack.pop().unwrap();
            set_value(&mut global.int_mem, loc, addr, b);
        }
        Kind::Real => {
            let loc = if let Some(mem) = local {
                Some(&mut mem.real_mem)
            } else {
                None
            };
            let b = stack.real_stack.pop().unwrap();
            set_value(&mut global.real_mem, loc, addr, b);
        }
        Kind::Str => {
            let loc = if let Some(mem) = local {
                Some(&mut mem.str_mem)
            } else {
                None
            };
            let b = stack.str_stack.pop().unwrap();
            set_value(&mut global.str_mem, loc, addr, b);
        }
    }
}

fn get_value<'a, T>(glob: &'a Vec<T>, loc: Option<&'a Vec<T>>, addr: usize) -> &'a T {
    if glob.len() > addr {
        &glob[addr]
    } else if let Some(loc) = loc {
        let addr = addr - glob.len();
        &loc[addr]
    } else {
        panic!()
    }
}

fn set_value<T>(glob: &mut Vec<T>, loc: Option<&mut Vec<T>>, addr: usize, value: T) {
    if glob.len() > addr {
        glob[addr] = value;
    } else if let Some(loc) = loc {
        let addr = addr - glob.len();
        loc[addr] = value;
    } else {
        panic!("addr: {} - {}", addr, glob.len());
    }
}

fn load_constant(load: &Constant, stack: &mut EngineStack) {
    match load {
        Constant::Bool(b) => stack.bool_stack.push(*b),
        Constant::Integer(i) => stack.int_stack.push(*i),
        Constant::Real(r) => stack.real_stack.push(*r),
        Constant::Str(s) => stack.str_stack.push(*s),
    }
}

fn input(k: &Kind, stack: &mut EngineStack, reader: &mut LineReader, str_mem: &mut StringMemory) {
    match k {
        Kind::Bool => {
            let tmp = reader.next_bool().unwrap();
            stack.bool_stack.push(tmp);
        }
        Kind::Integer => {
            let tmp = reader.next_i32().unwrap();
            stack.int_stack.push(tmp);
        }
        Kind::Real => {
            let tmp = reader.next_f64().unwrap();
            stack.real_stack.push(tmp);
        }
        Kind::Str => {
            let tmp = reader.next_string().unwrap();
            let index = str_mem.insert_string(tmp);
            stack.str_stack.push(index);
        }
    }
}

enum OutputMode {
    NewLine,
    SameLine,
}

fn output(k: &Kind, stack: &mut EngineStack, m: OutputMode, str_mem: &StringMemory) {
    match k {
        Kind::Bool => {
            let b = stack.bool_stack.pop().unwrap();
            let tmp = format!("{}", b);
            print(&tmp, m);
        }
        Kind::Integer => {
            let i = stack.int_stack.pop().unwrap();
            let tmp = format!("{}", i);
            print(&tmp, m);
        }
        Kind::Real => {
            let r = stack.real_stack.pop().unwrap();
            let tmp = format!("{}", r);
            print(&tmp, m);
        }
        Kind::Str => {
            let s = stack.str_stack.pop().unwrap();
            let s = str_mem.get_string(s);
            print(s, m);
        }
    };
}

fn print(s: &str, mode: OutputMode) {
    match mode {
        OutputMode::NewLine => println!("{}", s),
        OutputMode::SameLine => print!("{}", s),
    }
}

fn boolean_operation(cmd: &Command, stack: &mut Vec<bool>) {
    let a = stack.pop().unwrap();
    let b = stack.pop().unwrap();
    let c = match cmd {
        Command::And => a && b,
        Command::Or => a || b,
        _ => unreachable!(),
    };
    stack.push(c);
}

fn full_math_operation<T>(op: &MathOperator, numbers: &mut Vec<T>, booleans: &mut Vec<bool>)
where
    T: Add<Output = T>
        + Sub<Output = T>
        + Mul<Output = T>
        + Div<Output = T>
        + PartialOrd
        + PartialEq,
{
    let res = math_operation(op, numbers);
    match res {
        NumResult::Number(n) => numbers.push(n),
        NumResult::Boolean(b) => booleans.push(b),
    }
}

fn math_operation<T>(op: &MathOperator, stack: &mut Vec<T>) -> NumResult<T>
where
    T: Add<Output = T>
        + Sub<Output = T>
        + Mul<Output = T>
        + Div<Output = T>
        + PartialOrd
        + PartialEq,
{
    let rhs = stack.pop().unwrap();
    let lhs = stack.pop().unwrap();
    match op {
        MathOperator::Add => {
            let c = lhs + rhs;
            NumResult::Number(c)
        }
        MathOperator::Sub => {
            let c = lhs - rhs;
            NumResult::Number(c)
        }
        MathOperator::Mul => {
            let c = lhs * rhs;
            NumResult::Number(c)
        }
        MathOperator::Div => {
            let c = lhs / rhs;
            NumResult::Number(c)
        }
        MathOperator::GreatEq => {
            let c = lhs >= rhs;
            NumResult::Boolean(c)
        }
        MathOperator::Greater => {
            let c = lhs > rhs;
            NumResult::Boolean(c)
        }
        MathOperator::LessEq => {
            let c = lhs <= rhs;
            NumResult::Boolean(c)
        }
        MathOperator::Less => {
            let c = lhs < rhs;
            NumResult::Boolean(c)
        }
        MathOperator::Equal => {
            let c = lhs == rhs;
            NumResult::Boolean(c)
        }
        MathOperator::NotEqual => {
            let c = lhs != rhs;
            NumResult::Boolean(c)
        }
    }
}

enum NumResult<T> {
    Number(T),
    Boolean(bool),
}

struct EngineMemory {
    int_mem: Vec<i32>,
    real_mem: Vec<f64>,
    bool_mem: Vec<bool>,
    str_mem: Vec<usize>,
}
impl EngineMemory {
    fn new() -> Self {
        Self {
            int_mem: vec![],
            real_mem: vec![],
            bool_mem: vec![],
            str_mem: vec![],
        }
    }
}

pub enum RuntimeError {}

struct Record<'a> {
    return_index: usize,
    return_block: &'a Block,
    func_mem: EngineMemory,
}

impl<'a> Record<'a> {
    fn new(return_block: &'a Block) -> Self {
        Self {
            return_index: 0,
            return_block,
            func_mem: EngineMemory::new(),
        }
    }
}
