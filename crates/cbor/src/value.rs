// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use zeroize::{Zeroize, ZeroizeOnDrop};

/// A decoded DAG-CBOR value.
///
/// Values wipe owned text and byte buffers on drop. Callers can also call
/// [`Zeroize::zeroize`] to clear a value before its owner is dropped.
#[derive(PartialEq)]
#[non_exhaustive]
pub enum CborValue {
    /// CBOR null.
    Null,
    /// CBOR boolean.
    Bool(bool),
    /// A signed integer within the `i64` range.
    Int(i64),
    /// A UTF-8 text string.
    String(String),
    /// A byte string.
    Bytes(Vec<u8>),
    /// An array of values.
    Array(Vec<CborValue>),
    /// A map of text-string keys to values, in canonical key order.
    Map(Vec<(String, CborValue)>),
}

impl core::fmt::Debug for CborValue {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let kind = match self {
            Self::Null => "Null",
            Self::Bool(_) => "Bool",
            Self::Int(_) => "Int",
            Self::String(_) => "String",
            Self::Bytes(_) => "Bytes",
            Self::Array(_) => "Array",
            Self::Map(_) => "Map",
        };
        formatter.debug_tuple("CborValue").field(&kind).finish()
    }
}

// Zeroize nested fields before the enum releases owned buffers.
impl Zeroize for CborValue {
    fn zeroize(&mut self) {
        match self {
            Self::Null => {}
            Self::Bool(value) => value.zeroize(),
            Self::Int(value) => value.zeroize(),
            Self::String(value) => value.zeroize(),
            Self::Bytes(value) => value.zeroize(),
            Self::Array(values) => values.zeroize(),
            Self::Map(entries) => entries.zeroize(),
        }
    }
}

impl Drop for CborValue {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for CborValue {}
