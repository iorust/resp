//! RESP Value

use self::value::{Value};
use std::iter::IntoIterator;

pub struct Deserializer<Iter: Iterator<Item=Result<Value>>> {
    rdr: Iter,
    buf: Vec<u8>,
    pos: usize,
    res: Vec<Value>,
}

impl Deserializer {
    pub fn new() -> Self {

    }

    pub fn feed(buf: &Vec<u8>) {

    }

    fn parse(&self) {

    }

    fn readBuf(&self) -> Option<u8> {

    }
}

impl IntoIterator for Deserializer {
    type Item = Result<Value>;
    type IntoIter: Iterator;
    fn into_iter(self) -> Self::IntoIter {
        
    }
}
