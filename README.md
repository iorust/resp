RESP
====
RESP(REdis Serialization Protocol) Serialization for Rust .

[![Crates version][version-image]][version-url]
[![Build Status][travis-image]][travis-url]
[![Coverage Status][coveralls-image]][coveralls-url]
[![Crates downloads][downloads-image]][downloads-url]

## API

```Rust
extern crate resp;
use resp::{ Value, encode, encode_slice, Decoder };
```

### Value

```Rust
enum Value {
    /// Null bulk reply, $-1\r\n
    Null,
    /// Null array reply, *-1\r\n
    NullArray,
    /// For Simple Strings the first byte of the reply is "+"
    String(String),
    /// For Errors the first byte of the reply is "-"
    Error(String),
    /// For Integers the first byte of the reply is ":"
    Integer(i64),
    /// For Bulk Strings the first byte of the reply is "$"
    Bulk(String),
    /// For Bulk <binary> Strings the first byte of the reply is "$"
    BufBulk(Vec<u8>),
    /// For Arrays the first byte of the reply is "*"
    Array(Vec<Value>),
}
```
RESP values.

#### Examples
```Rust
let err = Value::Error("error!".to_string());
let nul = Value::Null;
```

#### impl Value

##### `fn is_null(&self) -> bool`
```Rust
println!("{:?}", Value::Null.is_null())  // true
println!("{:?}", Value::NullArray.is_null())  // true
println!("{:?}", Value::Integer(123).is_null())  // false
```

##### `fn is_error(&self) -> bool`
```Rust
println!("{:?}", Value::Null.is_error())  // false
println!("{:?}", Value::NullArray.is_error())  // false
println!("{:?}", Value::Error("".to_string()).is_error())  // true
```

##### `fn encode(&self) -> Vec<u8>`
```Rust
let val = Value::String("OK正".to_string());
println!("{:?}", val.encode())  // [43, 79, 75, 230, 173, 163, 13, 10]
```

##### `fn to_encoded_string(&self) -> Result<String, FromUtf8Error>`
```Rust
let val = Value::String("OK正".to_string());
println!("{:?}", val.to_encoded_string().unwrap())  // "+OK正\r\n"
```

### encode

Encode a RESP value to buffer.

##### `fn encode(value: &Value) -> Vec<u8>`

```Rust
let val = Value::String("OK正".to_string());
println!("{:?}", encode(&val))  // [43, 79, 75, 230, 173, 163, 13, 10]
```

### encode_slice

Encode a slice of string to RESP request buffer. It is usefull for redis client to encode request command.

##### `fn encode_slice(array: &[&str]) -> Vec<u8>`

```Rust
let array = ["SET", "a", "1"];
println!("{:?}", String::from_utf8(encode_slice(&array)))
// Ok("*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n")
```

### Decoder

```Rust
struct Decoder {
    buf_bulk: bool,
    pos: usize,
    buf: Vec<u8>,
    res: Vec<Value>,
}
```
Decode redis reply buffers.

#### Examples
```Rust
let mut decoder = Decoder::new(false);
let buf = Value::NullArray.encode();

println!("{:?}", decoder.feed(&buf))  // Ok(())
println!("{:?}", decoder.read())  // Some(Value::NullArray)
```

#### impl Decoder

##### `fn new(buf_bulk: bool) -> Self`
```Rust
let mut decoder = Decoder::new(false);
```

##### `fn feed(&mut self, buf: &Vec<u8>) -> Result<(), String>`
```Rust
println!("{:?}", decoder.feed(&buf))  // Ok(())
```

##### `fn read(&mut self) -> Option<Value>`
```Rust
println!("{:?}", decoder.read())  // Some(Value::NullArray)
println!("{:?}", decoder.read())  // None
```

##### `fn buffer_len(&self) -> usize`
```Rust
println!("{:?}", decoder.buffer_len())  // 0
```

##### `fn result_len(&self) -> usize`
```Rust
println!("{:?}", decoder.result_len())  // 0
```

[version-image]: https://img.shields.io/crates/v/resp.svg
[version-url]: https://crates.io/crates/resp

[travis-image]: http://img.shields.io/travis/iorust/resp.svg
[travis-url]: https://travis-ci.org/iorust/resp

[coveralls-image]: https://coveralls.io/repos/github/iorust/resp/badge.svg?branch=master
[coveralls-url]: https://coveralls.io/github/iorust/resp?branch=master

[downloads-image]: https://img.shields.io/crates/d/resp.svg
[downloads-url]: https://crates.io/crates/resp
