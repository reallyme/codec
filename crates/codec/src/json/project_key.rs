// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{project::invalid_value, JsonError};
use serde::de::{value::BorrowedStrDeserializer, Visitor};

pub(super) struct Key<'a>(pub(super) &'a str);

macro_rules! numeric_key {
    ($method:ident) => {
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
            super::project::number(number(self.0)?, visitor)
        }
    };
}
fn number(text: &str) -> Result<&str, JsonError> {
    if !matches!(text.as_bytes().first(), Some(b'-' | b'0'..=b'9')) {
        return Err(invalid_value());
    }
    let raw: &serde_json::value::RawValue =
        serde_json::from_str(text).map_err(|_| invalid_value())?;
    if raw.get().len() != text.len() {
        return Err(invalid_value());
    }
    Ok(text)
}
impl<'de> serde::Deserializer<'de> for Key<'de> {
    type Error = JsonError;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        visitor.visit_borrowed_str(self.0)
    }
    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        match self.0 {
            "true" => visitor.visit_bool(true),
            "false" => visitor.visit_bool(false),
            _ => Err(invalid_value()),
        }
    }
    numeric_key!(deserialize_i8);
    numeric_key!(deserialize_i16);
    numeric_key!(deserialize_i32);
    numeric_key!(deserialize_i64);
    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        visitor.visit_i128(
            number(self.0)?
                .parse::<i128>()
                .map_err(|_| invalid_value())?,
        )
    }
    numeric_key!(deserialize_u8);
    numeric_key!(deserialize_u16);
    numeric_key!(deserialize_u32);
    numeric_key!(deserialize_u64);
    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        visitor.visit_u128(
            number(self.0)?
                .parse::<u128>()
                .map_err(|_| invalid_value())?,
        )
    }
    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        super::project::float32(number(self.0)?, visitor)
    }
    numeric_key!(deserialize_f64);
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, JsonError> {
        visitor.visit_some(self)
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
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, JsonError> {
        BorrowedStrDeserializer::<JsonError>::new(self.0).deserialize_enum(name, variants, visitor)
    }
    serde::forward_to_deserialize_any! {
        char str string bytes byte_buf unit unit_struct seq tuple tuple_struct map struct identifier ignored_any
    }
}
