// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use core::fmt;
use std::cell::Cell;

use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

use crate::JcsError;

/// Parses one JSON text while retaining duplicate-member information that
/// would be lost by ordinary `serde_json::Value` deserialization.
pub(crate) fn parse_json_text(input: &str) -> Result<Value, JcsError> {
    let duplicate_property = Cell::new(false);
    let seed = StrictValueSeed {
        duplicate_property: &duplicate_property,
    };
    let mut deserializer = serde_json::Deserializer::from_str(input);
    let value = seed
        .deserialize(&mut deserializer)
        .map_err(|_| JcsError::InvalidJson)?;
    deserializer.end().map_err(|_| JcsError::InvalidJson)?;

    if duplicate_property.get() {
        return Err(JcsError::DuplicateProperty);
    }
    Ok(value)
}

#[derive(Clone, Copy)]
struct StrictValueSeed<'a> {
    duplicate_property: &'a Cell<bool>,
}

impl<'de> DeserializeSeed<'de> for StrictValueSeed<'_> {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictValueVisitor {
            duplicate_property: self.duplicate_property,
        })
    }
}

struct StrictValueVisitor<'a> {
    duplicate_property: &'a Cell<bool>,
}

impl<'de> Visitor<'de> for StrictValueVisitor<'_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("one valid JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom(InvalidFiniteNumber))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        let seed = StrictValueSeed {
            duplicate_property: self.duplicate_property,
        };
        while let Some(value) = sequence.next_element_seed(seed)? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        let seed = StrictValueSeed {
            duplicate_property: self.duplicate_property,
        };
        while let Some(key) = object.next_key::<String>()? {
            let is_duplicate = values.contains_key(&key);
            let value = object.next_value_seed(seed)?;
            if is_duplicate {
                self.duplicate_property.set(true);
            } else {
                values.insert(key, value);
            }
        }
        Ok(Value::Object(values))
    }
}

#[derive(Clone, Copy)]
struct InvalidFiniteNumber;

impl fmt::Display for InvalidFiniteNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON numbers must be finite")
    }
}
