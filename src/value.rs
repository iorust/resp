//! RESP Value

use std::fmt;
use std::vec::Vec;
use std::string::String;
use std::io::{Result, Error, ErrorKind};
use super::serialize::{encode};

/// Represents a RESP value
/// http://redis.io/topics/protocol

/// up to 512 MB in length
pub const RESP_MAX: i64 = 512 * 1024 * 1024;

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Value {
    /// Null bulk reply, $-1\r\n
    Null,
    /// Null array reply, *-1\r\n
    NullArray,
    /// For Simple Strings the first byte of the reply is "+"[43]
    String(String),
    /// For Errors the first byte of the reply is "-"[45]
    Error(String),
    /// For Integers the first byte of the reply is ":"[58]
    Integer(i64),
    /// For Bulk Strings the first byte of the reply is "$"[36]
    Bulk(String),
    /// For Bulk <binary> Strings the first byte of the reply is "$"[36]
    BufBulk(Vec<u8>),
    /// For Arrays the first byte of the reply is "*"[42]
    Array(Vec<Value>),
}

impl Value {
    /// Returns true if the `Value` is a Null. Returns false otherwise.
    pub fn is_null(&self) -> bool {
        match self {
            &Value::Null => true,
            &Value::NullArray => true,
            _ => false
        }
    }

    /// Returns true if the `Value` is a Error. Returns false otherwise.
    pub fn is_error(&self) -> bool {
        match self {
            &Value::Error(_) => true,
            _ => false
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        encode(self)
    }

    pub fn to_encoded_string(&self) -> Result<String> {
        let bytes = self.encode();
        String::from_utf8(bytes).map_err(|err| Error::new(ErrorKind::InvalidData, err))
    }
}

fn format_to_hex_str(u: u8) -> String {
    if u >= 16 {
        format!(" {:x}", u)
    } else {
        format!(" 0{:x}", u)
    }
}

fn format_array_to_str(array: &Vec<Value>, index: usize, depth: usize) -> String {
    let mut prefix = String::with_capacity(depth * 2);
    for _ in 0..depth {
        prefix.push_str("  ");
    }
    let prefix = &prefix;

    let mut string = String::new();
    if index > 0 {
        string.push_str(&prefix[2..]);
        string.push_str(&(index + 1).to_string());
        string.push_str(")\n");
    }
    for (i, value) in array.iter().enumerate() {
        match value {
            &Value::Array(ref sub) => string.push_str(&format_array_to_str(sub, i, depth + 1)),
            _ => {
                string.push_str(prefix);
                string.push_str(&(i + 1).to_string());
                string.push_str(") ");
                string.push_str(&value.to_string());
                string.push_str("\n");
            }
        };
    }
    string
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Value::Null => write!(f, "{}", "(Null)"),
            &Value::NullArray => write!(f, "{}", "(Null Array)"),
            &Value::String(ref string) => write!(f, "\"{}\"", string),
            &Value::Error(ref error) => write!(f, "\"{}\"", error),
            &Value::Integer(int) => write!(f, "{}", int.to_string()),
            &Value::Bulk(ref string) => write!(f, "\"{}\"", string),
            &Value::BufBulk(ref buf) => {
                let mut string = String::with_capacity(62);
                string.push_str("<Buffer");
                for u in buf.iter().take(16) {
                    string.push_str(&format_to_hex_str(*u));
                }
                if buf.len() > 16 {
                    string.push_str(" ... >");
                } else {
                    string.push_str(">");
                }
                write!(f, "{}", string)
            }
            &Value::Array(ref array) => write!(f, "{}", format_array_to_str(array, 0, 0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_is_null() {
        assert_eq!(Value::Null.is_null(), true);
        assert_eq!(Value::NullArray.is_null(), true);
        assert_eq!(Value::String("OK".to_string()).is_null(), false);
        assert_eq!(Value::Error("Err".to_string()).is_null(), false);
        assert_eq!(Value::Integer(123).is_null(), false);
        assert_eq!(Value::Bulk("Bulk".to_string()).is_null(), false);
        assert_eq!(Value::BufBulk(vec![79, 75]).is_null(), false);
        assert_eq!(Value::Array(vec![Value::Null, Value::Integer(123)]).is_null(), false);
    }

    #[test]
    fn enum_is_error() {
        assert_eq!(Value::Null.is_error(), false);
        assert_eq!(Value::NullArray.is_error(), false);
        assert_eq!(Value::String("OK".to_string()).is_error(), false);
        assert_eq!(Value::Error("".to_string()).is_error(), true);
        assert_eq!(Value::Error("Err".to_string()).is_error(), true);
        assert_eq!(Value::Integer(123).is_error(), false);
        assert_eq!(Value::Bulk("Bulk".to_string()).is_error(), false);
        assert_eq!(Value::BufBulk(vec![79, 75]).is_error(), false);
        assert_eq!(Value::Array(vec![Value::Null, Value::Integer(123)]).is_error(), false);
    }

    #[test]
    fn enum_encode_null() {
        let val = Value::Null;
        assert_eq!(val.to_encoded_string().unwrap(), "$-1\r\n");
    }

    #[test]
    fn enum_encode_nullarray() {
        let val = Value::NullArray;
        assert_eq!(val.to_encoded_string().unwrap(), "*-1\r\n");
    }

    #[test]
    fn enum_encode_string() {
        let val = Value::String("OK正".to_string());
        assert_eq!(val.to_encoded_string().unwrap(), "+OK正\r\n");
    }

    #[test]
    fn enum_encode_error() {
        let val = Value::Error("error message".to_string());
        assert_eq!(val.to_encoded_string().unwrap(), "-error message\r\n");
    }

    #[test]
    fn enum_encode_integer() {
        let val = Value::Integer(123456789);
        assert_eq!(val.to_encoded_string().unwrap(), ":123456789\r\n");

        let val = Value::Integer(-123456789);
        assert_eq!(val.to_encoded_string().unwrap(), ":-123456789\r\n");
    }

    #[test]
    fn enum_encode_bulk() {
        let val = Value::Bulk("OK正".to_string());
        assert_eq!(val.to_encoded_string().unwrap(), "$5\r\nOK正\r\n");
    }

    #[test]
    fn enum_encode_bufbulk() {
        let val = Value::BufBulk(vec![79, 75]);
        assert_eq!(val.to_encoded_string().unwrap(), "$2\r\nOK\r\n");
    }

    #[test]
    fn enum_encode_array() {
        let val = Value::Array(Vec::new());
        assert_eq!(val.to_encoded_string().unwrap(), "*0\r\n");

        let mut vec: Vec<Value> = Vec::new();
        vec.push(Value::Null);
        vec.push(Value::NullArray);
        vec.push(Value::String("OK".to_string()));
        vec.push(Value::Error("message".to_string()));
        vec.push(Value::Integer(123456789));
        vec.push(Value::Bulk("Hello".to_string()));
        vec.push(Value::BufBulk(vec![79, 75]));
        let val = Value::Array(vec);
        assert_eq!(val.to_encoded_string().unwrap(),
            "*7\r\n$-1\r\n*-1\r\n+OK\r\n-message\r\n:123456789\r\n$5\r\nHello\r\n$2\r\nOK\r\n");
    }

    #[test]
    fn enum_fmt() {
        assert_eq!(Value::Null.to_string(), "(Null)");
        assert_eq!(Value::NullArray.to_string(), "(Null Array)");
        assert_eq!(Value::String("OK".to_string()).to_string(), "\"OK\"");
        assert_eq!(Value::Error("Err".to_string()).to_string(), "\"Err\"");
        assert_eq!(Value::Integer(123).to_string(), "123");
        assert_eq!(Value::Bulk("Bulk String".to_string()).to_string(), "\"Bulk String\"");
        assert_eq!(Value::BufBulk(vec![0, 100]).to_string(), "<Buffer 00 64>");
        assert_eq!(Value::BufBulk(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18]).to_string(),
            "<Buffer 00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f ... >");
        assert_eq!(Value::Array(vec![Value::Null, Value::Integer(123)]).to_string(), "1) (Null)\n2) 123\n");

        let _values = vec![
            Value::Null,
            Value::NullArray,
            Value::String("OK".to_string()),
            Value::Error("Err".to_string()),
            Value::Integer(123),
            Value::Bulk("Bulk String".to_string()),
            Value::BufBulk(vec![0, 100]),
            Value::Array(vec![Value::Null, Value::Integer(123)])
        ];
        let mut values = _values.clone();
        values.push(Value::Array(_values));
        values.push(Value::Null);
        let mut _values = values.clone();
        _values.push(Value::Array(values));
        _values.push(Value::Null);

let enum_fmt_result = "1) (Null)
2) (Null Array)
3) \"OK\"
4) \"Err\"
5) 123
6) \"Bulk String\"
7) <Buffer 00 64>
8)
  1) (Null)
  2) 123
9)
  1) (Null)
  2) (Null Array)
  3) \"OK\"
  4) \"Err\"
  5) 123
  6) \"Bulk String\"
  7) <Buffer 00 64>
  8)
    1) (Null)
    2) 123
10) (Null)
11)
  1) (Null)
  2) (Null Array)
  3) \"OK\"
  4) \"Err\"
  5) 123
  6) \"Bulk String\"
  7) <Buffer 00 64>
  8)
    1) (Null)
    2) 123
  9)
    1) (Null)
    2) (Null Array)
    3) \"OK\"
    4) \"Err\"
    5) 123
    6) \"Bulk String\"
    7) <Buffer 00 64>
    8)
      1) (Null)
      2) 123
  10) (Null)
12) (Null)
";
        assert_eq!(Value::Array(_values).to_string(), enum_fmt_result);
    }
}
