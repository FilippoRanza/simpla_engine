use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str;

use crate::command_definition::*;
use crate::opcode;
use crate::string_memory::StringMemory;

enum ProgramBuildState {
    Body,
    Function,
}

struct ProgramFactory {
    state: ProgramBuildState,
    body: Vec<Command>,
    func: Vec<Vec<Command>>,
    curr: Vec<Command>,
}

impl ProgramFactory {
    fn new() -> Self {
        Self {
            state: ProgramBuildState::Body,
            body: vec![],
            func: vec![],
            curr: vec![],
        }
    }

    fn switch_function(mut self) -> Self {
        if self.curr.len() > 0 {
            self.func.push(self.curr);
        }
        Self {
            body: self.body,
            func: self.func,
            state: ProgramBuildState::Function,
            curr: vec![],
        }
    }

    fn add_command(&mut self, cmd: Command) {
        match self.state {
            ProgramBuildState::Body => self.body.push(cmd),
            ProgramBuildState::Function => self.curr.push(cmd),
        }
    }

    fn build_program(mut self) -> Program {
        if self.curr.len() > 0 {
            self.func.push(self.curr);
        }

        let functions = self.func.into_iter().map(|blk| Block::new(blk)).collect();
        
        Program {
            body: Block::new(self.body),
            func: functions,
        }
    }
}

#[derive(Debug)]
pub enum LoadError {
    UnknownByte(UnknownByteError),
    MissingBytes(ErrorLocation),
    InputOutputError(std::io::Error),
    StringEncodeError(str::Utf8Error),
    BooleanEncodeError(u8),
}

impl From<std::io::Error> for LoadError {
    fn from(e: std::io::Error) -> Self {
        Self::InputOutputError(e)
    }
}

impl From<str::Utf8Error> for LoadError {
    fn from(e: str::Utf8Error) -> Self {
        Self::StringEncodeError(e)
    }
}

#[derive(Debug)]
pub struct UnknownByteError {
    pub value: u8,
    pub index: usize,
}

impl UnknownByteError {
    fn new(value: u8, index: usize) -> Self {
        Self { value, index }
    }
}

#[derive(Debug)]
pub struct ErrorLocation {
    pub index: usize,
    pub length: usize,
    pub err: ErrorOperation,
}

#[derive(Debug)]
pub enum ErrorOperation {
    LoadingU16,
    LoadingI32,
    LoadingF64,
    LoadingStr,
    LoadingBool,
}

impl ErrorLocation {
    fn new(index: usize, length: usize, err: ErrorOperation) -> Self {
        Self { index, length, err }
    }
}

pub fn load_program(file: &Path) -> Result<(Program, StringMemory), LoadError> {
    let data = load_file(file)?;
    parse_data(&data)
}

fn parse_data(data: &[u8]) -> Result<(Program, StringMemory), LoadError> {
    let mut factory = ProgramFactory::new();
    let mut index = 0;
    let mut string_memory = StringMemory::new();
    while index < data.len() {
        if let Some(cmd) = is_single_command(data[index]) {
            factory.add_command(cmd);
            index += 1;
        } else if let Some((cmd, offset)) = is_address_command(index, &data)? {
            factory.add_command(cmd);
            index += offset;
        } else if let Some((cmd, offset)) = is_constant_command(index, &data, &mut string_memory)? {
            factory.add_command(cmd);
            index += offset;
        } else if data[index] == opcode::FUNC {
            factory = factory.switch_function();
            index += 1;
        } else {
            let err = UnknownByteError::new(data[index], index);
            return Err(LoadError::UnknownByte(err));
        }
    }

    Ok((factory.build_program(), string_memory))
}

fn is_single_command(byte: u8) -> Option<Command> {
    match byte {
        opcode::ADDI..=opcode::AND | opcode::RDI..=opcode::WRLS | opcode::EXT | opcode::PARAM => {
            Some(convert_single(byte))
        }
        _ => None,
    }
}

fn is_address_command(index: usize, buff: &[u8]) -> Result<Option<(Command, usize)>, LoadError> {
    let byte = buff[index];
    let output = match byte {
        opcode::LDI..=opcode::STRS => {
            let k = Kind::new(byte);
            let cmd = if byte < opcode::STRI {
                let addr = get_u16(buff, index + 1)? as usize;
                Command::MemoryLoad(k, addr)
            } else {
                let addr = get_u16(buff, index + 1)? as usize;
                Command::MemoryStore(k, addr)
            };
            Some((cmd, 3))
        }
        opcode::JUMP..=opcode::RET => {
            let cond = ControlFlow::new(byte);
            let (addr, offset) = if byte == opcode::RET {
                (0, 1)
            } else {
                let tmp = get_u16(buff, index + 1)? as usize;
                (tmp, 3)
            };
            Some((Command::Control(cond, addr), offset))
        }
        opcode::STRIP..=opcode::STRSP => {
            let kind = Kind::new(byte);
            let addr = get_u16(buff, index + 1)? as usize;
            let cmd = Command::StoreParam(kind, addr);
            Some((cmd, 3))
        }

        _ => None,
    };
    Ok(output)
}

fn is_constant_command(
    index: usize,
    buff: &[u8],
    str_mem: &mut StringMemory,
) -> Result<Option<(Command, usize)>, LoadError> {
    let byte = buff[index];
    let output = match byte {
        opcode::LDIC..=opcode::LDSC => {
            let (tmp, offset) = convert_constant(index, buff, str_mem)?;
            let out = Command::ConstantLoad(tmp);
            Some((out, offset + 1))
        }
        _ => None,
    };

    Ok(output)
}

fn convert_constant(
    index: usize,
    buff: &[u8],
    str_mem: &mut StringMemory,
) -> Result<(Constant, usize), LoadError> {
    // load and store constant modulo 4 follows
    // the same pattern, check opcode list
    match buff[index] % 4 {
        3 => {
            let int_val = get_i32(buff, index + 1)?;
            Ok((Constant::Integer(int_val), 4))
        }
        0 => {
            let real_val = get_f64(buff, index + 1)?;
            Ok((Constant::Real(real_val), 8))
        }
        1 => {
            let bool_val = get_boolean(buff, index + 1)?;
            Ok((Constant::Bool(bool_val), 1))
        }
        2 => {
            let size = get_u16(buff, index + 1)? as usize;
            let byte_string = take_bytes(buff, index + 3, size)?;
            let tmp_str = str::from_utf8(byte_string)?;
            let string = tmp_str.to_owned();
            let index = str_mem.insert_string(string);
            Ok((Constant::Str(index), size + 2))
        }
        _ => unreachable!(),
    }
}

fn convert_single(byte: u8) -> Command {
    match byte {
        opcode::EXT => Command::Exit,
        opcode::ADDI..=opcode::NEI => Command::Integer(MathOperator::new(byte)),
        opcode::ADDR..=opcode::NER => Command::Real(MathOperator::new(byte - 10)),
        opcode::RDI..=opcode::RDS => Command::Input(Kind::new(byte)),
        opcode::WRI..=opcode::WRS => Command::Output(Kind::new(byte)),
        opcode::WRLI..=opcode::WRLS => Command::OutputLine(Kind::new(byte)),
        opcode::CSTI => Command::CastInt,
        opcode::CSTR => Command::CastReal,
        opcode::OR => Command::Or,
        opcode::AND => Command::And,
        opcode::PARAM => Command::NewRecord,
        _ => unreachable!(),
    }
}

fn load_file(file: &Path) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(file)?;
    let meta = file.metadata()?;
    let mut output = Vec::with_capacity(meta.len() as usize);
    file.read_to_end(&mut output)?;
    Ok(output)
}

fn take_bytes<'a>(buff: &'a [u8], start: usize, len: usize) -> Result<&'a [u8], LoadError> {
    if buff.len() > start + len - 1 {
        let end = start + len;
        let tmp = &buff[start..end];
        Ok(tmp)
    } else {
        let err = ErrorLocation::new(start, len, ErrorOperation::LoadingStr);
        Err(LoadError::MissingBytes(err))
    }
}

fn get_u16(buff: &[u8], index: usize) -> Result<u16, LoadError> {
    if buff.len() > index + 1 {
        let value = [buff[index], buff[index + 1]];
        let output = u16::from_be_bytes(value);
        Ok(output)
    } else {
        let err = ErrorLocation::new(index, 2, ErrorOperation::LoadingU16);
        Err(LoadError::MissingBytes(err))
    }
}

fn get_i32(buff: &[u8], index: usize) -> Result<i32, LoadError> {
    if buff.len() > index + 3 {
        let value = [
            buff[index],
            buff[index + 1],
            buff[index + 2],
            buff[index + 3],
        ];
        let output = i32::from_be_bytes(value);
        Ok(output)
    } else {
        let err = ErrorLocation::new(index, 4, ErrorOperation::LoadingI32);
        Err(LoadError::MissingBytes(err))
    }
}

fn get_f64(buff: &[u8], index: usize) -> Result<f64, LoadError> {
    if buff.len() > index + 7 {
        let value = [
            buff[index],
            buff[index + 1],
            buff[index + 2],
            buff[index + 3],
            buff[index + 4],
            buff[index + 5],
            buff[index + 6],
            buff[index + 7],
        ];
        let output = f64::from_be_bytes(value);
        Ok(output)
    } else {
        let err = ErrorLocation::new(index, 8, ErrorOperation::LoadingF64);
        Err(LoadError::MissingBytes(err))
    }
}

fn get_boolean(buff: &[u8], index: usize) -> Result<bool, LoadError> {
    if buff.len() > index {
        let byte = buff[index];
        match byte {
            255 => Ok(true),
            0 => Ok(false),
            other => {
                let err = LoadError::BooleanEncodeError(other);
                Err(err)
            }
        }
    } else {
        let err = ErrorLocation::new(index, 1, ErrorOperation::LoadingBool);
        Err(LoadError::MissingBytes(err))
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_correct_parse() {
        let simple = vec![opcode::ADDI, opcode::SUBI, opcode::ADDR, opcode::OR];
        parse_data(&simple).unwrap();

        // 5 chars
        let a = 'a' as u8;
        let with_string = vec![opcode::LDSC, 0, 5, a, a, a, a, a];
        let (prog, mem) = parse_data(&with_string).unwrap();
        assert_eq!(prog.body.code.len(), 1);
        assert_eq!(prog.func.len(), 0);

        let cmd = &prog.body.code[0];
        assert!(matches!(cmd, Command::ConstantLoad(ld) if
            matches!(ld, Constant::Str(s) if mem.get_string(*s) == "aaaaa")
        ));
    }

    #[test]
    fn test_wrong_byte() {
        let test_string = "test with lc";
        let len = test_string.len() as u16;
        let test_bytes = test_string.as_bytes();

        let mut data = Vec::new();
        data.push(opcode::LDSC);
        for b in &len.to_be_bytes() {
            data.push(*b)
        }

        for b in test_bytes {
            data.push(*b);
        }

        // 255 is an invalid opcode
        data.push(255);
        let stat = parse_data(&data).unwrap_err();
        match stat {
            LoadError::UnknownByte(err) => {
                assert_eq!(err.value, 255);
            }
            _ => assert!(false, "{:?}", stat),
        }
    }

    #[test]
    fn test_load_f64() {
        let number: f64 = 6.80;
        let bytes = number.to_be_bytes();

        let mut data = vec![opcode::LDRC];
        for b in &bytes {
            data.push(*b);
        }

        let (prog, _) = parse_data(&data).unwrap();
        assert_eq!(prog.body.code.len(), 1);
        assert_eq!(prog.func.len(), 0);

        let cmd = &prog.body.code[0];
        assert!(matches!(cmd, Command::ConstantLoad(ld) if
            matches!(ld, Constant::Real(r) if *r == number)
        ))
    }

    #[test]
    fn test_function_build() {
        let data = [
            opcode::ADDI,
            opcode::GEQI,
            opcode::CALL,
            0,
            1,
            opcode::EXT,
            opcode::FUNC,
            opcode::ADDI,
            opcode::RET,
            opcode::FUNC,
            opcode::GEQR,
            opcode::RET,
        ];

        let (prog, _) = parse_data(&data).unwrap();
        assert_eq!(prog.body.code.len(), 4);
        assert_eq!(prog.func.len(), 2, "{:?}", prog.func);
        for func in &prog.func {
            assert_eq!(func.code.len(), 2);
        }
    }
}
