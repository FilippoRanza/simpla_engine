use crate::command_definition::{Command, MathOperator, Program};
use std::cmp::{PartialEq, PartialOrd};
use std::ops::{Add, Div, Mul, Sub};

pub fn run_program(prog: Program) -> Result<(), RuntimeError> {
    let mut stack_vect: Vec<Record> = Vec::new();

    let mut curr_block = &prog.body;
    let mut index: usize = 0;

    let mut int_stack: Vec<i32> = Vec::new();
    let mut real_stack: Vec<f64> = Vec::new();
    let mut bool_stack: Vec<bool> = Vec::new();
    let mut str_stack: Vec<&str> = Vec::new();

    while index < curr_block.len() {
        let cmd = &curr_block[index];
        match cmd {
            Command::Integer(cmd) => full_math_operation(&cmd, &mut int_stack, &mut bool_stack),
            Command::Real(cmd) => full_math_operation(&cmd, &mut real_stack, &mut bool_stack),
            Command::CastInt => {
                let n = real_stack.pop().unwrap();
                let i = n as i32;
                int_stack.push(i);
            }
            Command::CastReal => {
                let i = int_stack.pop().unwrap();
                let n = i as f64;
                real_stack.push(n);
            }
            Command::And | Command::Or => boolean_operation(cmd, &mut bool_stack),
            Command::MemoryLoad(load, add) => {}
            Command::MemoryStore(store, add) => {}
            Command::Control(ctrl, addr) => {}
            Command::Input(k) => {}
            Command::Output(k) => {}
            Command::Exit => {}
            Command::ConstantLoad(load) => {}
            Command::ConstantStore(store) => {}
        }
    }

    Ok(())
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
