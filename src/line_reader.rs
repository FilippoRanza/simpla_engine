use std::collections::VecDeque;
use std::io::{self, Error, BufRead};
use std::str::FromStr;

#[derive(Debug)]
pub enum ReadError<'a> {
    InputOutput(Error),
    IntParseError(&'a str),
    RealParseError(&'a str),
    BoolParseError(&'a str),
    MissingInteger,
    MissingReal,
    MissingBoolean,
}

impl<'a> From<Error> for ReadError<'a> {
    fn from(e: Error) -> Self {
        Self::InputOutput(e)
    }
}

enum Kind {
    Integer,
    Real,
    Boolean,
}

enum ParseError<'a> {
    Missing,
    Parse(&'a str),
    InputOutput(Error),
}

impl<'a> ParseError<'a> {
    fn to_read_error(self, k: Kind) -> ReadError<'a> {
        match self {
            Self::Missing => match k {
                Kind::Integer => ReadError::MissingInteger,
                Kind::Real => ReadError::MissingReal,
                Kind::Boolean => ReadError::MissingBoolean,
            },
            Self::Parse(s) => match k {
                Kind::Integer => ReadError::IntParseError(s),
                Kind::Real => ReadError::RealParseError(s),
                Kind::Boolean => ReadError::BoolParseError(s),
            },
            Self::InputOutput(io) => ReadError::InputOutput(io),
        }
    }
}

impl<'a> From<Error> for ParseError<'a> {
    fn from(e: Error) -> Self {
        Self::InputOutput(e)
    }
}

pub struct LineReader {
    buff: String,
    int_buff: VecDeque<i32>,
    real_buff: VecDeque<f64>,
    bool_buff: VecDeque<bool>,
    auto_clean: bool,
}

impl LineReader {
    pub fn new(auto_clean: bool) -> Self {
        Self {
            buff: String::new(),
            int_buff: VecDeque::new(),
            real_buff: VecDeque::new(),
            bool_buff: VecDeque::new(),
            auto_clean,
        }
    }

    pub fn next_i32(&mut self) -> Result<i32, ReadError> {
        if self.auto_clean {
            self.real_buff.clear();
            self.bool_buff.clear();
        }
        let res = next_parsed_token(&mut self.buff, &mut self.int_buff);
        convert_result(res, Kind::Integer)
    }

    pub fn next_f64(&mut self) -> Result<f64, ReadError> {
        if self.auto_clean {
            self.int_buff.clear();
            self.bool_buff.clear();
        }
        let res = next_parsed_token(&mut self.buff, &mut self.real_buff);
        convert_result(res, Kind::Real)
    }

    pub fn next_bool(&mut self) -> Result<bool, ReadError> {
        if self.auto_clean {
            self.real_buff.clear();
            self.int_buff.clear();
        }
        let res = next_parsed_token(&mut self.buff, &mut self.bool_buff);
        convert_result(res, Kind::Boolean)
    }

    pub fn next_string(&mut self) -> Result<String, ReadError> {
        fill_buffer(&mut self.buff)?;
        let out = self.buff.clone();
        Ok(out)
    }
}

fn convert_result<'a, T>(res: Result<T, ParseError<'a>>, k: Kind) -> Result<T, ReadError<'a>> {
    match res {
        Ok(t) => Ok(t),
        Err(err) => Err(err.to_read_error(k)),
    }
}

fn next_parsed_token<'a, T>(
    buff: &'a mut String,
    store: &'a mut VecDeque<T>,
) -> Result<T, ParseError<'a>>
where
    T: FromStr,
{
    if store.is_empty() {
        fill_buffer(buff)?;
        for token in buff.split_ascii_whitespace() {
            let parse_res = token.parse();
            match parse_res {
                Ok(val) => store.push_back(val),
                Err(_) => return Err(ParseError::Parse(token)),
            }
        }
    }

    if let Some(val) = store.pop_front() {
        Ok(val)
    } else {
        Err(ParseError::Missing)
    }
}

fn fill_buffer(buff: &mut String) -> std::io::Result<()> {
    buff.clear();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    handle.read_line(buff)?;
    buff.pop();
    Ok(())
}
