// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop};

mod error;
mod limits;

pub use error::{DeterministicCborError, DeterministicCborProfileError};
pub use limits::{
    DETERMINISTIC_CBOR_NEGATIVE_MAX, DETERMINISTIC_CBOR_NEGATIVE_MIN,
    MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES, MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES,
    MAX_DETERMINISTIC_CBOR_INPUT_LEN, MAX_DETERMINISTIC_CBOR_NESTING_DEPTH,
    MAX_DETERMINISTIC_CBOR_NODES, MAX_DETERMINISTIC_CBOR_OUTPUT_LEN,
};

/// Integer domain supported by the deterministic generic-CBOR profile.
///
/// The enum deliberately distinguishes unsigned and negative values so the
/// full `u64` range is preserved without admitting two representations for
/// nonnegative integers.
#[non_exhaustive]
pub enum DeterministicCborInteger {
    /// Unsigned integer in the inclusive range `0..=u64::MAX`.
    Unsigned(u64),
    /// Negative integer in the inclusive range `i64::MIN..=-1`.
    Negative(DeterministicCborNegativeInteger),
}

impl DeterministicCborInteger {
    /// Construct an unsigned deterministic-CBOR integer.
    pub const fn unsigned(value: u64) -> Self {
        Self::Unsigned(value)
    }

    /// Construct a negative deterministic-CBOR integer.
    ///
    /// Zero and positive values are rejected so callers cannot create a second
    /// semantic spelling for a value that belongs in [`Self::Unsigned`].
    pub const fn negative(value: i64) -> Result<Self, DeterministicCborProfileError> {
        if value < 0 {
            Ok(Self::Negative(DeterministicCborNegativeInteger { value }))
        } else {
            Err(DeterministicCborProfileError::NegativeIntegerMustBeNegative)
        }
    }
}

impl fmt::Debug for DeterministicCborInteger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsigned(_) => formatter
                .debug_tuple("DeterministicCborInteger::Unsigned")
                .field(&Redacted)
                .finish(),
            Self::Negative(value) => formatter
                .debug_tuple("DeterministicCborInteger::Negative")
                .field(value)
                .finish(),
        }
    }
}

impl DeterministicCborInteger {
    fn zeroize_owned(&mut self) {
        match self {
            Self::Unsigned(value) => value.zeroize(),
            Self::Negative(value) => value.zeroize_owned(),
        }
    }
}

impl Drop for DeterministicCborInteger {
    fn drop(&mut self) {
        self.zeroize_owned();
    }
}

impl ZeroizeOnDrop for DeterministicCborInteger {}

/// Validated negative integer in the deterministic-CBOR supported range.
///
/// The field is private so callers cannot bypass
/// [`DeterministicCborInteger::negative`] and create a nonnegative value in the
/// negative-integer variant.
pub struct DeterministicCborNegativeInteger {
    value: i64,
}

impl DeterministicCborNegativeInteger {
    /// Return the validated negative value.
    pub const fn value(&self) -> i64 {
        self.value
    }
}

impl fmt::Debug for DeterministicCborNegativeInteger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("DeterministicCborNegativeInteger")
            .field(&Redacted)
            .finish()
    }
}

impl DeterministicCborNegativeInteger {
    fn zeroize_owned(&mut self) {
        self.value.zeroize();
    }
}

impl Drop for DeterministicCborNegativeInteger {
    fn drop(&mut self) {
        self.zeroize_owned();
    }
}

impl ZeroizeOnDrop for DeterministicCborNegativeInteger {}

/// Map key domain supported by deterministic generic CBOR.
///
/// Text keys are treated as potentially sensitive because the codec is a
/// generic infrastructure layer and cannot know whether a key contains PII or
/// credential material.
#[non_exhaustive]
pub enum DeterministicCborMapKey {
    /// Integer map key.
    Integer(DeterministicCborInteger),
    /// UTF-8 text map key.
    Text(String),
}

impl DeterministicCborMapKey {
    /// Construct a text key without Unicode normalization.
    ///
    /// Exact UTF-8 bytes define text-key equality in this profile. Validation
    /// and encoding must not normalize, case-fold, or apply locale-sensitive
    /// comparison to this value.
    pub fn text(value: String) -> Self {
        Self::Text(value)
    }
}

impl fmt::Debug for DeterministicCborMapKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(value) => formatter
                .debug_tuple("DeterministicCborMapKey::Integer")
                .field(value)
                .finish(),
            Self::Text(value) => formatter
                .debug_struct("DeterministicCborMapKey::Text")
                .field("byte_len", &value.len())
                .field("value", &Redacted)
                .finish(),
        }
    }
}

impl Drop for DeterministicCborMapKey {
    fn drop(&mut self) {
        self.zeroize_owned();
    }
}

impl DeterministicCborMapKey {
    fn zeroize_owned(&mut self) {
        match self {
            Self::Integer(value) => value.zeroize_owned(),
            Self::Text(value) => value.zeroize(),
        }
    }
}

impl ZeroizeOnDrop for DeterministicCborMapKey {}

/// Entry in a deterministic-CBOR map.
///
/// Maps use entry lists rather than host hash maps so duplicate keys can be
/// rejected by the profile validator and ordering never depends on runtime
/// iteration behavior. Construction intentionally preserves duplicates and
/// input order; it does not establish that an entry belongs to a validated
/// deterministic-CBOR tree.
pub struct DeterministicCborMapEntry {
    key: DeterministicCborMapKey,
    value: DeterministicCborValue,
}

impl DeterministicCborMapEntry {
    /// Construct a deterministic-CBOR map entry.
    pub fn new(key: DeterministicCborMapKey, value: DeterministicCborValue) -> Self {
        Self { key, value }
    }

    /// Borrow the map key.
    pub const fn key(&self) -> &DeterministicCborMapKey {
        &self.key
    }

    /// Borrow the map value.
    pub const fn value(&self) -> &DeterministicCborValue {
        &self.value
    }
}

impl fmt::Debug for DeterministicCborMapEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeterministicCborMapEntry")
            .field("key", &self.key)
            .field("value", &self.value)
            .finish()
    }
}

impl DeterministicCborMapEntry {
    fn zeroize_owned(&mut self) {
        self.key.zeroize_owned();
        self.value.zeroize_owned();
    }
}

impl Drop for DeterministicCborMapEntry {
    fn drop(&mut self) {
        self.zeroize_owned();
    }
}

impl ZeroizeOnDrop for DeterministicCborMapEntry {}

/// Value domain supported by deterministic generic CBOR.
///
/// Construction alone does not enforce aggregate size, node, nesting,
/// duplicate-key, or map-order rules. Boundary adapters must validate
/// untrusted transport data before constructing this recursive owner, and the
/// semantic encoder must validate caller-constructed values before allocating
/// output.
///
/// All owned payloads and container backing allocations are treated as
/// potentially sensitive and recursively zeroized on drop. Debug output is
/// structural and redacted.
#[non_exhaustive]
pub enum DeterministicCborValue {
    /// CBOR null.
    Null,
    /// CBOR boolean.
    Bool(bool),
    /// Supported deterministic-CBOR integer.
    Integer(DeterministicCborInteger),
    /// UTF-8 text string.
    Text(String),
    /// Byte string.
    Bytes(Vec<u8>),
    /// Array of deterministic-CBOR values.
    Array(Vec<DeterministicCborValue>),
    /// Map represented as an ordered entry list.
    Map(Vec<DeterministicCborMapEntry>),
}

impl fmt::Debug for DeterministicCborValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => formatter.write_str("DeterministicCborValue::Null"),
            Self::Bool(_) => formatter
                .debug_tuple("DeterministicCborValue::Bool")
                .field(&Redacted)
                .finish(),
            Self::Integer(value) => formatter
                .debug_tuple("DeterministicCborValue::Integer")
                .field(value)
                .finish(),
            Self::Text(value) => formatter
                .debug_struct("DeterministicCborValue::Text")
                .field("byte_len", &value.len())
                .field("value", &Redacted)
                .finish(),
            Self::Bytes(value) => formatter
                .debug_struct("DeterministicCborValue::Bytes")
                .field("byte_len", &value.len())
                .field("value", &Redacted)
                .finish(),
            Self::Array(values) => formatter
                .debug_struct("DeterministicCborValue::Array")
                .field("len", &values.len())
                .finish(),
            Self::Map(entries) => formatter
                .debug_struct("DeterministicCborValue::Map")
                .field("len", &entries.len())
                .finish(),
        }
    }
}

impl Drop for DeterministicCborValue {
    fn drop(&mut self) {
        self.zeroize_owned();
    }
}

impl DeterministicCborValue {
    fn zeroize_owned(&mut self) {
        match self {
            Self::Bool(value) => value.zeroize(),
            Self::Integer(value) => value.zeroize_owned(),
            Self::Text(value) => value.zeroize(),
            Self::Bytes(value) => value.zeroize(),
            Self::Array(values) => zeroize_values(values),
            Self::Map(entries) => zeroize_entries(entries),
            Self::Null => {}
        }
    }
}

impl ZeroizeOnDrop for DeterministicCborValue {}

pub(crate) fn try_vec_with_capacity<T>(capacity: usize) -> Result<Vec<T>, DeterministicCborError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(capacity)
        .map_err(|_| DeterministicCborError::AllocationFailure)?;
    Ok(values)
}

pub(crate) fn try_string_copy(value: &str) -> Result<String, DeterministicCborError> {
    let mut copy = String::new();
    copy.try_reserve_exact(value.len())
        .map_err(|_| DeterministicCborError::AllocationFailure)?;
    copy.push_str(value);
    Ok(copy)
}

fn zeroize_values(values: &mut Vec<DeterministicCborValue>) {
    for value in values.iter_mut() {
        value.zeroize_owned();
    }
    values.clear();
    values.spare_capacity_mut().zeroize();
}

fn zeroize_entries(entries: &mut Vec<DeterministicCborMapEntry>) {
    for entry in entries.iter_mut() {
        entry.zeroize_owned();
    }
    entries.clear();
    entries.spare_capacity_mut().zeroize();
}

struct Redacted;

impl fmt::Debug for Redacted {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[cfg(test)]
mod tests;
