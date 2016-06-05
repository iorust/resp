//! RESP Errors

use std::error;
use std::result;
use std::fmt;
use std::io;
use std::string::{String, ToString, FromUtf8Error};

/// The errors that can arise while parsing a RESP stream.
#[derive(Clone, PartialEq)]
pub enum ErrorCode {
    /// Invalid RESP simple string
    InvalidString,
    /// Invalid RESP error
    InvalidError,
    /// Invalid RESP integer
    InvalidInteger,
    /// Invalid RESP bulk string
    InvalidBulk,
    /// Invalid RESP array
    InvalidArray,
    /// Invalid RESP prefix
    InvalidPrefix(u8),
}

impl ErrorCode {
    fn as_str(&self) -> &'static str {
        match *self {
            ErrorCode::InvalidString => "Parse '+' failed",
            ErrorCode::InvalidError => "Parse '-' failed",
            ErrorCode::InvalidInteger => "Parse ':' failed",
            ErrorCode::InvalidBulk => "Parse '$' failed",
            ErrorCode::InvalidArray => "Parse '*' failed",
            ErrorCode::InvalidPrefix(_) => "Invalid prefix",
        }
    }
}

impl ToString for ErrorCode {
    fn to_string(&self) -> String {
        String::from(self.as_str())
    }
}

impl fmt::Debug for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ErrorCode::InvalidPrefix(ref prefix) => write!(f, "Invalid prefix: {:x}", prefix),
            _ => self.to_string().fmt(f)
        }
    }
}

/// This type represents all possible errors with RESP.
#[derive(Debug)]
pub enum Error {
    /// The RESP data had some protocol error.
    Protocol(ErrorCode),

    /// Some IO error occurred.
    Io(io::Error),

    /// Some UTF8 error occurred.
    FromUtf8(FromUtf8Error),
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Protocol(ref code) => code.as_str(),
            Error::Io(ref error) => error::Error::description(error),
            Error::FromUtf8(ref error) => error.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref error) => Some(error),
            Error::FromUtf8(ref error) => Some(error),
            _ => None,
        }
    }

}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Protocol(ref code) => fmt::Debug::fmt(code, fmt),
            Error::Io(ref error) => fmt::Display::fmt(error, fmt),
            Error::FromUtf8(ref error) => fmt::Display::fmt(error, fmt),
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::Io(error)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(error: FromUtf8Error) -> Error {
        Error::FromUtf8(error)
    }
}

/// Helper alias for `Result` objects that return a RESP `Error`.
pub type Result<T> = result::Result<T, Error>;
