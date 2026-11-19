// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use codec_proto::generated::proto::reallyme::codec::v1::{
    CodecJsonArray, CodecJsonMember, CodecJsonObject,
};
use serde::de::{DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor};
use serde::Deserializer;

use super::{
    project::{invalid_value, Projection},
    project_key::Key,
    JsonError, Value,
};

pub(super) struct ArrayAccess<'a>(core::slice::Iter<'a, Value>);
impl<'a> ArrayAccess<'a> {
    pub(super) fn finish(&self) -> Result<(), JsonError> {
        if self.0.len() == 0 {
            Ok(())
        } else {
            Err(invalid_value())
        }
    }
    pub(super) fn new(value: &'a CodecJsonArray) -> Self {
        Self(value.values.iter())
    }
}
impl<'de> SeqAccess<'de> for ArrayAccess<'de> {
    type Error = JsonError;
    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, JsonError> {
        self.0
            .next()
            .map(|value| seed.deserialize(Projection(value)))
            .transpose()
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.0.len())
    }
}

pub(super) struct ObjectAccess<'a> {
    members: core::slice::Iter<'a, CodecJsonMember>,
    pending: Option<&'a Value>,
}
impl<'a> ObjectAccess<'a> {
    pub(super) fn finish(&self) -> Result<(), JsonError> {
        if self.members.len() == 0 && self.pending.is_none() {
            Ok(())
        } else {
            Err(invalid_value())
        }
    }
    pub(super) fn new(value: &'a CodecJsonObject) -> Self {
        Self {
            members: value.members.iter(),
            pending: None,
        }
    }
}
impl<'de> MapAccess<'de> for ObjectAccess<'de> {
    type Error = JsonError;
    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, JsonError> {
        if self.pending.is_some() {
            return Err(invalid_value());
        }
        self.members
            .next()
            .map(|member| {
                self.pending = Some(member.value.as_option().ok_or_else(invalid_value)?);
                seed.deserialize(Key(member.name.as_str()))
            })
            .transpose()
    }
    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, JsonError> {
        seed.deserialize(Projection(self.pending.take().ok_or_else(invalid_value)?))
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.members.len())
    }
}

pub(super) struct JsonEnum<'a> {
    pub(super) name: &'a str,
    pub(super) value: Option<&'a Value>,
}
impl<'de> EnumAccess<'de> for JsonEnum<'de> {
    type Error = JsonError;
    type Variant = Self;
    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self), JsonError> {
        let variant = seed.deserialize(Key(self.name))?;
        Ok((variant, self))
    }
}
impl<'de> VariantAccess<'de> for JsonEnum<'de> {
    type Error = JsonError;
    fn unit_variant(self) -> Result<(), JsonError> {
        if let Some(value) = self.value {
            <() as serde::Deserialize>::deserialize(Projection(value))
        } else {
            Ok(())
        }
    }
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, JsonError> {
        seed.deserialize(Projection(self.value.ok_or_else(invalid_value)?))
    }
    fn tuple_variant<V: Visitor<'de>>(
        self,
        length: usize,
        visitor: V,
    ) -> Result<V::Value, JsonError> {
        Projection(self.value.ok_or_else(invalid_value)?).deserialize_tuple(length, visitor)
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, JsonError> {
        Projection(self.value.ok_or_else(invalid_value)?).deserialize_struct(
            "JSON variant",
            fields,
            visitor,
        )
    }
}
