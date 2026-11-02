//! Serializes typed values into bytes while preserving the former JSON value encoding.

use std::fmt::Display;

use serde::{Serialize, Serializer, ser::Error};
use serde_json::Result;

use super::{object::Object, raw::RAW_VALUE_TOKEN, sequence::Sequence};

pub(crate) struct CanonicalSerializer;

macro_rules! scalar {
    ($($method:ident($kind:ty)),* $(,)?) => {
        $(fn $method(self, value: $kind) -> Result<Vec<u8>> {
            serde_json::to_vec(&value)
        })*
    };
}

impl Serializer for CanonicalSerializer {
    type Ok = Vec<u8>;
    type Error = serde_json::Error;
    type SerializeSeq = Sequence;
    type SerializeTuple = Sequence;
    type SerializeTupleStruct = Sequence;
    type SerializeTupleVariant = Sequence;
    type SerializeMap = Object;
    type SerializeStruct = Object;
    type SerializeStructVariant = Object;

    scalar!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_f64(f64),
        serialize_char(char),
        serialize_str(&str),
        serialize_bytes(&[u8]),
    );

    fn serialize_i128(self, value: i128) -> Result<Vec<u8>> {
        if let Ok(value) = u64::try_from(value) {
            self.serialize_u64(value)
        } else if let Ok(value) = i64::try_from(value) {
            self.serialize_i64(value)
        } else {
            Err(serde_json::Error::custom("number out of range"))
        }
    }

    fn serialize_u128(self, value: u128) -> Result<Vec<u8>> {
        match u64::try_from(value) {
            Ok(value) => self.serialize_u64(value),
            Err(_) => Err(serde_json::Error::custom("number out of range")),
        }
    }

    fn serialize_f32(self, value: f32) -> Result<Vec<u8>> {
        self.serialize_f64(f64::from(value))
    }

    fn serialize_none(self) -> Result<Vec<u8>> {
        self.serialize_unit()
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Vec<u8>> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Vec<u8>> {
        serde_json::to_vec(&())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Vec<u8>> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<Vec<u8>> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Vec<u8>> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Vec<u8>> {
        tagged(variant, value.serialize(self)?)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Sequence> {
        Ok(Sequence::new(None))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Sequence> {
        self.serialize_seq(None)
    }

    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Sequence> {
        self.serialize_seq(None)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Sequence> {
        Ok(Sequence::new(Some(variant)))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Object> {
        Ok(Object::new(None))
    }

    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Object> {
        if name == RAW_VALUE_TOKEN {
            Ok(Object::raw())
        } else {
            self.serialize_map(None)
        }
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Object> {
        Ok(Object::new(Some(variant)))
    }

    fn collect_str<T: ?Sized + Display>(self, value: &T) -> Result<Vec<u8>> {
        self.serialize_str(&value.to_string())
    }
}

pub(super) fn tagged(variant: &str, value: Vec<u8>) -> Result<Vec<u8>> {
    let mut bytes = vec![b'{'];
    bytes.extend(serde_json::to_vec(variant)?);
    bytes.push(b':');
    bytes.extend(value);
    bytes.push(b'}');
    Ok(bytes)
}
