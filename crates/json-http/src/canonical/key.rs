//! Map-key encoding that retains the former typed JSON request contract.

use std::fmt::Display;

use serde::{
    Serialize, Serializer,
    ser::{Error, Impossible},
};
use serde_json::Result;

pub(super) struct MapKeySerializer;

fn invalid_key() -> serde_json::Error {
    serde_json::Error::custom("key must be a string")
}

macro_rules! scalar {
    ($($method:ident($kind:ty)),* $(,)?) => {
        $(fn $method(self, value: $kind) -> Result<String> {
            Ok(value.to_string())
        })*
    };
}

macro_rules! floating {
    ($($method:ident($kind:ty)),* $(,)?) => {
        $(fn $method(self, value: $kind) -> Result<String> {
            if value.is_finite() {
                serde_json::to_string(&value)
            } else {
                Err(serde_json::Error::custom("float key must be finite (got NaN or +/-inf)"))
            }
        })*
    };
}

impl Serializer for MapKeySerializer {
    type Ok = String;
    type Error = serde_json::Error;
    type SerializeSeq = Impossible<String, serde_json::Error>;
    type SerializeTuple = Impossible<String, serde_json::Error>;
    type SerializeTupleStruct = Impossible<String, serde_json::Error>;
    type SerializeTupleVariant = Impossible<String, serde_json::Error>;
    type SerializeMap = Impossible<String, serde_json::Error>;
    type SerializeStruct = Impossible<String, serde_json::Error>;
    type SerializeStructVariant = Impossible<String, serde_json::Error>;

    scalar!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_i128(i128),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_u128(u128),
        serialize_char(char),
        serialize_str(&str),
    );
    floating!(serialize_f32(f32), serialize_f64(f64));

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<String> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<String> {
        value.serialize(self)
    }

    fn collect_str<T: ?Sized + Display>(self, value: &T) -> Result<String> {
        Ok(value.to_string())
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<String> {
        Err(invalid_key())
    }

    fn serialize_none(self) -> Result<String> {
        Err(invalid_key())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, _value: &T) -> Result<String> {
        Err(invalid_key())
    }

    fn serialize_unit(self) -> Result<String> {
        Err(invalid_key())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<String> {
        Err(invalid_key())
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<String> {
        Err(invalid_key())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(invalid_key())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(invalid_key())
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(invalid_key())
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(invalid_key())
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(invalid_key())
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(invalid_key())
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(invalid_key())
    }
}
