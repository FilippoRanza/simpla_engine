use std::fmt;
use std::io::{self, BufRead, Error};
use std::str::FromStr;

#[derive(Debug)]
pub enum ReadError {
    InputOutput(Error),
    IntParseError(String),
    RealParseError(String),
    BoolParseError(String),
    EOF,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputOutput(io_err) => write!(f, "IO Error: {}", io_err),
            Self::IntParseError(err) => write!(f, "{}", parse_error_mgs(err, "integer")),
            Self::RealParseError(err) => write!(f, "{}", parse_error_mgs(err, "real")),
            Self::BoolParseError(err) => write!(f, "{}", parse_error_mgs(err, "boolean")),
            Self::EOF => write!(f, "STDIN reach EOF: no more input available"),
        }
    }
}

fn parse_error_mgs(token: &str, expect: &str) -> String {
    format!(
        "Parse Error: `{}` cannot be converted into type {}",
        token, expect
    )
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
    Parse(&'a str),
    InputOutput(Error),
}

impl<'a> ParseError<'a> {
    fn to_read_error(self, k: Kind) -> ReadError {
        match self {
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
    string_buff: StringBuffer,
}

impl LineReader {
    pub fn new() -> Self {
        Self {
            string_buff: StringBuffer::new(),
        }
    }

    pub fn next_i32(&mut self) -> Result<i32, ReadError> {
        self.next(Kind::Integer)
    }

    pub fn next_f64(&mut self) -> Result<f64, ReadError> {
        self.next(Kind::Real)
    }

    pub fn next_bool(&mut self) -> Result<bool, ReadError> {
        self.next(Kind::Boolean)
    }

    pub fn next_string(&mut self) -> Result<String, ReadError> {
        loop {
            let buff = self.string_buff.get_buffer();
            if let Some(buff) = buff {
                return Ok(buff);
            } else {
                self.string_buff.read_from_stdin()?;
            }
        }
    }

    fn next<T>(&mut self, k: Kind) -> Result<T, ReadError>
    where
        T: FromStr,
    {
        loop {
            let token = self.string_buff.next_token();
            if let Some(token) = token {
                let res = parse_token(token);
                return convert_result(res, k);
            } else {
                self.string_buff.read_from_stdin()?;
            }
        }
    }
}

fn convert_result<'a, T>(res: Result<T, ParseError<'a>>, k: Kind) -> Result<T, ReadError> {
    match res {
        Ok(t) => Ok(t),
        Err(err) => Err(err.to_read_error(k)),
    }
}

fn parse_token<T>(tok: &str) -> Result<T, ParseError>
where
    T: FromStr,
{
    let parse_res = tok.parse();
    match parse_res {
        Ok(v) => Ok(v),
        Err(_) => Err(ParseError::Parse(tok)),
    }
}

struct StringBuffer {
    buff: Option<String>,
    begin: usize,
}

impl StringBuffer {
    #[cfg(test)]
    fn from_string(s: String) -> Self {
        Self {
            buff: Some(s),
            begin: 0,
        }
    }

    fn new() -> Self {
        Self {
            buff: None,
            begin: 0,
        }
    }

    fn read_from_stdin(&mut self) -> Result<(), ReadError> {
        let mut buff = get_line()?;
        buff.pop();
        self.begin = 0;
        self.buff = Some(buff);
        Ok(())
    }

    fn get_buffer(&mut self) -> Option<String> {
        let s = self.buff.take();
        if let Some(s) = s {
            if s.len() == 0 || self.begin == 0{
                Some(s)
            }
            else if self.begin == s.len() {
                None
            } else {
                let tmp = &s[self.begin..];
                Some(tmp.to_owned())
            }
        } else {
            None
        }
    }

    fn next_token(&mut self) -> Option<&str> {
        if let Some(s) = &self.buff {
            let (output, begin) = find_next_token(self.begin, &s)?;
            self.begin = begin;
            Some(output)
        } else {
            None
        }
    }
}

fn find_next_token<'a>(mut begin: usize, s: &'a str) -> Option<(&'a str, usize)> {
    enum TokenState {
        Begin,
        Token,
        End,
    }
    if begin == s.len() {
        None
    } else {
        let mut stat = TokenState::Begin;
        let mut end = begin;
        for (c, i) in s[begin..].chars().zip(begin..) {
            stat = match stat {
                TokenState::Begin => {
                    if c.is_ascii_whitespace() {
                        TokenState::Begin
                    } else {
                        begin = i;
                        TokenState::Token
                    }
                }
                TokenState::Token => {
                    if c.is_ascii_whitespace() {
                        end = i;
                        TokenState::End
                    } else {
                        TokenState::Token
                    }
                }
                TokenState::End => break,
            };
        }

        match stat {
            TokenState::Begin => None,
            TokenState::Token => Some((&s[begin..], s.len())),
            TokenState::End => Some((&s[begin..end], end)),
        }
    }
}

fn get_line() -> Result<String, ReadError> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buff = String::new();
    let count = handle.read_line(&mut buff)?;
    if count == 0 {
        Err(ReadError::EOF)
    } else {
        Ok(buff)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_string_buffer_tokens() {
        let mut buffer = StringBuffer::from_string(" 45 45.67    12.12 test  ".to_owned());
        assert_eq!(buffer.next_token(), Some("45"));
        assert_eq!(buffer.next_token(), Some("45.67"));
        assert_eq!(buffer.next_token(), Some("12.12"));
        assert_eq!(buffer.next_token(), Some("test"));
        assert_eq!(buffer.next_token(), None);

        let mut buffer = StringBuffer::from_string("45 45.67    12.12 test".to_owned());
        assert_eq!(buffer.next_token(), Some("45"));
        assert_eq!(buffer.next_token(), Some("45.67"));
        assert_eq!(buffer.next_token(), Some("12.12"));
        assert_eq!(buffer.next_token(), Some("test"));
        assert_eq!(buffer.next_token(), None);
    }

    #[test]
    fn test_string_buffer_full_string() {
        let mut buffer = StringBuffer::from_string("12 true full string test".to_owned());
        assert_eq!(buffer.next_token(), Some("12"));
        assert_eq!(
            buffer.get_buffer(),
            Some(" true full string test".to_owned())
        );
        assert_eq!(buffer.next_token(), None);
        assert_eq!(buffer.get_buffer(), None);
    }
}
