//! RESP Value

use std::vec::Vec;
use std::string::String;
use std::io::{Error, ErrorKind};
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

    pub fn to_encoded_string(&self) -> Result<String, Error> {
        let bytes = self.encode();
        String::from_utf8(bytes).map_err(|err| Error::new(ErrorKind::InvalidData, err))
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
}
