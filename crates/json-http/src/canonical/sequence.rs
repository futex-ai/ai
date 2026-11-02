//! Incremental encoded arrays and tuple variants.

use serde::{
    Serialize,
    ser::{SerializeSeq, SerializeTuple, SerializeTupleStruct, SerializeTupleVariant},
};
use serde_json::Result;

use super::serializer::{CanonicalSerializer, tagged};

pub(crate) struct Sequence {
    bytes: Vec<u8>,
    variant: Option<&'static str>,
}

impl Sequence {
    pub(super) fn new(variant: Option<&'static str>) -> Self {
        Self {
            bytes: vec![b'['],
            variant,
        }
    }

    fn element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        if self.bytes.len() > 1 {
            self.bytes.push(b',');
        }
        self.bytes.extend(value.serialize(CanonicalSerializer)?);
        Ok(())
    }

    fn finish(mut self) -> Result<Vec<u8>> {
        self.bytes.push(b']');
        match self.variant {
            Some(variant) => tagged(variant, self.bytes),
            None => Ok(self.bytes),
        }
    }
}

macro_rules! sequence {
    ($trait:ident, $method:ident) => {
        impl $trait for Sequence {
            type Ok = Vec<u8>;
            type Error = serde_json::Error;

            fn $method<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
                self.element(value)
            }

            fn end(self) -> Result<Vec<u8>> {
                self.finish()
            }
        }
    };
}

sequence!(SerializeSeq, serialize_element);
sequence!(SerializeTuple, serialize_element);
sequence!(SerializeTupleStruct, serialize_field);
sequence!(SerializeTupleVariant, serialize_field);
