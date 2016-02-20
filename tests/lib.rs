//! RESP and serialization

extern crate resp;

use resp::{Value, encode, encode_slice, Decoder};

#[test]
fn enum_is_null() {
    assert_eq!(Value::Null.is_null(), true);
    assert_eq!(Value::NullArray.is_null(), true);
    assert_eq!(Value::Integer(123).is_null(), false);
}

#[test]
fn enum_is_error() {
    assert_eq!(Value::Null.is_error(), false);
    assert_eq!(Value::NullArray.is_error(), false);
    assert_eq!(Value::Error("".to_string()).is_error(), true);
}

#[test]
fn enum_encode() {
    let val = Value::String("OK正".to_string());
    assert_eq!(val.encode(), vec![43, 79, 75, 230, 173, 163, 13, 10]);
}

#[test]
fn enum_to_encoded_string() {
    let val = Value::String("OK正".to_string());
    assert_eq!(val.to_encoded_string().unwrap(), "+OK正\r\n");
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
}

#[test]
fn fn_encode() {
    let val = Value::String("OK正".to_string());
    assert_eq!(encode(&val), vec![43, 79, 75, 230, 173, 163, 13, 10]);
}

#[test]
fn fn_encode_slice() {
    let array = ["SET", "a", "1"];
    assert_eq!(encode_slice(&array),
        "*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n".to_string().into_bytes());
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
}
