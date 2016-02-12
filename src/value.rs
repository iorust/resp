//! RESP Value

use std::vec::Vec;

/// Represents a RESP value
#[derive(Clone, PartialEq)]
pub enum Value<'a> {
    /// Null bulk reply, $-1\r\n
    Null,
    /// Null multi bulk reply, *-1\r\n
    NullMulti,
    /// Status reply, start with '+' 43
    Status(&'a str),
    /// Error reply, start with '-' 45
    Error(&'a str),
    /// Integer reply, start with ':' 58
    Integer(i64),
    /// Bulk reply, start with '$' 36
    Bulk(&'a str),
    /// Buffer bulk reply, start with '$' 36
    BufBulk(&'a [u8]),
    /// multi bulk reply, start with '*' 42
    MultiBulk(Vec<Value<'a>>),
}

trait FillCLRF {
    fn fill_clrf(&mut self);
}

impl FillCLRF for Vec<u8> {
    fn fill_clrf(&mut self) {
        self.push(13);
        self.push(10);
    }
}

impl<'a> Value<'a> {
    /// Returns true if the `Value` is a Null. Returns false otherwise.
    pub fn is_null(&self) -> bool {
        match self {
            &Value::Null => true,
            &Value::NullMulti => true,
            _ => false
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        match self {
            &Value::Null => {
                let mut res: Vec<u8> = Vec::with_capacity(5);
                res.push(36);
                res.push(45);
                res.push(49);
                res.fill_clrf();
                res
            }

            &Value::NullMulti => {
                let mut res: Vec<u8> = Vec::with_capacity(5);
                res.push(42);
                res.push(45);
                res.push(49);
                res.fill_clrf();
                res
            }

            &Value::Status(value) => {
                let mut res: Vec<u8> = Vec::with_capacity(value.len() + 3);
                res.push(43);
                res.extend_from_slice(value.as_bytes());
                res.fill_clrf();
                res
            },

            &Value::Error(value) => {
                let mut res: Vec<u8> = Vec::with_capacity(value.len() + 3);
                res.push(45);
                res.extend_from_slice(value.as_bytes());
                res.fill_clrf();
                res
            },

            &Value::Integer(value) => {
                let value = value.to_string();
                let mut res: Vec<u8> = Vec::with_capacity(value.len() + 3);
                res.push(58);
                res.extend_from_slice(value.as_bytes());
                res.fill_clrf();
                res
            },

            &Value::Bulk(value) => {
                let len = value.len().to_string();
                let mut res: Vec<u8> = Vec::with_capacity(len.len() + value.len() + 5);
                res.push(36);
                res.extend_from_slice(len.as_bytes());
                res.fill_clrf();
                res.extend_from_slice(value.as_bytes());
                res.fill_clrf();
                res
            },

            &Value::BufBulk(buffer) => {
                let len = buffer.len().to_string();
                let mut res: Vec<u8> = Vec::with_capacity(len.len() + buffer.len() + 5);
                res.push(36);
                res.extend_from_slice(len.as_bytes());
                res.fill_clrf();
                res.extend_from_slice(buffer);
                res.fill_clrf();
                res
            },

            &Value::MultiBulk(ref vec) => {
                let len = vec.len().to_string();
                let mut res: Vec<u8> = Vec::new();
                res.push(42);
                res.extend_from_slice(len.as_bytes());
                res.fill_clrf();
                for value in vec.into_iter() {
                    res.append(value.serialize().as_mut());
                }
                res.shrink_to_fit();
                res
            },
        }
    }
}


#[cfg(test)]
#[warn(non_snake_case)]
mod tests {
    use super::*;
    use std::str;

    #[test]
    fn it_value_null() {
        let val = Value::Null;
        assert_eq!(val.is_null(), true);
        assert_eq!(str::from_utf8(&val.serialize()).unwrap(), "$-1\r\n");

        let val = Value::Status("OK");
        assert_eq!(val.is_null(), false);
    }

    #[test]
    fn it_value_nullmulti() {
        let val = Value::NullMulti;
        assert_eq!(val.is_null(), true);
        assert_eq!(str::from_utf8(&val.serialize()).unwrap(), "*-1\r\n");
    }

    #[test]
    fn it_value_status() {
        let val = Value::Status("OK正");
        assert_eq!(str::from_utf8(&val.serialize()).unwrap(), "+OK正\r\n");
    }

    #[test]
    fn it_value_error() {
        let val = Value::Error("error message");
        assert_eq!(str::from_utf8(&val.serialize()).unwrap(), "-error message\r\n");
    }

    #[test]
    fn it_value_integer() {
        let i:i64 = 123456789;
        let val = Value::Integer(i);
        assert_eq!(str::from_utf8(&val.serialize()).unwrap(), ":123456789\r\n");

        let val = Value::Integer(-123456789);
        assert_eq!(str::from_utf8(&val.serialize()).unwrap(), ":-123456789\r\n");
    }

    #[test]
    fn it_value_bulk() {
        let val = Value::Bulk("OK正");
        assert_eq!(str::from_utf8(&val.serialize()).unwrap(), "$5\r\nOK正\r\n");
    }

    #[test]
    fn it_value_bufbulk() {
        let buf: [u8; 2] = [79, 75];
        let val = Value::BufBulk(&buf);
        assert_eq!(str::from_utf8(&val.serialize()).unwrap(), "$2\r\nOK\r\n");
    }

    #[test]
    fn it_value_multibulk() {
        let val = Value::MultiBulk(Vec::new());
        assert_eq!(str::from_utf8(&val.serialize()).unwrap(), "*0\r\n");

        let mut vec: Vec<Value> = Vec::new();
        vec.push(Value::Null);
        vec.push(Value::NullMulti);
        vec.push(Value::Status("OK"));
        vec.push(Value::Error("message"));
        vec.push(Value::Integer(123456789));
        vec.push(Value::Bulk("Hello"));
        let val = Value::MultiBulk(vec);
        assert_eq!(str::from_utf8(&val.serialize()).unwrap(),
            "*6\r\n$-1\r\n*-1\r\n+OK\r\n-message\r\n:123456789\r\n$5\r\nHello\r\n");
    }
}
