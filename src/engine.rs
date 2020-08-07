use crate::command_definition::{Command, Kind, MathOperator, Program};
use crate::line_reader::LineReader;
use std::cmp::{PartialEq, PartialOrd};
use std::ops::{Add, Div, Mul, Sub};

pub fn run_program(prog: Program) -> Result<(), RuntimeError> {
    let mut stack_vect: Vec<Record> = Vec::new();

    let mut curr_block = &prog.body;
    let mut index: usize = 0;

    let mut engine_stack = EngineStack::new();

    let mut reader = LineReader::new(true);

    while index < curr_block.len() {
        let cmd = &curr_block[index];
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
            Command::MemoryLoad(load, add) => {}
            Command::MemoryStore(store, add) => {}
            Command::Control(ctrl, addr) => {}
            Command::Input(k) => input(k, &mut engine_stack, &mut reader),
            Command::Output(k) => output(k, &mut engine_stack, OutputMode::SameLine),
            Command::OutputLine(k) => output(k, &mut engine_stack, OutputMode::NewLine),
            Command::Exit => {}
            Command::ConstantLoad(load) => {}
            Command::ConstantStore(store) => {}
        }
    }

    Ok(())
}

struct EngineStack {
    int_stack: Vec<i32>,
    real_stack: Vec<f64>,
    bool_stack: Vec<bool>,
    str_stack: Vec<String>,
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

fn input(k: &Kind, stack: &mut EngineStack, reader: &mut LineReader) {
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
            stack.str_stack.push(tmp);
        }
    }
}

enum OutputMode {
    NewLine,
    SameLine,
}

fn output(k: &Kind, stack: &mut EngineStack, m: OutputMode) {
    let output = match k {
        Kind::Bool => {
            let b = stack.bool_stack.pop().unwrap();
            format!("{}", b)
        }
        Kind::Integer => {
            let i = stack.int_stack.pop().unwrap();
            format!("{}", i)
        }
        Kind::Real => {
            let r = stack.real_stack.pop().unwrap();
            format!("{}", r)
        }
        Kind::Str => {
            let s = stack.str_stack.pop().unwrap();
            format!("{}", s)
        }
    };

    match m {
        OutputMode::SameLine => print!("{}", output),
        OutputMode::NewLine => println!("{}", output),
    };
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
    let a = stack.pop().unwrap();
    let b = stack.pop().unwrap();
    match op {
        MathOperator::Add => {
            let c = a + b;
            NumResult::Number(c)
        }
        MathOperator::Sub => {
            let c = a - b;
            NumResult::Number(c)
        }
        MathOperator::Mul => {
            let c = a * b;
            NumResult::Number(c)
        }
        MathOperator::Div => {
            let c = a / b;
            NumResult::Number(c)
        }
        MathOperator::GreatEq => {
            let c = a >= b;
            NumResult::Boolean(c)
        }
        MathOperator::Greater => {
            let c = a > b;
            NumResult::Boolean(c)
        }
        MathOperator::LessEq => {
            let c = a <= b;
            NumResult::Boolean(c)
        }
        MathOperator::Less => {
            let c = a < b;
            NumResult::Boolean(c)
        }
        MathOperator::Equal => {
            let c = a == b;
            NumResult::Boolean(c)
        }
        MathOperator::NotEqual => {
            let c = a != b;
            NumResult::Boolean(c)
        }
    }
}

enum NumResult<T> {
    Number(T),
    Boolean(bool),
}

pub enum RuntimeError {}

struct Record<'a> {
    return_index: usize,
    return_block: &'a Vec<Command>,

    int_mem: Vec<i32>,
    real_mem: Vec<f64>,
    bool_mem: Vec<bool>,
    str_mem: Vec<String>,
}
