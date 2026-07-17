// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// Error returned when canonical lowercase hex decoding fails.
#[derive(Debug, Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum HexError {
    /// Hex input must contain two lowercase hexadecimal characters per byte.
    #[error("odd-length hex input")]
    OddLength,

    /// The decoder accepts only canonical lowercase hex.
    #[error("uppercase hex input")]
    Uppercase,

    /// The input contained a byte outside `0-9` or `a-f`.
    #[error("invalid hex input")]
    InvalidCharacter,
}
