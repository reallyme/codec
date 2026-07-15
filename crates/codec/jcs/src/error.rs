// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// Error returned when JSON canonicalization (JCS) fails.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum JcsError {
    /// A number was NaN or infinite, which JCS does not permit.
    #[error("jcs: non-finite number is not allowed")]
    NonFiniteNumber,

    /// Serializing the value to canonical JSON failed.
    #[error("jcs: JSON serialization error")]
    SerializationError,

    /// Nesting exceeded [`MAX_NESTING_DEPTH`](crate::MAX_NESTING_DEPTH).
    /// A defense-in-depth bound in case a caller builds or deserializes a
    /// `serde_json::Value` without the parser's own depth limit.
    #[error("jcs: nesting depth limit exceeded")]
    DepthExceeded,
}
