// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// Error returned when JSON canonicalization (JCS) fails.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum JcsError {
    /// The supplied bytes were not one complete valid JSON text.
    #[error("jcs: invalid JSON text")]
    InvalidJson,

    /// A JSON object declared the same member name more than once.
    #[error("jcs: duplicate object member name")]
    DuplicateProperty,

    /// A number was NaN or infinite, which JCS does not permit.
    #[error("jcs: non-finite number is not allowed")]
    NonFiniteNumber,

    /// An exactly represented integer was outside the interoperable range.
    #[error("jcs: integer is outside the interoperable IEEE-754 range")]
    IntegerOutsideInteroperableRange,

    /// Serializing the value to canonical JSON failed.
    #[error("jcs: JSON serialization error")]
    SerializationError,

    /// Nesting exceeded [`MAX_NESTING_DEPTH`](crate::MAX_NESTING_DEPTH).
    /// A defense-in-depth bound in case a caller builds or deserializes a
    /// `serde_json::Value` without the parser's own depth limit.
    #[error("jcs: nesting depth limit exceeded")]
    DepthExceeded,
}
