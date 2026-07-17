// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

/// A decoded DAG-CBOR value.
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
