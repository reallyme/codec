// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::fmt;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::JcsError;

/// Parses one JSON text while retaining duplicate-member information that
/// would be lost by ordinary `serde_json::Value` deserialization.
pub(crate) fn parse_json_text(input: &str) -> Result<SensitiveJsonValue, JcsError> {
    static NUMBER_VISITOR_IS_SAFE: OnceLock<bool> = OnceLock::new();
    if !NUMBER_VISITOR_IS_SAFE.get_or_init(|| {
        matches!(
            parse_json_text_unchecked("1.5"),
            Ok(SensitiveJsonValue::Number(_))
        )
    }) {
        return Err(JcsError::UnsupportedNumberRepresentation);
    }
    parse_json_text_unchecked(input)
}

fn parse_json_text_unchecked(input: &str) -> Result<SensitiveJsonValue, JcsError> {
    let duplicate_property = Cell::new(false);
    let depth_exceeded = Cell::new(false);
    let seed = StrictValueSeed {
        duplicate_property: &duplicate_property,
        depth_exceeded: &depth_exceeded,
        depth: 0,
    };
    let mut deserializer = serde_json::Deserializer::from_str(input);
    deserializer.disable_recursion_limit();
    let value = seed.deserialize(&mut deserializer).map_err(|_| {
        if depth_exceeded.get() {
            JcsError::DepthExceeded
        } else {
            JcsError::InvalidJson
        }
    })?;
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
    Number(SensitiveNumber),
    String(String),
    #[expect(
        clippy::vec_box,
        reason = "array growth must not copy sensitive numeric payloads"
    )]
    Array(Vec<Box<SensitiveJsonValue>>),
    Object(BTreeMap<String, SensitiveJsonValue>),
}

/// Parsed numeric value kept in a primitive that supports volatile wiping.
/// `serde_json::Number` does not implement `Zeroize`, so retaining it in a
/// sensitive parse tree would leave numeric identifiers in freed storage.
pub(crate) enum SensitiveNumber {
    Signed(i64),
    Unsigned(u64),
    Float(f64),
}

impl Zeroize for SensitiveNumber {
    fn zeroize(&mut self) {
        match self {
            Self::Signed(value) => value.zeroize(),
            Self::Unsigned(value) => value.zeroize(),
            Self::Float(value) => value.zeroize(),
        }
    }
}

impl Drop for SensitiveNumber {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for SensitiveNumber {}

impl SensitiveJsonValue {
    fn zeroize_owned(&mut self) {
        match self {
            Self::Null | Self::Bool(_) => {}
            Self::Number(_) => {}
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

impl ZeroizeOnDrop for SensitiveJsonValue {}

#[derive(Clone, Copy)]
struct StrictValueSeed<'a> {
    duplicate_property: &'a Cell<bool>,
    depth_exceeded: &'a Cell<bool>,
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for StrictValueSeed<'_> {
    type Value = SensitiveJsonValue;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictValueVisitor {
            duplicate_property: self.duplicate_property,
            depth_exceeded: self.depth_exceeded,
            depth: self.depth,
        })
    }
}

struct StrictValueVisitor<'a> {
    duplicate_property: &'a Cell<bool>,
    depth_exceeded: &'a Cell<bool>,
    depth: usize,
}

impl StrictValueVisitor<'_> {
    fn child_depth<E: serde::de::Error>(&self) -> Result<usize, E> {
        let next = self.depth.checked_add(1).ok_or_else(|| {
            self.depth_exceeded.set(true);
            E::custom(InvalidDepth)
        })?;
        if next > crate::MAX_NESTING_DEPTH {
            self.depth_exceeded.set(true);
            return Err(E::custom(InvalidDepth));
        }
        Ok(next)
    }
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
        Ok(SensitiveJsonValue::Number(SensitiveNumber::Signed(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(SensitiveJsonValue::Number(SensitiveNumber::Unsigned(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if !value.is_finite() {
            return Err(E::custom(InvalidFiniteNumber));
        }
        Ok(SensitiveJsonValue::Number(SensitiveNumber::Float(value)))
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
        let child_depth = self.child_depth::<A::Error>()?;
        // Vec growth moves only box pointers; parsed numbers and strings stay
        // at stable owned addresses and are wiped when their boxes drop.
        let mut values = Vec::new();
        let seed = StrictValueSeed {
            duplicate_property: self.duplicate_property,
            depth_exceeded: self.depth_exceeded,
            depth: child_depth,
        };
        while let Some(value) = sequence.next_element_seed(seed)? {
            values.push(Box::new(value));
        }
        Ok(SensitiveJsonValue::Array(values))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        // Establish the wiping owner before parsing any members: errors from
        // next_key or next_value must also clear already accepted object keys.
        let child_depth = self.child_depth::<A::Error>()?;
        let mut result = SensitiveJsonValue::Object(BTreeMap::new());
        let seed = StrictValueSeed {
            duplicate_property: self.duplicate_property,
            depth_exceeded: self.depth_exceeded,
            depth: child_depth,
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

#[derive(Clone, Copy)]
struct InvalidDepth;

impl fmt::Display for InvalidDepth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON nesting exceeds the supported limit")
    }
}
