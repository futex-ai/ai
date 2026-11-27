//! Buffers encoded object fields, with the final value of duplicate keys retained.

use std::collections::BTreeMap;

use serde::{
    Serialize,
    ser::{Error, SerializeMap, SerializeStruct, SerializeStructVariant},
};
use serde_json::Result;

use super::{
    key::MapKeySerializer,
    raw::{self, RAW_VALUE_TOKEN},
    serializer::{CanonicalSerializer, tagged},
};

enum Encoding {
    Fields(BTreeMap<String, Vec<u8>>),
    Ordered(BTreeMap<String, (usize, Vec<u8>)>),
    Raw(Option<Vec<u8>>),
}

pub(crate) struct Object {
    encoding: Encoding,
    key: Option<String>,
    variant: Option<&'static str>,
}

impl Object {
    pub(super) fn new(variant: Option<&'static str>) -> Self {
        Self {
            encoding: Encoding::Fields(BTreeMap::new()),
            key: None,
            variant,
        }
    }

    pub(super) fn ordered() -> Self {
        Self {
            encoding: Encoding::Ordered(BTreeMap::new()),
            key: None,
            variant: None,
        }
    }

    pub(super) fn raw() -> Self {
        Self {
            encoding: Encoding::Raw(None),
            key: None,
            variant: None,
        }
    }

    fn field<T: ?Sized + Serialize>(&mut self, key: &str, value: &T) -> Result<()> {
        match &mut self.encoding {
            Encoding::Fields(fields) => {
                fields.insert(
                    key.to_owned(),
                    value.serialize(CanonicalSerializer::SORTED)?,
                );
            }
            Encoding::Ordered(fields) => {
                let bytes = value.serialize(CanonicalSerializer::SORTED)?;
                let position = fields.len();
                fields
                    .entry(key.to_owned())
                    .or_insert((position, Vec::new()))
                    .1 = bytes;
            }
            Encoding::Raw(bytes) if key == RAW_VALUE_TOKEN => {
                *bytes = Some(raw::serialize(value)?);
            }
            Encoding::Raw(_) => return Err(serde_json::Error::custom("expected RawValue")),
        }
        Ok(())
    }

    fn finish(self) -> Result<Vec<u8>> {
        let bytes = match self.encoding {
            Encoding::Fields(fields) => encode_fields(fields)?,
            Encoding::Ordered(fields) => {
                let mut fields: Vec<_> = fields.into_iter().collect();
                fields.sort_unstable_by_key(|(_, (position, _))| *position);
                encode_fields(fields.into_iter().map(|(key, (_, value))| (key, value)))?
            }
            Encoding::Raw(Some(bytes)) => return Ok(bytes),
            Encoding::Raw(None) => return Err(serde_json::Error::custom("expected RawValue")),
        };
        match self.variant {
            Some(variant) => tagged(variant, bytes),
            None => Ok(bytes),
        }
    }
}

fn encode_fields(fields: impl IntoIterator<Item = (String, Vec<u8>)>) -> Result<Vec<u8>> {
    let mut bytes = vec![b'{'];
    for (key, value) in fields {
        if bytes.len() > 1 {
            bytes.push(b',');
        }
        bytes.extend(serde_json::to_vec(&key)?);
        bytes.push(b':');
        bytes.extend(value);
    }
    bytes.push(b'}');
    Ok(bytes)
}

impl SerializeMap for Object {
    type Ok = Vec<u8>;
    type Error = serde_json::Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        self.key = Some(key.serialize(MapKeySerializer)?);
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        let Some(key) = self.key.take() else {
            return Err(serde_json::Error::custom("map key must precede its value"));
        };
        self.field(&key, value)
    }

    fn end(self) -> Result<Vec<u8>> {
        self.finish()
    }
}

macro_rules! object {
    ($trait:ident) => {
        impl $trait for Object {
            type Ok = Vec<u8>;
            type Error = serde_json::Error;

            fn serialize_field<T: ?Sized + Serialize>(
                &mut self,
                key: &'static str,
                value: &T,
            ) -> Result<()> {
                self.field(key, value)
            }

            fn end(self) -> Result<Vec<u8>> {
                self.finish()
            }
        }
    };
}

object!(SerializeStruct);
object!(SerializeStructVariant);
