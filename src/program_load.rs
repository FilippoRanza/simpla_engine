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
    main_mem: Option<MemorySize>,
    func_mem: Vec<MemorySize>,
}

impl ProgramFactory {
    fn new() -> Self {
        Self {
            state: ProgramBuildState::Body,
            body: vec![],
            func: vec![],
            curr: vec![],
            main_mem: None,
            func_mem: vec![],
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
            main_mem: self.main_mem,
            func_mem: self.func_mem,
        }
    }

    fn add_command(&mut self, cmd: Command) {
        match self.state {
            ProgramBuildState::Body => self.body.push(cmd),
            ProgramBuildState::Function => self.curr.push(cmd),
        }
    }

    fn add_memory_size(
        &mut self,
        int_count: AddrSize,
        real_count: AddrSize,
        boolean_count: AddrSize,
        string_count: AddrSize,
    ) {
        let mem_size = MemorySize {
            integer_count: int_count as usize,
            real_count: real_count as usize,
            boolean_count: boolean_count as usize,
            string_count: string_count as usize,
        };
        match self.state {
            ProgramBuildState::Body => self.main_mem = Some(mem_size),
            ProgramBuildState::Function => self.func_mem.push(mem_size),
        }
    }

    fn build_program(mut self) -> (Program, ProgramMemory) {
        if self.curr.len() > 0 {
            self.func.push(self.curr);
        }

        let functions = self.func.into_iter().map(|blk| Block::new(blk)).collect();

        let prog = Program {
            body: Block::new(self.body),
            func: functions,
        };

        let mem = ProgramMemory {
            main: self.main_mem.unwrap(),
            func: self.func_mem
        };

        (prog, mem)
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

pub fn load_program(file: &Path) -> Result<(Program, ProgramMemory, StringMemory), LoadError> {
    let data = load_file(file)?;
    parse_data(&data)
}

fn parse_data(data: &[u8]) -> Result<(Program, ProgramMemory, StringMemory), LoadError> {
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
        } else if data[index] == opcode::INIT {
            let (int_count, real_count, bool_count, str_count) =
                get_memory_command(index + 1, data)?;
            factory.add_memory_size(int_count, real_count, bool_count, str_count);
            index += 9;
        } else {
            let err = UnknownByteError::new(data[index], index);
            return Err(LoadError::UnknownByte(err));
        }
    }

    let (prog, mem) = factory.build_program();
    Ok((prog, mem, string_memory))
}

fn get_memory_command(
    index: usize,
    buff: &[u8],
) -> Result<(AddrSize, AddrSize, AddrSize, AddrSize), LoadError> {
    Ok((
        get_u16(buff, index)?,
        get_u16(buff, index + 2)?,
        get_u16(buff, index + 4)?,
        get_u16(buff, index + 6)?,
    ))
}

fn is_single_command(byte: u8) -> Option<Command> {
    match byte {
        opcode::ADDI..=opcode::AND
        | opcode::RDI..=opcode::WRS
        | opcode::FLN
        | opcode::FLU
        | opcode::EXT
        | opcode::BFOR..=opcode::NOT
        | opcode::GEQS..=opcode::NEB => Some(convert_single(byte)),
        _ => None,
    }
}

fn is_address_command(index: usize, buff: &[u8]) -> Result<Option<(Command, usize)>, LoadError> {
    let byte = buff[index];
    let output = match byte {
        opcode::LDI..=opcode::STRS => {
            let k = Kind::new(byte);
            let cmd = if byte < opcode::STRI {
                let addr = get_u16(buff, index + 1)?;
                Command::MemoryLoad(k, addr)
            } else {
                let addr = get_u16(buff, index + 1)?;
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
            let addr = get_u16(buff, index + 1)?;
            let cmd = Command::StoreParam(kind, addr);
            Some((cmd, 3))
        }
        opcode::PARAM => {
            let tmp = get_u16(buff, index + 1)? as usize;
            Some((Command::NewRecord(tmp), 3))
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
            let index = str_mem.insert_static_string(string);
            Ok((Constant::Str(index), size + 2))
        }
        _ => unreachable!(),
    }
}

fn convert_single(byte: u8) -> Command {
    match byte {
        opcode::EXT => Command::Exit,
        opcode::ADDI..=opcode::NEI => Command::Integer(Operator::new(byte)),
        opcode::ADDR..=opcode::NER => Command::Real(Operator::new(byte - 10)),
        opcode::RDI..=opcode::RDS => Command::Input(Kind::new(byte)),
        opcode::WRI..=opcode::WRS => Command::Output(Kind::new(byte)),
        opcode::FLU => Command::Flush(FlushMode::Flush),
        opcode::FLN => Command::Flush(FlushMode::NewLine),
        opcode::CSTI => Command::CastInt,
        opcode::CSTR => Command::CastReal,
        opcode::OR => Command::Or,
        opcode::AND => Command::And,
        opcode::BFOR => Command::ForControl(ForControl::New),
        opcode::CFOR => Command::ForControl(ForControl::Check),
        opcode::EFOR => Command::ForControl(ForControl::End),
        opcode::NEGI => Command::Unary(Kind::Integer),
        opcode::NEGR => Command::Unary(Kind::Real),
        opcode::NOT => Command::Unary(Kind::Bool),
        opcode::GEQS..=opcode::NES => Command::StrCompare(RelationalOperator::new(byte - 63)),
        opcode::GEQB..=opcode::NEB => Command::BoolCompare(RelationalOperator::new(byte - 69)),
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
    


    fn add_init_header(mut code: Vec<u8>) -> Vec<u8> {
        let mut init_header: Vec<u8> = (0..9).map(|_| 0).collect();
        init_header[0] = opcode::INIT;
        init_header.append(&mut code);
        init_header
    }

    #[test]
    fn test_correct_parse() {
        
        let simple = add_init_header(vec![opcode::ADDI, opcode::SUBI, opcode::ADDR, opcode::OR]);
        parse_data(&simple).unwrap();

        // 5 chars
        let a = 'a' as u8;
        let with_string = add_init_header(vec![opcode::LDSC, 0, 5, a, a, a, a, a]);
        let (prog, _, mem) = parse_data(&with_string).unwrap();
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

        let mut data = add_init_header(vec![opcode::LDRC]);
        for b in &bytes {
            data.push(*b);
        }

        let (prog, _, _) = parse_data(&data).unwrap();
        assert_eq!(prog.body.code.len(), 1);
        assert_eq!(prog.func.len(), 0);

        let cmd = &prog.body.code[0];
        assert!(matches!(cmd, Command::ConstantLoad(ld) if
            matches!(ld, Constant::Real(r) if *r == number)
        ))
    }

    #[test]
    fn test_function_build() {
        let data = vec![
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
        let data = add_init_header(data);
        let (prog, _, _) = parse_data(&data).unwrap();
        assert_eq!(prog.body.code.len(), 4);
        assert_eq!(prog.func.len(), 2, "{:?}", prog.func);
        for func in &prog.func {
            assert_eq!(func.code.len(), 2);
        }
    }


}
