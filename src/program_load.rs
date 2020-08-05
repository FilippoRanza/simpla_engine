use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str;

use crate::command_definition::*;
use crate::opcode;

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

pub struct UnknownByteError {
    pub value: u8,
    pub index: usize
}

impl UnknownByteError {
    fn new(value: u8, index: usize) -> Self {
        Self { 
            value,
            index
        }
    }
}

pub struct ErrorLocation {
    pub index: usize,
    pub length: usize,
}

impl ErrorLocation {
    fn new(index: usize, length: usize) -> Self {
        Self { index, length }
    }
}

pub fn load_program(file: &Path) -> Result<Vec<Command>, LoadError> {
    let data = load_file(file)?;
    let mut output = Vec::new();

    let mut index = 0;
    while index < data.len() {
        if let Some(cmd) = is_single_command(data[index]) {
            output.push(cmd);
            index += 1;
        } else if let Some(cmd) = is_address_command(index, &data)? {
            output.push(cmd);
            index += 3;
        } else if let Some((cmd, offset)) = is_constant_command(index, &data)? {
            output.push(cmd);
            index += offset;
        } else {
            let err = UnknownByteError::new(data[index], index);
            return Err(LoadError::UnknownByte(err))
        }
    }

    Ok(output)
}

fn is_single_command(byte: u8) -> Option<Command> {
    match byte {
        opcode::ADDI..=opcode::AND | opcode::RDI..=opcode::WRS | opcode::EXT => {
            Some(convert_single(byte))
        }
        _ => None,
    }
}

fn is_address_command(index: usize, buff: &[u8]) -> Result<Option<Command>, LoadError> {
    let byte = buff[index];
    let addr = get_u16(buff, index + 1)? as usize;
    let output = match byte {
        opcode::LDI..=opcode::STRS => {
            let k = Kind::new(byte);
            let cmd = if byte < opcode::STRI {
                Command::MemoryLoad(k, addr)
            } else {
                Command::MemoryStore(k, addr)
            };
            Some(cmd)
        }
        opcode::JUMP..=opcode::RET => {
            let cond = ControlFlow::new(byte);
            Some(Command::Control(cond, addr))
        }
        _ => None,
    };
    Ok(output)
}

fn is_constant_command(index: usize, buff: &[u8]) -> Result<Option<(Command, usize)>, LoadError> {
    let byte = buff[index];
    let output = match byte {
        opcode::LDIC..=opcode::LDSC => {
            let (tmp, offset) = convert_constant(index, buff)?;
            let out = Command::ConstantLoad(tmp);
            Some((out, offset + 1))
        }
        opcode::STRIC..=opcode::STRSC => {
            let (tmp, offset) = convert_constant(index, buff)?;
            let out = Command::ConstantStore(tmp);
            Some((out, offset + 1))
        }
        _ => None,
    };

    Ok(output)
}

fn convert_constant(index: usize, buff: &[u8]) -> Result<(Constant, usize), LoadError> {
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
            Ok((Constant::Str(string), size + 2))
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
        opcode::CSTI => Command::CastInt,
        opcode::CSTR => Command::CastReal,
        opcode::OR => Command::Or,
        opcode::AND => Command::And,
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
    if buff.len() > start + len {
        let end = start + len;
        let tmp = &buff[start..end];
        Ok(tmp)
    } else {
        let err = ErrorLocation::new(start, len);
        Err(LoadError::MissingBytes(err))
    }
}

fn get_u16(buff: &[u8], index: usize) -> Result<u16, LoadError> {
    if buff.len() > index + 2 {
        let value = [buff[index], buff[index + 1]];
        let output = u16::from_be_bytes(value);
        Ok(output)
    } else {
        let err = ErrorLocation::new(index, 2);
        Err(LoadError::MissingBytes(err))
    }
}

fn get_i32(buff: &[u8], index: usize) -> Result<i32, LoadError> {
    if buff.len() > index + 4 {
        let value = [
            buff[index],
            buff[index + 1],
            buff[index + 2],
            buff[index + 3],
        ];
        let output = i32::from_be_bytes(value);
        Ok(output)
    } else {
        let err = ErrorLocation::new(index, 4);
        Err(LoadError::MissingBytes(err))
    }
}

fn get_f64(buff: &[u8], index: usize) -> Result<f64, LoadError> {
    if buff.len() > index + 8 {
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
        let err = ErrorLocation::new(index, 8);
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
        let err = ErrorLocation::new(index, 1);
        Err(LoadError::MissingBytes(err))
    }
}
