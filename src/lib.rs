//! RESP and serialization

pub use self::value::{ Value };
pub use self::serialize::{ encode, encode_slice, Decoder };

mod value;
mod serialize;
