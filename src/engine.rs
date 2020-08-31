use std::collections::HashMap;

use crate::command_definition::{
    AddrSize, Block, Command, Constant, ControlFlow, FlushMode, Kind, MathOperator, Program,
};
use crate::for_loop_stack::ForLoopStack;
use crate::line_reader::{LineReader, ReadError};
use crate::reference_memory::{ReferenceCount, ReferenceStack};
use crate::string_memory::StringMemory;
use std::cmp::{PartialEq, PartialOrd};
use std::fmt;
use std::io::{stdout, Write};
use std::ops::{Add, Div, Mul, Sub};

const ADDR_SIZE_ZERO: AddrSize = 0;
const LOCAL_MASK: AddrSize = 1 << (ADDR_SIZE_ZERO.count_zeros() - 1);

pub fn run_program(prog: Program, mut string_memory: StringMemory) -> Result<(), RuntimeError> {
    let mut stack_vect: Vec<Record> = Vec::new();

    let mut curr_block = &prog.body;
    let mut index: usize = 0;

    let mut global_memory = EngineMemory::new();
    let mut engine_stack = EngineStack::new();

    let mut reader = LineReader::new(true);

    let mut next_record: Option<Record> = None;
    let mut for_loop_stack = ForLoopStack::new();

    while index < curr_block.code.len() {
        let cmd = &curr_block.code[index];
        index += 1;
        string_memory.clean();
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
                memory_load(
                    load,
                    *add,
                    &mut engine_stack,
                    &global_memory,
                    local,
                    &mut string_memory,
                );
            }
            Command::MemoryStore(store, add) => {
                let local = if let Some(last) = stack_vect.last_mut() {
                    Some(&mut last.func_mem)
                } else {
                    None
                };
                memory_store(
                    store,
                    *add,
                    &mut engine_stack,
                    &mut global_memory,
                    local,
                    &mut string_memory,
                )
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
            Command::Input(k) => input(k, &mut engine_stack, &mut reader, &mut string_memory)?,
            Command::Output(k) => output(k, &mut engine_stack, &mut string_memory),
            Command::Flush(mode) => handle_flush(mode),
            Command::Exit => break,
            Command::ConstantLoad(load) => {
                load_constant(load, &mut engine_stack, &mut string_memory)
            }
            Command::StoreParam(k, addr) => {
                if let Some(ref mut record) = next_record {
                    let local_memory = Some(&mut record.func_mem);
                    memory_store(
                        k,
                        *addr,
                        &mut engine_stack,
                        &mut global_memory,
                        local_memory,
                        &mut string_memory,
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
            Command::ForControl(control) => {
                for_loop_stack.process_command(control, &mut engine_stack.int_stack)
            }
        }
    }

    Ok(())
}

struct EngineStack {
    int_stack: Vec<i32>,
    real_stack: Vec<f64>,
    bool_stack: Vec<bool>,
    str_stack: ReferenceStack,
}

impl EngineStack {
    fn new() -> Self {
        Self {
            int_stack: vec![],
            real_stack: vec![],
            bool_stack: vec![],
            str_stack: ReferenceStack::new(),
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
    addr: AddrSize,
    stack: &mut EngineStack,
    global: &EngineMemory,
    local: Option<&EngineMemory>,
    str_mem: &mut StringMemory,
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
            stack.str_stack.push(str_mem, *s)
        }
    }
}

fn memory_store(
    k: &Kind,
    addr: AddrSize,
    stack: &mut EngineStack,
    global: &mut EngineMemory,
    local: Option<&mut EngineMemory>,
    str_mem: &mut StringMemory,
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
            let b = stack.str_stack.pop(str_mem);
            str_mem.increment(&b);
            let prev = set_value(&mut global.str_mem, loc, addr, b);
            clean_prev(prev, str_mem);
        }
    }
}

fn clean_prev(prev: Option<usize>, str_mem: &mut StringMemory) {
    if let Some(prev) = prev {
        str_mem.decrement(&prev);
    }
}

fn get_value<'a, T>(
    glob: &'a HashMap<AddrSize, T>,
    loc: Option<&'a HashMap<AddrSize, T>>,
    addr: AddrSize,
) -> &'a T {
    if addr & LOCAL_MASK == 0 {
        glob.get(&addr).unwrap()
    } else {
        let loc = loc.unwrap();
        loc.get(&addr).unwrap()
    }
}

fn set_value<'a, T>(
    glob: &'a mut HashMap<AddrSize, T>,
    loc: Option<&'a mut HashMap<AddrSize, T>>,
    addr: AddrSize,
    value: T,
) -> Option<T>
where
    T: Copy,
{
    if addr & LOCAL_MASK == 0 {
        insert_and_get_prev(glob, addr, value)
    } else {
        let loc = loc.unwrap();
        insert_and_get_prev(loc, addr, value)
    }
}

fn insert_and_get_prev<T>(map: &mut HashMap<AddrSize, T>, addr: AddrSize, value: T) -> Option<T>
where
    T: Copy,
{
    let output = if let Some(prev) = map.get(&addr) {
        Some(*prev)
    } else {
        None
    };
    map.insert(addr, value);
    output
}

fn load_constant(load: &Constant, stack: &mut EngineStack, str_mem: &mut StringMemory) {
    match load {
        Constant::Bool(b) => stack.bool_stack.push(*b),
        Constant::Integer(i) => stack.int_stack.push(*i),
        Constant::Real(r) => stack.real_stack.push(*r),
        Constant::Str(s) => stack.str_stack.push(str_mem, *s),
    }
}

fn input(
    k: &Kind,
    stack: &mut EngineStack,
    reader: &mut LineReader,
    str_mem: &mut StringMemory,
) -> Result<(), ReadError> {
    match k {
        Kind::Bool => {
            let tmp = reader.next_bool()?;
            stack.bool_stack.push(tmp);
        }
        Kind::Integer => {
            let tmp = reader.next_i32()?;
            stack.int_stack.push(tmp);
        }
        Kind::Real => {
            let tmp = reader.next_f64()?;
            stack.real_stack.push(tmp);
        }
        Kind::Str => {
            let tmp = reader.next_string()?;
            let index = str_mem.insert_string(tmp);
            stack.str_stack.push(str_mem, index);
            str_mem.decrement(&index);
        }
    }
    Ok(())
}

fn output(k: &Kind, stack: &mut EngineStack, str_mem: &mut StringMemory) {
    match k {
        Kind::Bool => {
            let b = stack.bool_stack.pop().unwrap();
            print!("{}", b);
        }
        Kind::Integer => {
            let i = stack.int_stack.pop().unwrap();
            print!("{}", i);
        }
        Kind::Real => {
            let r = stack.real_stack.pop().unwrap();
            print!("{}", r);
        }
        Kind::Str => {
            let index = stack.str_stack.pop(str_mem);
            let s = str_mem.get_string(index);
            print!("{}", s);
        }
    };
}

fn handle_flush(mode: &FlushMode) {
    match mode {
        FlushMode::Flush => stdout().flush().unwrap(),
        FlushMode::NewLine => println!(),
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
    int_mem: HashMap<AddrSize, i32>,
    real_mem: HashMap<AddrSize, f64>,
    bool_mem: HashMap<AddrSize, bool>,
    str_mem: HashMap<AddrSize, usize>,
}

impl EngineMemory {
    fn new() -> Self {
        Self {
            int_mem: HashMap::new(),
            real_mem: HashMap::new(),
            bool_mem: HashMap::new(),
            str_mem: HashMap::new(),
        }
    }
}

pub enum RuntimeError {
    ReadError(ReadError),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadError(io_err) => write!(f, "{}", io_err),
        }
    }
}

impl std::convert::From<ReadError> for RuntimeError {
    fn from(e: ReadError) -> RuntimeError {
        RuntimeError::ReadError(e)
    }
}

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
