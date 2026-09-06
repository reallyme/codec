// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use zeroize::Zeroize;

/// A decoded DAG-CBOR value.
///
/// Values retain the movable public enum API. Callers handling sensitive
/// documents can wrap them in [`zeroize::Zeroizing`] or call [`Zeroize::zeroize`]
/// when finished; codec-owned temporary values use wiping owners internally.
#[derive(Debug, Clone, PartialEq)]
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

// Preserve the public movable enum API while allowing adapters and callers to
// install a Zeroizing owner for identity-bearing documents.
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
