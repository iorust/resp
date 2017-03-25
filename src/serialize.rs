//! RESP serialize

use std::vec::Vec;
use std::string::String;
use std::io::{Read, BufRead, BufReader};
use super::error::{Result, Error, ErrorCode};

use super::Value;

/// up to 512 MB in length
const RESP_MAX_SIZE: i64 = 512 * 1024 * 1024;
const CRLF_BYTES: &'static [u8] = b"\r\n";
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
            buf.extend_from_slice(CRLF_BYTES);
        }
        Value::Error(ref val) => {
            buf.push(b'-');
            buf.extend_from_slice(val.as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
        }
        Value::Integer(ref val) => {
            buf.push(b':');
            buf.extend_from_slice(val.to_string().as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
        }
        Value::Bulk(ref val) => {
            buf.push(b'$');
            buf.extend_from_slice(val.len().to_string().as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
            buf.extend_from_slice(val.as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
        }
        Value::BufBulk(ref val) => {
            buf.push(b'$');
            buf.extend_from_slice(val.len().to_string().as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
            buf.extend_from_slice(val);
            buf.extend_from_slice(CRLF_BYTES);
        }
        Value::Array(ref val) => {
            buf.push(b'*');
            buf.extend_from_slice(val.len().to_string().as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
            for item in val {
                buf_encode(item, buf);
            }
        }
    }
}

/// A streaming RESP Decoder.
#[derive(Debug)]
pub struct Decoder<R> {
    buf_bulk: bool,
    reader: BufReader<R>,
}

impl<R: Read> Decoder<R> {
    /// Creates a new Decoder instance for decoding the RESP buffers.
    /// # Examples
    /// ```
    /// # use std::io::BufReader;
    /// # use self::resp::{Decoder, Value};
    ///
    /// let value = Value::Bulk("Hello".to_string());
    /// let buf = value.encode();
    /// let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
    /// assert_eq!(decoder.decode().unwrap(), Value::Bulk("Hello".to_string()));
    /// ```
    pub fn new(reader: BufReader<R>) -> Self {
        Decoder {
            buf_bulk: false,
            reader: reader,
        }
    }

    /// Creates a new Decoder instance for decoding the RESP buffers. The instance will decode
    /// bulk value to buffer bulk.
    /// # Examples
    /// ```
    /// # use std::io::BufReader;
    /// # use self::resp::{Decoder, Value};
    ///
    /// let value = Value::Bulk("Hello".to_string());
    /// let buf = value.encode();
    /// let mut decoder = Decoder::with_buf_bulk(BufReader::new(buf.as_slice()));
    /// // Always decode "$" buffers to Value::BufBulk even if feed Value::Bulk buffers
    /// assert_eq!(decoder.decode().unwrap(), Value::BufBulk("Hello".to_string().into_bytes()));
    /// ```
    pub fn with_buf_bulk(reader: BufReader<R>) -> Self {
        Decoder {
            buf_bulk: true,
            reader: reader,
        }
    }

    /// decode a value, will return `None` if no value decoded.
    pub fn decode(&mut self) -> Result<Value> {
        let mut res: Vec<u8> = Vec::new();
        if let Err(err) = self.reader.read_until(b'\n', &mut res) {
            return Err(Error::Io(err));
        }

        let len = res.len();
        if len < 3 {
            return Err(Error::Protocol(ErrorCode::InvalidString));
        }
        if res[len - 2] != b'\r' || res[len - 1] != b'\n' {
            return Err(Error::Protocol(ErrorCode::InvalidString));
        }

        let bytes = res[1..len - 2].as_ref();
        match res[0] {
            // Value::String
            b'+' => parse_string(bytes).and_then(|val| Ok(Value::String(val))),
            // Value::Error
            b'-' => parse_string(bytes).and_then(|val| Ok(Value::Error(val))),
            // Value::Integer
            b':' => parse_integer(bytes).and_then(|val| Ok(Value::Integer(val))),
            // Value::Bulk
            b'$' => {
                match parse_integer(bytes) {
                    Err(_) => Err(Error::Protocol(ErrorCode::InvalidBulk)),
                    Ok(int) => {
                        if int == -1 {
                            // Null bulk
                            return Ok(Value::Null);
                        }
                        if int < -1 || int >= RESP_MAX_SIZE {
                            return Err(Error::Protocol(ErrorCode::InvalidBulk));
                        }

                        let mut buf: Vec<u8> = Vec::new();
                        let int = int as usize;
                        buf.resize(int + 2, 0);

                        if let Err(err) = self.reader.read_exact(buf.as_mut_slice()) {
                            return Err(Error::Io(err));
                        }
                        if buf[int] != b'\r' || buf[int + 1] != b'\n' {
                            return Err(Error::Protocol(ErrorCode::InvalidString));
                        }
                        buf.truncate(int);
                        if self.buf_bulk {
                            return Ok(Value::BufBulk(buf));
                        }
                        parse_string(buf.as_slice()).and_then(|val| Ok(Value::Bulk(val)))
                    }
                }
            }
            // Value::Array
            b'*' => {
                match parse_integer(bytes) {
                    Err(_) => Err(Error::Protocol(ErrorCode::InvalidArray)),
                    Ok(int) => {
                        if int == -1 {
                            // Null array
                            return Ok(Value::NullArray);
                        }
                        if int < -1 || int >= RESP_MAX_SIZE {
                            return Err(Error::Protocol(ErrorCode::InvalidArray));
                        }

                        let mut array: Vec<Value> = Vec::with_capacity(int as usize);
                        for _ in 0..int {
                            match self.decode() {
                                Ok(value) => {
                                    array.push(value);
                                }
                                Err(err) => {
                                    return Err(err);
                                }
                            }
                        }
                        Ok(Value::Array(array))
                    }
                }
            }
            prefix => Err(Error::Protocol(ErrorCode::InvalidPrefix(prefix))),
        }
    }
}

#[inline]
fn parse_string(bytes: &[u8]) -> Result<String> {
    String::from_utf8(bytes.to_vec()).map_err(|err| Error::FromUtf8(err))
}

#[inline]
fn parse_integer(bytes: &[u8]) -> Result<i64> {
    let str_integer = try!(parse_string(bytes));
    (str_integer.parse::<i64>()).map_err(|_| Error::Protocol(ErrorCode::InvalidInteger))
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
        let buf = Value::Null.encode();
        let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(), Value::Null);
        assert!(decoder.decode().is_err());

        let buf = Value::NullArray.encode();
        let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(), Value::NullArray);
        assert!(decoder.decode().is_err());

        let buf = Value::String("OK".to_string()).encode();
        let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(), Value::String("OK".to_string()));
        assert!(decoder.decode().is_err());

        let buf = Value::Error("message".to_string()).encode();
        let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(),
                   Value::Error("message".to_string()));
        assert!(decoder.decode().is_err());

        let buf = Value::Integer(123456789).encode();
        let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(), Value::Integer(123456789));
        assert!(decoder.decode().is_err());

        let buf = Value::Bulk("Hello".to_string()).encode();
        let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(), Value::Bulk("Hello".to_string()));
        assert!(decoder.decode().is_err());

        let buf = Value::BufBulk("Hello".to_string().into_bytes()).encode();
        let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(), Value::Bulk("Hello".to_string()));
        assert!(decoder.decode().is_err());

        let array = vec!["SET", "a", "1"];
        let buf = encode_slice(&array);
        let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(),
                   Value::Array(vec![Value::Bulk("SET".to_string()),
                                     Value::Bulk("a".to_string()),
                                     Value::Bulk("1".to_string())]));
        assert!(decoder.decode().is_err());
    }

    #[test]
    fn struct_decoder_with_buf_bulk() {
        let buf = Value::Null.encode();
        let mut decoder = Decoder::with_buf_bulk(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(), Value::Null);
        assert!(decoder.decode().is_err());

        let buf = Value::NullArray.encode();
        let mut decoder = Decoder::with_buf_bulk(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(), Value::NullArray);
        assert!(decoder.decode().is_err());

        let buf = Value::String("OK".to_string()).encode();
        let mut decoder = Decoder::with_buf_bulk(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(), Value::String("OK".to_string()));
        assert!(decoder.decode().is_err());

        let buf = Value::Error("message".to_string()).encode();
        let mut decoder = Decoder::with_buf_bulk(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(),
                   Value::Error("message".to_string()));
        assert!(decoder.decode().is_err());

        let buf = Value::Integer(123456789).encode();
        let mut decoder = Decoder::with_buf_bulk(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(), Value::Integer(123456789));
        assert!(decoder.decode().is_err());

        let buf = Value::Bulk("Hello".to_string()).encode();
        let mut decoder = Decoder::with_buf_bulk(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(),
                   Value::BufBulk("Hello".to_string().into_bytes()));
        assert!(decoder.decode().is_err());

        let buf = Value::BufBulk("Hello".to_string().into_bytes()).encode();
        let mut decoder = Decoder::with_buf_bulk(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(),
                   Value::BufBulk("Hello".to_string().into_bytes()));
        assert!(decoder.decode().is_err());

        let array = vec!["SET", "a", "1"];
        let buf = encode_slice(&array);
        let mut decoder = Decoder::with_buf_bulk(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(),
                   Value::Array(vec![Value::BufBulk("SET".to_string().into_bytes()),
                                     Value::BufBulk("a".to_string().into_bytes()),
                                     Value::BufBulk("1".to_string().into_bytes())]));
        assert!(decoder.decode().is_err());
    }

    #[test]
    fn struct_decoder_with_invalid_data() {
        let buf: &[u8] = &[];
        let mut decoder = Decoder::new(BufReader::new(buf));
        assert!(decoder.decode().is_err());


        let buf = Value::String("OK正".to_string()).encode();
        let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(),
                   Value::String("OK正".to_string()));
        assert!(decoder.decode().is_err());

        let mut buf = Value::String("OK正".to_string()).encode();
        // [43, 79, 75, 230, 173, 163, 13, 10]
        buf.remove(5);
        let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
        assert!(decoder.decode().is_err());


        let buf = "$\r\n".to_string().into_bytes();
        let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
        assert!(decoder.decode().is_err());

        let buf = "$-2\r\n".to_string().into_bytes();
        let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
        assert!(decoder.decode().is_err());

        let buf = "&-1\r\n".to_string().into_bytes();
        let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
        assert!(decoder.decode().is_err());

        let buf = "$-1\r\n".to_string().into_bytes();
        let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(), Value::Null);
        assert!(decoder.decode().is_err());

        let buf = "$0\r\n\r\n".to_string().into_bytes();
        let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
        assert_eq!(decoder.decode().unwrap(), Value::Bulk("".to_string()));
        assert!(decoder.decode().is_err());
    }

    // #[test]
    // fn struct_decoder_continuingly() {
    //     let mut decoder = Decoder::new();

    //     let buf = "$0\r\n".to_string().into_bytes();
    //     assert_eq!(decoder.feed(&buf).unwrap(), ());
    //     assert_eq!(decoder.decode(), None);
    //     let buf = "\r\n".to_string().into_bytes();
    //     assert_eq!(decoder.feed(&buf).unwrap(), ());
    //     assert_eq!(decoder.decode().unwrap(), Value::Bulk("".to_string()));

    //     let _values = vec![Value::Null,
    //                        Value::NullArray,
    //                        Value::String("abcdefg".to_string()),
    //                        Value::Error("abcdefg".to_string()),
    //                        Value::Integer(123456789),
    //                        Value::Bulk("abcdefg".to_string())];
    //     let mut values = _values.clone();
    //     values.push(Value::Array(_values));
    //     let buf: Vec<u8> = values.iter().flat_map(|value| value.encode()).collect();
    //     let mut read_values: Vec<Value> = Vec::new();

    //     // feed byte by byte~
    //     for byte in buf {
    //         let byte = vec![byte];
    //         assert_eq!(decoder.feed(&byte).unwrap(), ());
    //         if decoder.result_len() > 0 {
    //             // one value should be parsed.
    //             assert_eq!(decoder.result_len(), 1);
    //             // buffer should be clear.
    //             assert_eq!(decoder.buffer_len(), 0);
    //             read_values.push(decoder.decode().unwrap());
    //             assert_eq!(decoder.result_len(), 0);
    //         } else {
    //             assert_eq!(decoder.buffer_len() > 0, true);
    //             assert_eq!(decoder.result_len(), 0);
    //         }
    //     }
    //     assert_eq!(&read_values, &values);
    // }
}