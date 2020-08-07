use crate::opcode;

#[derive(Debug)]
pub struct Program {
    pub body: Vec<Command>,
    pub func: Vec<Vec<Command>>,
}

#[derive(Debug)]
pub enum Command {
    Integer(MathOperator),
    Real(MathOperator),
    CastInt,
    CastReal,
    And,
    Or,
    MemoryLoad(Kind, usize),
    MemoryStore(Kind, usize),
    Control(ControlFlow, usize),
    Input(Kind),
    Output(Kind),
    OutputLine(Kind),
    Exit,
    ConstantLoad(Constant),
    ConstantStore(Constant),
}
#[derive(Debug)]
pub enum Kind {
    Integer,
    Real,
    Str,
    Bool,
}

impl Kind {
    pub fn new(byte: u8) -> Self {
        match byte % 4 {
            0 => Self::Integer,
            1 => Self::Real,
            2 => Self::Bool,
            3 => Self::Str,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub enum MathOperator {
    Add,
    Sub,
    Mul,
    Div,
    GreatEq,
    Greater,
    LessEq,
    Less,
    Equal,
    NotEqual,
}

impl MathOperator {
    pub fn new(b: u8) -> Self {
        match b {
            0 => Self::Add,
            1 => Self::Sub,
            2 => Self::Mul,
            3 => Self::Div,
            4 => Self::GreatEq,
            5 => Self::Greater,
            6 => Self::LessEq,
            7 => Self::Less,
            8 => Self::Equal,
            9 => Self::NotEqual,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub enum ControlFlow {
    Jump,
    JumpTrue,
    JumpFalse,
    Label,
    Call,
    Ret,
}

impl ControlFlow {
    pub fn new(byte: u8) -> Self {
        match byte {
            opcode::JUMP => Self::Jump,
            opcode::JEQ => Self::JumpTrue,
            opcode::JNE => Self::JumpFalse,
            opcode::LBL => Self::Label,
            opcode::CALL => Self::Call,
            opcode::RET => Self::Ret,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub enum Constant {
    Integer(i32),
    Real(f64),
    Str(usize),
    Bool(bool),
}
