use std::collections::VecDeque;
use std::io::{self, Error, BufRead};
use std::str::FromStr;
use std::fmt;

#[derive(Debug)]
pub enum ReadError {
    InputOutput(Error),
    IntParseError(String),
    RealParseError(String),
    BoolParseError(String),
    MissingInteger,
    MissingReal,
    MissingBoolean,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputOutput(io_err) => write!(f, "IO Error: {}", io_err),
            Self::IntParseError(err) => write!(f, "{}", parse_error_mgs(err, "integer")),
            Self::RealParseError(err) => write!(f, "{}", parse_error_mgs(err, "real")),
            Self::BoolParseError(err) => write!(f, "{}", parse_error_mgs(err, "boolean")),
            Self::MissingBoolean => write!(f, "{}", missing_error_msg("boolean")),
            Self::MissingInteger => write!(f, "{}", missing_error_msg("integer")),
            Self::MissingReal => write!(f, "{}", missing_error_msg("real")),
        }
    }
}

fn missing_error_msg(expect: &str) -> String {
    format!("EOF Error: {} was expectd", expect)
}

fn parse_error_mgs(token: &str, expect: &str) -> String {
    format!("Parse Error: `{}` cannot be converted into type {}", token, expect)
}

impl From<Error> for ReadError {
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
    fn to_read_error(self, k: Kind) -> ReadError {
        match self {
            Self::Missing => match k {
                Kind::Integer => ReadError::MissingInteger,
                Kind::Real => ReadError::MissingReal,
                Kind::Boolean => ReadError::MissingBoolean,
            },
            Self::Parse(s) => match k {
                Kind::Integer => ReadError::IntParseError(s.to_owned()),
                Kind::Real => ReadError::RealParseError(s.to_owned()),
                Kind::Boolean => ReadError::BoolParseError(s.to_owned()),
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

fn convert_result<'a, T>(res: Result<T, ParseError<'a>>, k: Kind) -> Result<T, ReadError> {
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
