// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::fmt;
use std::cell::Cell;
use std::collections::BTreeMap;

use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Number;
use zeroize::{Zeroize, Zeroizing};

use crate::JcsError;

/// Parses one JSON text while retaining duplicate-member information that
/// would be lost by ordinary `serde_json::Value` deserialization.
pub(crate) fn parse_json_text(input: &str) -> Result<SensitiveJsonValue, JcsError> {
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

/// Owned JSON value parsed from caller text.
///
/// Raw JSON strings, object names, and document structure can carry PII. This
/// type gives parser output an explicit owner whose drop path scrubs all owned
/// string buffers before releasing them, including partially parsed values on
/// serde error paths.
pub(crate) enum SensitiveJsonValue {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<SensitiveJsonValue>),
    Object(BTreeMap<String, SensitiveJsonValue>),
}

impl SensitiveJsonValue {
    fn zeroize_owned(&mut self) {
        match self {
            Self::Null | Self::Bool(_) | Self::Number(_) => {}
            Self::String(text) => text.zeroize(),
            Self::Array(values) => {
                // Each child owns its own Drop cleanup. Clearing the array
                // avoids repeatedly traversing each subtree at every ancestor.
                values.clear();
            }
            Self::Object(values) => {
                let owned_entries = core::mem::take(values);
                for (mut key, value) in owned_entries {
                    key.zeroize();
                    drop(value);
                }
            }
        }
    }
}

impl Drop for SensitiveJsonValue {
    fn drop(&mut self) {
        self.zeroize_owned();
    }
}

#[derive(Clone, Copy)]
struct StrictValueSeed<'a> {
    duplicate_property: &'a Cell<bool>,
}

impl<'de> DeserializeSeed<'de> for StrictValueSeed<'_> {
    type Value = SensitiveJsonValue;

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
    type Value = SensitiveJsonValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("one valid JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(SensitiveJsonValue::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(SensitiveJsonValue::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(SensitiveJsonValue::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Number::from_f64(value)
            .map(SensitiveJsonValue::Number)
            .ok_or_else(|| E::custom(InvalidFiniteNumber))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(SensitiveJsonValue::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(SensitiveJsonValue::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(SensitiveJsonValue::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(SensitiveJsonValue::Null)
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
        Ok(SensitiveJsonValue::Array(values))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        // Establish the wiping owner before parsing any members: errors from
        // next_key or next_value must also clear already accepted object keys.
        let mut result = SensitiveJsonValue::Object(BTreeMap::new());
        let seed = StrictValueSeed {
            duplicate_property: self.duplicate_property,
        };
        if let SensitiveJsonValue::Object(values) = &mut result {
            while let Some(key) = object.next_key::<String>()? {
                let mut key = Zeroizing::new(key);
                let is_duplicate = values.contains_key(key.as_str());
                let value = object.next_value_seed(seed)?;
                if is_duplicate {
                    self.duplicate_property.set(true);
                } else {
                    values.insert(core::mem::take(&mut *key), value);
                }
            }
        }
        Ok(result)
    }
}

#[derive(Clone, Copy)]
struct InvalidFiniteNumber;

impl fmt::Display for InvalidFiniteNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON numbers must be finite")
    }
}
