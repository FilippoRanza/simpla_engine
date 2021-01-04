use crate::opcode;
use std::collections::HashMap;

pub type AddrSize = u16;

#[derive(Debug)]
pub struct Program {
    pub body: Block,
    pub func: Vec<Block>,
}

#[derive(Debug)]
pub struct Block {
    pub code: Vec<Command>,
    pub labels: HashMap<usize, usize>,
}

#[derive(Debug)]
pub struct ProgramMemory {
    pub main: MemorySize,
    pub func: Vec<MemorySize>
}


#[derive(Debug, std::default::Default)]
pub struct MemorySize {
    pub integer_count: usize,
    pub real_count: usize,
    pub boolean_count: usize,
    pub string_count: usize,
}

impl Block {
    pub fn new(code: Vec<Command>) -> Self {
        let labels = Self::build_labels(&code);
        Self { code, labels }
    }

    fn build_labels(code: &[Command]) -> HashMap<usize, usize> {
        code.iter()
            .enumerate()
            .filter_map(|(addr, cmd)| match cmd {
                Command::Control(ControlFlow::Label, label) => Some((*label, addr)),
                _ => None,
            })
            .collect()
    }
}

#[derive(Debug)]
pub enum Command {
    Integer(Operator),
    Real(Operator),
    CastInt,
    CastReal,
    MemoryLoad(Kind, AddrSize),
    MemoryStore(Kind, AddrSize),
    Control(ControlFlow, usize),
    Input(Kind),
    Output(Kind),
    Flush(FlushMode),
    ForControl(ForControl),
    Exit,
    ConstantLoad(Constant),
    StoreParam(Kind, AddrSize),
    NewRecord(usize),
    Unary(Kind),
    StrCompare(RelationalOperator),
    BoolCompare(RelationalOperator),
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
pub enum Operator {
    Math(MathOperator),
    Rel(RelationalOperator),
}

impl Operator {
    pub fn new(b: u8) -> Self {
        match b {
            0..=3 => Self::Math(MathOperator::new(b)),
            4..=9 => Self::Rel(RelationalOperator::new(b)),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub enum RelationalOperator {
    GreatEq,
    Greater,
    LessEq,
    Less,
    Equal,
    NotEqual,
}

impl RelationalOperator {
    pub fn new(b: u8) -> Self {
        match b {
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
pub enum MathOperator {
    Add,
    Sub,
    Mul,
    Div,
}

impl MathOperator {
    pub fn new(b: u8) -> Self {
        match b {
            0 => Self::Add,
            1 => Self::Sub,
            2 => Self::Mul,
            3 => Self::Div,
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

#[derive(Debug)]
pub enum FlushMode {
    Flush,
    NewLine,
}

#[derive(Debug)]
pub enum ForControl {
    New,
    End,
    Check,
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_label_translation() {
        // just some random code
        let code = [
            Command::Real(Operator::Math(MathOperator::Add)),
            Command::Control(ControlFlow::Jump, 0),
            Command::Real(Operator::Math(MathOperator::Add)),
            Command::Control(ControlFlow::Label, 1),
            Command::Real(Operator::Math(MathOperator::Add)),
            Command::Control(ControlFlow::JumpFalse, 1),
            Command::Real(Operator::Math(MathOperator::Add)),
            Command::Control(ControlFlow::Label, 0),
            Command::Exit,
        ];

        let results: &[(usize, usize)] = &[(0, 7), (1, 3)];

        let mapping = Block::build_labels(&code);
        assert_eq!(mapping.len(), 2);
        for (lbl, index) in results {
            assert_eq!(mapping.get(lbl).unwrap(), index);
        }
    }
}
