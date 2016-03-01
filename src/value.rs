//! RESP Value

use std::vec::Vec;
use std::string::String;
use std::io::{Result, Error, ErrorKind};
use super::serialize::{encode};

/// Represents a RESP value
/// http://redis.io/topics/protocol

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

    pub fn to_beautify_string(&self) -> String {
        match self {
            &Value::Null => format!("{}", "(Null)"),
            &Value::NullArray => format!("{}", "(Null Array)"),
            &Value::String(ref val) => format!("{}", val),
            &Value::Error(ref val) => format!("(Error) {}", val),
            &Value::Integer(ref val) => format!("(Integer) {}", val.to_string()),
            &Value::Bulk(ref val) => format!("\"{}\"", val),
            &Value::BufBulk(ref val) => {
                if val.len() == 0 {
                    return format!("{}", "(Empty Buffer)");
                }
                let mut string = String::with_capacity(52);
                for u in val.iter().take(16) {
                    string.push_str(&format_to_hex_str(*u));
                }
                if val.len() > 16 {
                    string.push_str(" ...");
                }
                format!("(Buffer) {}", &string[1..])
            }
            &Value::Array(ref val) => format!("{}", format_array_to_str(val, 0)),
        }
    }
}

fn format_to_hex_str(u: u8) -> String {
    if u >= 16 {
        format!(" {:x}", u)
    } else {
        format!(" 0{:x}", u)
    }
}

fn format_index_str(index: usize, num_len: usize) -> String {
    let mut string = index.to_string();
    while string.len() < num_len {
        string.insert(0, ' ');
    }
    format!("{}) ", string)
}

fn format_array_to_str(array: &Vec<Value>, min_index_len: usize) -> String {
    if array.len() == 0 {
        return format!("{}", "(Empty Array)");
    }

    let mut string = String::new();
    let mut index_len = min_index_len;
    let len = array.len();
    let num_len = len.to_string().len();
    if num_len > index_len {
        index_len = num_len;
    }
    for (i, value) in array.iter().enumerate() {
        // first element don't need indent.
        let num_len = if i == 0 { index_len - min_index_len } else { index_len };
        string.push_str(&format_index_str(i + 1, num_len));
        match value {
            &Value::Array(ref sub) => string.push_str(&format_array_to_str(sub, index_len + 3)),
            _ => string.push_str(&value.to_beautify_string()),
        };
        if i + 1 < len {
            string.push('\n');
        }
    }
    string
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
    fn enum_to_beautify_string() {
        assert_eq!(Value::Null.to_beautify_string(), "(Null)");
        assert_eq!(Value::NullArray.to_beautify_string(), "(Null Array)");
        assert_eq!(Value::String("OK".to_string()).to_beautify_string(), "OK");
        assert_eq!(Value::Error("Err".to_string()).to_beautify_string(), "(Error) Err");
        assert_eq!(Value::Integer(123).to_beautify_string(), "(Integer) 123");
        assert_eq!(Value::Bulk("Bulk String".to_string()).to_beautify_string(), "\"Bulk String\"");
        assert_eq!(Value::BufBulk(vec![]).to_beautify_string(), "(Empty Buffer)");
        assert_eq!(Value::BufBulk(vec![0, 100]).to_beautify_string(), "(Buffer) 00 64");
        assert_eq!(Value::BufBulk(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            17, 18]).to_beautify_string(),
            "(Buffer) 00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f ...");
        assert_eq!(Value::Array(vec![]).to_beautify_string(), "(Empty Array)");
        assert_eq!(Value::Array(vec![Value::Null, Value::Integer(123)]).to_beautify_string(),
            "1) (Null)\n2) (Integer) 123");

        let _values = vec![
            Value::Null,
            Value::NullArray,
            Value::String("OK".to_string()),
            Value::Error("Err".to_string()),
            Value::Integer(123),
            Value::Bulk("Bulk String".to_string()),
            Value::Array(vec![]),
            Value::BufBulk(vec![0, 100]),
            Value::Array(vec![Value::Array(vec![]), Value::Integer(123), Value::Bulk("Bulk String".to_string())])
        ];
        let mut values = _values.clone();
        values.push(Value::Array(_values));
        values.push(Value::Null);
        let mut _values = values.clone();
        _values.push(Value::Array(values));
        _values.push(Value::Null);

let enum_fmt_result = " 1) (Null)
 2) (Null Array)
 3) OK
 4) (Error) Err
 5) (Integer) 123
 6) \"Bulk String\"
 7) (Empty Array)
 8) (Buffer) 00 64
 9) 1) (Empty Array)
    2) (Integer) 123
    3) \"Bulk String\"
10) 1) (Null)
    2) (Null Array)
    3) OK
    4) (Error) Err
    5) (Integer) 123
    6) \"Bulk String\"
    7) (Empty Array)
    8) (Buffer) 00 64
    9) 1) (Empty Array)
       2) (Integer) 123
       3) \"Bulk String\"
11) (Null)
12) 1) (Null)
    2) (Null Array)
    3) OK
    4) (Error) Err
    5) (Integer) 123
    6) \"Bulk String\"
    7) (Empty Array)
    8) (Buffer) 00 64
    9) 1) (Empty Array)
       2) (Integer) 123
       3) \"Bulk String\"
   10) 1) (Null)
       2) (Null Array)
       3) OK
       4) (Error) Err
       5) (Integer) 123
       6) \"Bulk String\"
       7) (Empty Array)
       8) (Buffer) 00 64
       9) 1) (Empty Array)
          2) (Integer) 123
          3) \"Bulk String\"
   11) (Null)
13) (Null)";

        assert_eq!(Value::Array(_values).to_beautify_string(), enum_fmt_result);
        // println!("{}", Value::Array(_values).to_beautify_string());
    }
}
