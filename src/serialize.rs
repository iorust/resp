//! RESP serialize

use std::str;
use std::vec::Vec;
use std::string::String;
use std::io::{Result, Error, ErrorKind};

use super::Value;

/// up to 512 MB in length
const RESP_MAX_SIZE: i64 = 512 * 1024 * 1024;
const CLRF_BYTES: &'static [u8] = b"\r\n";
const NULL_BYTES: &'static [u8] = b"$-1\r\n";
const NULL_ARRAY_BYTES: &'static [u8] = b"*-1\r\n";

/// Encode the value to RESP binary buffer.
/// # Examples
/// ```
/// # use self::resp::{Value, encode};
/// let val = Value::String("OK正".to_string());
/// assert_eq!(encode(&val), vec![43, 79, 75, 230, 173, 163, 13, 10]);
/// ```
pub fn encode(value: &Value) -> Vec<u8> {
    let mut res: Vec<u8> = Vec::new();
    buf_encode(value, &mut res);
    res
}

/// Encode a array of slice string to RESP binary buffer.
/// It is usefull for redis client to encode request command.
/// # Examples
/// ```
/// # use self::resp::encode_slice;
/// let array = ["SET", "a", "1"];
/// assert_eq!(encode_slice(&array),
///            "*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n".to_string().into_bytes());
/// ```
pub fn encode_slice(slice: &[&str]) -> Vec<u8> {
    let array: Vec<Value> = slice.iter().map(|string| Value::Bulk(string.to_string())).collect();
    let mut res: Vec<u8> = Vec::new();
    buf_encode(&Value::Array(array), &mut res);
    res
}

fn buf_encode(value: &Value, buf: &mut Vec<u8>) {
    match *value {
        Value::Null => {
            buf.extend_from_slice(NULL_BYTES);
        }
        Value::NullArray => {
            buf.extend_from_slice(NULL_ARRAY_BYTES);
        }
        Value::String(ref val) => {
            buf.push(b'+');
            buf.extend_from_slice(val.as_bytes());
            buf.extend_from_slice(CLRF_BYTES);
        }
        Value::Error(ref val) => {
            buf.push(b'-');
            buf.extend_from_slice(val.as_bytes());
            buf.extend_from_slice(CLRF_BYTES);
        }
        Value::Integer(ref val) => {
            buf.push(b':');
            buf.extend_from_slice(val.to_string().as_bytes());
            buf.extend_from_slice(CLRF_BYTES);
        }
        Value::Bulk(ref val) => {
            buf.push(b'$');
            buf.extend_from_slice(val.len().to_string().as_bytes());
            buf.extend_from_slice(CLRF_BYTES);
            buf.extend_from_slice(val.as_bytes());
            buf.extend_from_slice(CLRF_BYTES);
        }
        Value::BufBulk(ref val) => {
            buf.push(b'$');
            buf.extend_from_slice(val.len().to_string().as_bytes());
            buf.extend_from_slice(CLRF_BYTES);
            buf.extend_from_slice(val);
            buf.extend_from_slice(CLRF_BYTES);
        }
        Value::Array(ref val) => {
            buf.push(b'*');
            buf.extend_from_slice(val.len().to_string().as_bytes());
            buf.extend_from_slice(CLRF_BYTES);
            for item in val {
                buf_encode(item, buf);
            }
        }
    }
}

/// A streaming RESP decoder.
#[derive(Debug)]
pub struct Decoder {
    buf_bulk: bool,
    pos: usize,
    buf: Vec<u8>,
    res: Vec<Value>,
}

impl Decoder {
    /// Creates a new decoder instance for decoding the RESP buffers.
    /// # Examples
    /// ```
    /// # use self::resp::{Decoder, Value};
    /// let mut decoder = Decoder::new();
    ///
    /// let value = Value::Bulk("Hello".to_string());
    /// assert_eq!(decoder.feed(&value.encode()).unwrap(), ());
    /// assert_eq!(decoder.read().unwrap(), value);
    /// assert_eq!(decoder.read(), None);

    /// let value = Value::BufBulk("Hello".to_string().into_bytes());
    /// assert_eq!(decoder.feed(&value.encode()).unwrap(), ());
    ///
    /// // Always decode "$" buffers to Value::Bulk even if feed Value::BufBulk buffers
    /// assert_eq!(decoder.read().unwrap(), Value::Bulk("Hello".to_string()));
    /// assert_eq!(decoder.read(), None);
    /// ```
    pub fn new() -> Self {
        Decoder {
            buf_bulk: false,
            pos: 0,
            buf: Vec::new(),
            res: Vec::with_capacity(8),
        }
    }

    /// Creates a new decoder instance for decoding the RESP buffers. The instance will decode
    /// bulk value to buffer bulk.
    /// # Examples
    /// ```
    /// # use self::resp::{Decoder, Value};
    /// let mut decoder = Decoder::with_buf_bulk();
    ///
    /// let value = Value::Bulk("Hello".to_string());
    /// assert_eq!(decoder.feed(&value.encode()).unwrap(), ());
    ///
    /// // Always decode "$" buffers to Value::BufBulk even if feed Value::Bulk buffers
    /// assert_eq!(decoder.read().unwrap(), Value::BufBulk("Hello".to_string().into_bytes()));
    /// assert_eq!(decoder.read(), None);

    /// let value = Value::BufBulk("Hello".to_string().into_bytes());
    /// assert_eq!(decoder.feed(&value.encode()).unwrap(), ());
    /// assert_eq!(decoder.read().unwrap(), value);
    /// assert_eq!(decoder.read(), None);
    /// ```
    pub fn with_buf_bulk() -> Self {
        Decoder {
            buf_bulk: true,
            pos: 0,
            buf: Vec::new(),
            res: Vec::with_capacity(8),
        }
    }

    /// Feeds buffers to decoder. The buffer may contain one more values, or be a part of value.
    /// You can feed buffer at all times.
    /// # Examples
    /// ```
    /// # use self::resp::{Decoder, Value};
    /// let mut decoder = Decoder::new();
    /// assert_eq!(decoder.buffer_len(), 0);
    ///
    /// let value = Value::Bulk("Test".to_string());
    /// let buf = value.encode();
    /// assert_eq!(decoder.feed(&buf[0..4]).unwrap(), ());
    /// assert_eq!(decoder.read(), None);
    /// assert_eq!(decoder.buffer_len(), 4);
    /// assert_eq!(decoder.result_len(), 0);
    ///
    /// assert_eq!(decoder.feed(&buf[4..]).unwrap(), ());
    /// assert_eq!(decoder.buffer_len(), 0);
    /// assert_eq!(decoder.result_len(), 1);
    /// assert_eq!(decoder.read().unwrap(), value);
    /// assert_eq!(decoder.read(), None);
    /// assert_eq!(decoder.buffer_len(), 0);
    /// assert_eq!(decoder.result_len(), 0);
    /// ```
    pub fn feed(&mut self, buf: &[u8]) -> Result<()> {
        self.buf.extend(buf);
        self.parse()
    }

    /// Reads a decoded value, will return `None` if no value decoded.
    pub fn read(&mut self) -> Option<Value> {
        if self.res.len() == 0 {
            return None;
        }
        Some(self.res.remove(0))
    }

    /// Returns the buffer's length that wait for decoding. It usually is `0`. Non-zero means that
    /// decoder need more buffer.
    pub fn buffer_len(&self) -> usize {
        self.buf.len()
    }

    /// Returns decoded values count. The decoded values will be hold by decoder, until you read
    /// them.
    pub fn result_len(&self) -> usize {
        self.res.len()
    }

    fn prune_buf(&mut self) {
        if self.pos == self.buf.len() {
            self.pos = 0;
            self.buf.clear();
        }
    }

    fn parse(&mut self) -> Result<()> {
        match parse_one_value(&self.buf[self.pos..], 0, self.buf_bulk) {
            Some(ParseResult::Res(value, pos)) => {
                self.res.push(value);
                self.pos += pos;
                self.prune_buf();
                self.parse()
            }
            Some(ParseResult::Err(message)) => {
                self.pos = self.buf.len();
                self.prune_buf();
                Err(Error::new(ErrorKind::InvalidData, message.to_string()))
            }
            None => Ok(()),
        }
    }
}

fn parse_string(bytes: &[u8]) -> Result<String> {
    if let Ok(string) = str::from_utf8(bytes) {
        return Ok(string.to_string());
    }
    Err(Error::new(ErrorKind::InvalidData, "parse string failed"))
}

fn parse_integer(bytes: &[u8]) -> Result<i64> {
    if let Ok(string) = str::from_utf8(bytes) {
        if let Ok(int) = string.parse::<i64>() {
            return Ok(int);
        }
    }
    Err(Error::new(ErrorKind::InvalidData, "parse integer failed"))
}

fn is_crlf(a: u8, b: u8) -> bool {
    a == b'\r' && b == b'\n'
}

fn read_crlf(buffer: &[u8], start: usize) -> Option<usize> {
    for pos in start..(buffer.len() - 1) {
        if is_crlf(buffer[pos], buffer[pos + 1]) {
            return Some(pos);
        }
    }
    None
}

enum ErrorMessage {
    ParseString,
    ParseError,
    ParseInteger,
    ParseBulk,
    ParseArray,
    ParseInvalid,
}

impl ErrorMessage {
    fn to_string(self) -> &'static str {
        match self {
            ErrorMessage::ParseString => "Parse '+' failed",
            ErrorMessage::ParseError => "Parse '-' failed",
            ErrorMessage::ParseInteger => "Parse ':' failed",
            ErrorMessage::ParseBulk => "Parse '$' failed",
            ErrorMessage::ParseArray => "Parse '*' failed",
            ErrorMessage::ParseInvalid => "Invalid Chunk: parse failed",
        }
    }
}

enum ParseResult {
    // parse success
    Res(Value, usize),
    // parse failed
    Err(ErrorMessage),
}

fn parse_one_value(buffer: &[u8], offset: usize, buf_bulk: bool) -> Option<ParseResult> {
    // Exclude first byte, and two "CLRF" bytes.
    // Means that buf is too short, wait more.
    let buf_len = buffer.len();
    if offset + 3 > buf_len {
        return None;
    }

    let identifier = buffer[offset];
    let mut offset = offset + 1;
    match identifier {
        // Value::String
        b'+' => {
            if let Some(pos) = read_crlf(buffer, offset) {
                let bytes = buffer[offset..pos].as_ref();
                offset = pos + 2;
                match parse_string(bytes) {
                    Ok(string) => Some(ParseResult::Res(Value::String(string), offset)),
                    Err(_) => Some(ParseResult::Err(ErrorMessage::ParseString)),
                }
            } else {
                None
            }
        }
        // Value::Error
        b'-' => {
            if let Some(pos) = read_crlf(buffer, offset) {
                let bytes = buffer[offset..pos].as_ref();
                offset = pos + 2;
                match parse_string(bytes) {
                    Ok(string) => Some(ParseResult::Res(Value::Error(string), offset)),
                    Err(_) => Some(ParseResult::Err(ErrorMessage::ParseError)),
                }
            } else {
                None
            }
        }
        // Value::Integer
        b':' => {
            if let Some(pos) = read_crlf(buffer, offset) {
                let bytes = buffer[offset..pos].as_ref();
                offset = pos + 2;
                match parse_integer(bytes) {
                    Ok(int) => Some(ParseResult::Res(Value::Integer(int), offset)),
                    Err(_) => Some(ParseResult::Err(ErrorMessage::ParseInteger)),
                }
            } else {
                None
            }
        }
        // Value::Bulk
        b'$' => {
            if let Some(pos) = read_crlf(buffer, offset) {
                let bytes = buffer[offset..pos].as_ref();
                offset = pos + 2;
                match parse_integer(bytes) {
                    Ok(int) => {
                        if int == -1 {
                            // Null bulk
                            return Some(ParseResult::Res(Value::Null, offset));
                        }
                        if int < -1 || int >= RESP_MAX_SIZE {
                            return Some(ParseResult::Err(ErrorMessage::ParseBulk));
                        }

                        let int = int as usize;
                        let end = int + offset;
                        if end + 1 >= buf_len {
                            return None;
                        }
                        if !is_crlf(buffer[end], buffer[end + 1]) {
                            return Some(ParseResult::Err(ErrorMessage::ParseBulk));
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
                            Err(_) => Some(ParseResult::Err(ErrorMessage::ParseBulk)),
                        }
                    }
                    Err(_) => Some(ParseResult::Err(ErrorMessage::ParseBulk)),
                }
            } else {
                None
            }
        }
        // Value::Array
        b'*' => {
            if let Some(pos) = read_crlf(buffer, offset) {
                let bytes = buffer[offset..pos].as_ref();
                offset = pos + 2;
                match parse_integer(bytes) {
                    Ok(int) => {
                        if int == -1 {
                            // Null array
                            return Some(ParseResult::Res(Value::NullArray, offset));
                        }
                        if int < -1 || int >= RESP_MAX_SIZE {
                            return Some(ParseResult::Err(ErrorMessage::ParseArray));
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
                    Err(_) => Some(ParseResult::Err(ErrorMessage::ParseArray)),
                }
            } else {
                None
            }
        }
        _ => Some(ParseResult::Err(ErrorMessage::ParseInvalid)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::Value;

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
        assert_eq!(decoder.read().unwrap(),
                   Value::Array(vec![Value::Bulk("SET".to_string()),
                                     Value::Bulk("a".to_string()),
                                     Value::Bulk("1".to_string())]));
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
        assert_eq!(decoder.read().unwrap(),
                   Value::BufBulk("Hello".to_string().into_bytes()));
        assert_eq!(decoder.read(), None);

        let buf = Value::BufBulk("Hello".to_string().into_bytes()).encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(),
                   Value::BufBulk("Hello".to_string().into_bytes()));
        assert_eq!(decoder.read(), None);

        let array = vec!["SET", "a", "1"];
        let buf = encode_slice(&array);
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(),
                   Value::Array(vec![Value::BufBulk("SET".to_string().into_bytes()),
                                     Value::BufBulk("a".to_string().into_bytes()),
                                     Value::BufBulk("1".to_string().into_bytes())]));
        assert_eq!(decoder.read(), None);
    }

    #[test]
    fn struct_decoder_feed_error() {
        let mut decoder = Decoder::new();

        assert_eq!(decoder.feed(&[]).unwrap(), ());
        assert_eq!(decoder.read(), None);

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

        // feed a available data after error
        let buf = Value::Null.encode();
        assert_eq!(decoder.feed(&buf).unwrap(), ());
        assert_eq!(decoder.read().unwrap(), Value::Null);

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

        let _values = vec![Value::Null,
                           Value::NullArray,
                           Value::String("abcdefg".to_string()),
                           Value::Error("abcdefg".to_string()),
                           Value::Integer(123456789),
                           Value::Bulk("abcdefg".to_string())];
        let mut values = _values.clone();
        values.push(Value::Array(_values));
        let buf: Vec<u8> = values.iter().flat_map(|value| value.encode()).collect();
        let mut read_values: Vec<Value> = Vec::new();

        // feed byte by byte~
        for byte in buf {
            let byte = vec![byte];
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
