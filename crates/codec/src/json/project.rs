// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use codec_proto::generated::proto::reallyme::codec::v1::codec_json_value::Kind;
use serde::de::{DeserializeOwned, Visitor};
use zeroize::Zeroizing;

use super::{
    project_access::{ArrayAccess, JsonEnum, ObjectAccess},
    ErrorReason as R, JsonError, Value,
};

pub(super) fn invalid_value() -> JsonError {
    JsonError::new(R::CODEC_ERROR_REASON_CANONICAL_JSON_INVALID_VALUE)
}

/// Project a generated JSON tree into an owned Serde target without creating
/// another JSON text or exposing field names in diagnostics.
///
/// Only the immediately preceding bounded decode calls this adapter. Keeping
/// it private prevents projections of externally constructed, unbounded trees.
pub(super) fn from_value<T: DeserializeOwned>(value: &Value) -> Result<T, JsonError> {
    T::deserialize(Projection(value))
}

pub(super) fn number<'de, V: Visitor<'de>>(text: &str, visitor: V) -> Result<V::Value, JsonError> {
    if text.starts_with('-') && text != "-0" {
        if let Ok(value) = text.parse::<i64>() {
            return visitor.visit_i64(value);
        }
    } else if let Ok(value) = text.parse::<u64>() {
        return visitor.visit_u64(value);
    }
    let value = Zeroizing::new(text.parse::<f64>().map_err(|_| invalid_value())?);
    if !value.is_finite() {
        return Err(invalid_value());
    }
    visitor.visit_f64(*value)
}

pub(super) fn float32<'de, V: Visitor<'de>>(text: &str, visitor: V) -> Result<V::Value, JsonError> {
    // Serde JSON retains integer visitors when the integer fits, but parses
    // decimal/exponent forms directly at single precision. Parsing as f64
    // first could double-round a halfway decimal and change its f32 value.
    if (text != "-0" && text.parse::<i64>().is_ok()) || text.parse::<u64>().is_ok() {
        return number(text, visitor);
    }
    let value = Zeroizing::new(text.parse::<f32>().map_err(|_| invalid_value())?);
    if !value.is_finite() {
        return Err(invalid_value());
    }
    visitor.visit_f64(f64::from(*value))
}

pub(super) struct Projection<'a>(pub(super) &'a Value);

// Match the input-shape checks of the streaming JSON deserializer even
// when a custom target visitor accepts additional scalar kinds.
macro_rules! numeric_projection {
    ($method:ident) => {
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
            match self.0.kind.as_ref() {
                Some(Kind::NumberText(text)) => number(text.as_str(), visitor),
                _ => Err(invalid_value()),
            }
        }
    };
}

impl<'de> serde::Deserializer<'de> for Projection<'de> {
    type Error = JsonError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        match self.0.kind.as_ref().ok_or_else(invalid_value)? {
            Kind::NullValue(_) => visitor.visit_unit(),
            Kind::BoolValue(value) => visitor.visit_bool(*value),
            Kind::StringValue(value) => visitor.visit_borrowed_str(value.as_str()),
            Kind::ArrayValue(value) => {
                let mut access = ArrayAccess::new(value);
                let result = visitor.visit_seq(&mut access)?;
                access.finish()?;
                Ok(result)
            }
            Kind::ObjectValue(value) => {
                let mut access = ObjectAccess::new(value);
                let result = visitor.visit_map(&mut access)?;
                access.finish()?;
                Ok(result)
            }
            Kind::NumberText(text) => number(text.as_str(), visitor),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        if matches!(self.0.kind, Some(Kind::NullValue(_))) {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, JsonError> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, JsonError> {
        match self.0.kind.as_ref().ok_or_else(invalid_value)? {
            Kind::StringValue(value) => visitor.visit_enum(JsonEnum {
                name: value.as_str(),
                value: None,
            }),
            Kind::ObjectValue(object) if object.members.len() == 1 => {
                let member = object.members.first().ok_or_else(invalid_value)?;
                visitor.visit_enum(JsonEnum {
                    name: member.name.as_str(),
                    value: Some(member.value.as_option().ok_or_else(invalid_value)?),
                })
            }
            _ => Err(invalid_value()),
        }
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        if let Some(Kind::StringValue(text)) = self.0.kind.as_ref() {
            visitor.visit_borrowed_bytes(text.as_bytes())
        } else {
            self.deserialize_seq(visitor)
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        visitor.visit_unit()
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        match self.0.kind.as_ref() {
            Some(Kind::NumberText(text)) => float32(text.as_str(), visitor),
            _ => Err(invalid_value()),
        }
    }

    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        match self.0.kind.as_ref() {
            Some(Kind::NumberText(text)) => {
                visitor.visit_i128(text.parse::<i128>().map_err(|_| invalid_value())?)
            }
            _ => Err(invalid_value()),
        }
    }
    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        match self.0.kind.as_ref() {
            Some(Kind::NumberText(text)) => {
                visitor.visit_u128(text.parse::<u128>().map_err(|_| invalid_value())?)
            }
            _ => Err(invalid_value()),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        match self.0.kind.as_ref() {
            Some(Kind::BoolValue(_)) => self.deserialize_any(visitor),
            _ => Err(invalid_value()),
        }
    }
    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        match self.0.kind.as_ref() {
            Some(Kind::StringValue(_)) => self.deserialize_any(visitor),
            _ => Err(invalid_value()),
        }
    }
    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        self.deserialize_str(visitor)
    }
    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        self.deserialize_str(visitor)
    }
    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        self.deserialize_str(visitor)
    }
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        match self.0.kind.as_ref() {
            Some(Kind::ArrayValue(_)) => self.deserialize_any(visitor),
            _ => Err(invalid_value()),
        }
    }
    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _length: usize,
        visitor: V,
    ) -> Result<V::Value, JsonError> {
        self.deserialize_seq(visitor)
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        length: usize,
        visitor: V,
    ) -> Result<V::Value, JsonError> {
        self.deserialize_tuple(length, visitor)
    }
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        match self.0.kind.as_ref() {
            Some(Kind::ObjectValue(_)) => self.deserialize_any(visitor),
            _ => Err(invalid_value()),
        }
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, JsonError> {
        match self.0.kind.as_ref() {
            Some(Kind::ObjectValue(_) | Kind::ArrayValue(_)) => self.deserialize_any(visitor),
            _ => Err(invalid_value()),
        }
    }
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        match self.0.kind.as_ref() {
            Some(Kind::NullValue(_)) => visitor.visit_unit(),
            _ => Err(invalid_value()),
        }
    }
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, JsonError> {
        self.deserialize_unit(visitor)
    }

    numeric_projection!(deserialize_i8);
    numeric_projection!(deserialize_i16);
    numeric_projection!(deserialize_i32);
    numeric_projection!(deserialize_i64);
    numeric_projection!(deserialize_u8);
    numeric_projection!(deserialize_u16);
    numeric_projection!(deserialize_u32);
    numeric_projection!(deserialize_u64);
    numeric_projection!(deserialize_f64);
}
