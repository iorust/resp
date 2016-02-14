//! RESP Value

use std::str;
use std::vec::Vec;
use std::string::String;
use std::io::{Error, ErrorKind};

pub use super::value::{ Value, RESP_MAX };

pub fn encode(value: &Value) -> Vec<u8> {
    match value {
        &Value::Null => {
            let mut res: Vec<u8> = Vec::with_capacity(5);
            res.push(36);
            res.push(45);
            res.push(49);
            res.fill_clrf();
            res
        }

        &Value::NullArray => {
            let mut res: Vec<u8> = Vec::with_capacity(5);
            res.push(42);
            res.push(45);
            res.push(49);
            res.fill_clrf();
            res
        }

        &Value::String(ref value) => {
            let mut res: Vec<u8> = Vec::with_capacity(value.len() + 3);
            res.push(43);
            res.extend(value.as_bytes());
            res.fill_clrf();
            res
        }

        &Value::Error(ref value) => {
            let mut res: Vec<u8> = Vec::with_capacity(value.len() + 3);
            res.push(45);
            res.extend(value.as_bytes());
            res.fill_clrf();
            res
        }

        &Value::Integer(ref value) => {
            let value = value.to_string();
            let mut res: Vec<u8> = Vec::with_capacity(value.len() + 3);
            res.push(58);
            res.extend(value.as_bytes());
            res.fill_clrf();
            res
        }

        &Value::Bulk(ref value) => {
            let len = value.len().to_string();
            let mut res: Vec<u8> = Vec::with_capacity(len.len() + value.len() + 5);
            res.push(36);
            res.extend(len.as_bytes());
            res.fill_clrf();
            res.extend(value.as_bytes());
            res.fill_clrf();
            res
        }

        &Value::BufBulk(ref buffer) => {
            let len = buffer.len().to_string();
            let mut res: Vec<u8> = Vec::with_capacity(len.len() + buffer.len() + 5);
            res.push(36);
            res.extend(len.as_bytes());
            res.fill_clrf();
            res.extend(buffer);
            res.fill_clrf();
            res
        }

        &Value::Array(ref vec) => {
            let len = vec.len().to_string();
            let mut res: Vec<u8> = Vec::new();
            res.push(42);
            res.extend(len.as_bytes());
            res.fill_clrf();
            res.extend(vec.iter().flat_map(|value| value.encode()));
            res.shrink_to_fit();
            res
        }
    }
}

pub fn encode_slice(slice: &[&str]) -> Vec<u8> {
    let array: Vec<Value> = slice.iter().map(|string| Value::Bulk(string.to_string())).collect();
    Value::Array(array).encode()
}

#[derive(Debug)]
pub struct Decoder {
    buf_bulk: bool,
    pos: usize,
    buf: Vec<u8>,
    res: Vec<Value>,
}

impl Decoder {
    pub fn new() -> Self {
        Decoder {
            pos: 0,
            buf_bulk: false,
            buf: Vec::new(),
            res: Vec::with_capacity(8),
        }
    }

    pub fn with_buf_bulk() -> Self {
        Decoder {
            pos: 0,
            buf_bulk: true,
            buf: Vec::new(),
            res: Vec::with_capacity(8),
        }
    }

    pub fn feed(&mut self, buf: &Vec<u8>) -> Result<(), Error> {
        self.buf.extend(buf);
        self.parse()
    }

    pub fn read(&mut self) -> Option<Value> {
        if self.res.len() == 0 {
            return None;
        }
        Some(self.res.remove(0))
    }

    pub fn buffer_len(&self) -> usize {
        self.buf.len()
    }

    pub fn result_len(&self) -> usize {
        self.res.len()
    }

    fn prune_buf(&mut self, pos: usize) {
        if pos == 0 || pos >= self.buf.len() {
            self.buf.clear();
        } else {
            let mut count = pos;
            while count > 0 {
                count -= 1;
                self.buf.remove(0);
            }
        }
    }

    fn parse(&mut self) -> Result<(), Error> {
        match parse_one_value(&self.buf, self.pos, self.buf_bulk) {
            Some(ParseResult::Res(value, pos)) => {
                self.res.push(value);
                self.prune_buf(pos);
                self.pos = 0;
                self.parse()
            }
            Some(ParseResult::Err(message)) => {
                self.prune_buf(0);
                Err(Error::new(ErrorKind::InvalidData, message))
            }
            None => Ok(())
        }
    }
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

fn parse_string(bytes: &[u8]) -> Result<String, String> {
    if let Ok(string) = str::from_utf8(bytes) {
        return Ok(string.to_string());
    }
    Err("parse string failed".to_string())
}

fn parse_integer(bytes: &[u8]) -> Result<i64, String> {
    if let Ok(string) = str::from_utf8(bytes) {
        if let Ok(int) = string.parse::<i64>() {
            return Ok(int);
        }
    }
    Err("parse integer failed".to_string())
}

fn is_crlf(a: u8, b: u8) -> bool {
    a == 13 && b == 10
}

fn read_crlf(buffer: &Vec<u8>, start: usize) -> Option<usize> {
    for pos in start..(buffer.len() - 1) {
        if is_crlf(buffer[pos], buffer[pos + 1]) {
            return Some(pos)
        }
    }
    None
}

enum ParseResult {
    // parse success
    Res(Value, usize),
    // parse failed
    Err(String),
}

fn parse_one_value(buffer: &Vec<u8>, offset: usize, buf_bulk: bool) -> Option<ParseResult> {
    // Exclude first byte, and two "CLRF" bytes.
    // Means that buf is too short, wait more.
    let buf_len = buffer.len();
    let prev_offset = offset;
    if offset + 3 > buf_len {
        return None;
    }

    let mut offset = prev_offset + 1;

    match buffer[prev_offset] {
        /// Value::String
        43 => {
            if let Some(pos) = read_crlf(buffer, offset) {
                let bytes = buffer[offset..pos].as_ref();
                offset = pos + 2;
                match parse_string(bytes) {
                    Ok(string) => Some(ParseResult::Res(Value::String(string), offset)),
                    Err(_) => Some(ParseResult::Err("Parse '+' failed".to_string())),
                }
            } else {
                None
            }
        }

        /// Value::Error
        45 => {
            if let Some(pos) = read_crlf(buffer, offset) {
                let bytes = buffer[offset..pos].as_ref();
                offset = pos + 2;
                match parse_string(bytes) {
                    Ok(string) => Some(ParseResult::Res(Value::Error(string), offset)),
                    Err(_) => Some(ParseResult::Err("Parse '-' failed".to_string())),
                }
            } else {
                None
            }
        }

        /// Value::Integer
        58 => {
            if let Some(pos) = read_crlf(buffer, offset) {
                let bytes = buffer[offset..pos].as_ref();
                offset = pos + 2;
                match parse_integer(bytes) {
                    Ok(int) => Some(ParseResult::Res(Value::Integer(int), offset)),
                    Err(_) => Some(ParseResult::Err("Parse ':' failed".to_string())),
                }
            } else {
                None
            }
        }

        /// Value::Bulk
        36 => {
            if let Some(pos) = read_crlf(buffer, offset) {
                let bytes = buffer[offset..pos].as_ref();
                offset = pos + 2;
                match parse_integer(bytes) {
                    Ok(int) => {
                        if int == -1 {
                            // Null bulk
                            return Some(ParseResult::Res(Value::Null, offset));
                        }

                        if int < -1 || int >= RESP_MAX {
                            return Some(ParseResult::Err("Parse '$' failed".to_string()));
                        }

                        let int = int as usize;
                        let end = int + offset;
                        if end + 1 >= buf_len {
                            return None;
                        }

                        if !is_crlf(buffer[end], buffer[end + 1]) {
                            return Some(ParseResult::Err("Parse '$' failed".to_string()));
                        }

                        let bytes = buffer[offset..end].as_ref();
                        offset = end + 2;

                        if buf_bulk {
                            let mut buf: Vec<u8> = Vec::with_capacity(bytes.len());
                            buf.extend(bytes);
                            return Some(ParseResult::Res(Value::BufBulk(buf), offset));
                        }

                        match parse_string(bytes) {
                            Ok(string) => Some(ParseResult::Res(Value::Bulk(string), offset)),
                            Err(_) => Some(ParseResult::Err("Parse '$' failed".to_string())),
                        }
                    }

                    Err(_) => Some(ParseResult::Err("Parse '$' failed".to_string())),
                }
            } else {
                None
            }
        }

        /// Value::Array
        42 => {
            if let Some(pos) = read_crlf(buffer, offset) {
                let bytes = buffer[offset..pos].as_ref();
                offset = pos + 2;
                match parse_integer(bytes) {
                    Ok(int) => {
                        if int == -1 {
                            // Null array
                            return Some(ParseResult::Res(Value::NullArray, offset));
                        }

                        if int < -1 || int >= RESP_MAX {
                            return Some(ParseResult::Err("Parse '*' failed".to_string()));
                        }

                        let mut array: Vec<Value> = Vec::with_capacity(int as usize);
                        for _ in 0..int {
                            match parse_one_value(buffer, offset, buf_bulk) {
                                Some(ParseResult::Res(value, pos)) => {
                                    array.push(value);
                                    offset = pos;
                                }
                                Some(ParseResult::Err(message)) => {
                                    return Some(ParseResult::Err(message));
                                }
                                None => {
                                    return None;
                                }
                            }
                        }

                        Some(ParseResult::Res(Value::Array(array), offset))
                    }
                    Err(_) => Some(ParseResult::Err("Parse '*' failed".to_string())),
                }
            } else {
                None
            }
        }

        _ => Some(ParseResult::Err("Invalid Chunk: parse failed".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fn_encode_slice() {
        let array = ["SET", "a", "1"];
        assert_eq!(String::from_utf8(encode_slice(&array)).unwrap(),
            "*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n");

        let array = vec!["SET", "a", "1"];
        assert_eq!(String::from_utf8(encode_slice(&array)).unwrap(),
            "*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n");
    }

    #[test]
    fn struct_decoder() {
        let mut decoder = Decoder::new();
        assert_eq!(decoder.buffer_len(), 0);
        assert_eq!(decoder.result_len(), 0);

        let buf = Value::Null.encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.buffer_len(), 0);
        assert_eq!(decoder.result_len(), 1);
        assert_eq!(decoder.read().unwrap(), Value::Null);
        assert_eq!(decoder.result_len(), 0);
        assert_eq!(decoder.read(), None);

        let buf = Value::NullArray.encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::NullArray);
        assert_eq!(decoder.read(), None);

        let buf = Value::String("OK".to_string()).encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::String("OK".to_string()));
        assert_eq!(decoder.read(), None);

        let buf = Value::Error("message".to_string()).encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::Error("message".to_string()));
        assert_eq!(decoder.read(), None);

        let buf = Value::Integer(123456789).encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::Integer(123456789));
        assert_eq!(decoder.read(), None);

        let buf = Value::Bulk("Hello".to_string()).encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::Bulk("Hello".to_string()));
        assert_eq!(decoder.read(), None);

        let buf = Value::BufBulk("Hello".to_string().into_bytes()).encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::Bulk("Hello".to_string()));
        assert_eq!(decoder.read(), None);

        let array = vec!["SET", "a", "1"];
        let buf = encode_slice(&array);
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::Array(vec![
            Value::Bulk("SET".to_string()),
            Value::Bulk("a".to_string()),
            Value::Bulk("1".to_string())
        ]));
        assert_eq!(decoder.read(), None);
    }

    #[test]
    fn struct_decoder_with_buf_bulk() {
        let mut decoder = Decoder::with_buf_bulk();

        let buf = Value::Null.encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::Null);
        assert_eq!(decoder.read(), None);

        let buf = Value::NullArray.encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::NullArray);
        assert_eq!(decoder.read(), None);

        let buf = Value::String("OK".to_string()).encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::String("OK".to_string()));
        assert_eq!(decoder.read(), None);

        let buf = Value::Error("message".to_string()).encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::Error("message".to_string()));
        assert_eq!(decoder.read(), None);

        let buf = Value::Integer(123456789).encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::Integer(123456789));
        assert_eq!(decoder.read(), None);

        let buf = Value::Bulk("Hello".to_string()).encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::BufBulk("Hello".to_string().into_bytes()));
        assert_eq!(decoder.read(), None);

        let buf = Value::BufBulk("Hello".to_string().into_bytes()).encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::BufBulk("Hello".to_string().into_bytes()));
        assert_eq!(decoder.read(), None);

        let array = vec!["SET", "a", "1"];
        let buf = encode_slice(&array);
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::Array(vec![
            Value::BufBulk("SET".to_string().into_bytes()),
            Value::BufBulk("a".to_string().into_bytes()),
            Value::BufBulk("1".to_string().into_bytes())
        ]));
        assert_eq!(decoder.read(), None);
    }

    #[test]
    fn struct_decoder_feed_error() {
        let mut decoder = Decoder::new();

        let buf = Value::String("OK正".to_string()).encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::String("OK正".to_string()));
        assert_eq!(decoder.read(), None);
        let mut buf = Value::String("OK正".to_string()).encode();
        // [43, 79, 75, 230, 173, 163, 13, 10]
        buf.remove(5);
        assert_eq!(decoder.feed(&buf).is_err(), true);
        assert_eq!(decoder.buffer_len(), 0);
        assert_eq!(decoder.result_len(), 0);

        let buf = "$\r\n".to_string().into_bytes();
        assert_eq!(decoder.feed(&buf).is_err(), true);

        let buf = "$-2\r\n".to_string().into_bytes();
        assert_eq!(decoder.feed(&buf).is_err(), true);

        let buf = "&-1\r\n".to_string().into_bytes();
        assert_eq!(decoder.feed(&buf).is_err(), true);

        let buf = "$-1\r\n".to_string().into_bytes();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::Null);

        let buf = "$0\r\n\r\n".to_string().into_bytes();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::Bulk("".to_string()));

    }

    #[test]
    fn struct_decoder_continuingly() {
        let mut decoder = Decoder::new();

        let buf = "$0\r\n".to_string().into_bytes();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read(), None);
        let buf = "\r\n".to_string().into_bytes();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::Bulk("".to_string()));

        let _values = vec![
            Value::Null,
            Value::NullArray,
            Value::String("abcdefg".to_string()),
            Value::Error("abcdefg".to_string()),
            Value::Integer(123456789),
            Value::Bulk("abcdefg".to_string())
        ];
        let mut values = _values.clone();
        values.push(Value::Array(_values));
        let buf: Vec<u8> = values.iter().flat_map(|value| value.encode()).collect();
        let mut read_values: Vec<Value> = Vec::new();

        // feed byte by byte~
        for byte in buf.iter() {
            let byte = vec![*byte];
            assert_eq!(decoder.feed(&byte).unwrap(), ());
            if decoder.result_len() > 0 {
                // one value should be parsed.
                assert_eq!(decoder.result_len(), 1);
                // buffer should be clear.
                assert_eq!(decoder.buffer_len(), 0);
                read_values.push(decoder.read().unwrap());
                assert_eq!(decoder.result_len(), 0);
            } else {
                assert_eq!(decoder.buffer_len() > 0, true);
                assert_eq!(decoder.result_len(), 0);
            }
        }
        assert_eq!(&read_values, &values);
    }
}
