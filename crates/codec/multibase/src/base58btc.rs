// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use bs58::decode::Error as Bs58DecodeError;
use thiserror::Error;

/// Maximum accepted base58btc input length.
///
/// The underlying base58 conversion is not linear in input size. This cap keeps
/// untrusted byte and text inputs bounded while leaving headroom above the
/// largest currently supported multikey encodings.
pub const MAX_BASE58BTC_INPUT_LEN: usize = 8 * 1024;

/// Error returned when base58btc decoding fails.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Base58Error {
    /// The output buffer was too small to hold the decoded bytes.
    #[error("base58btc output buffer too small")]
    BufferTooSmall,
    /// Decoding failed for a reason not covered by the other variants.
    #[error("base58btc decode failed")]
    DecodeFailed,
    /// The input contained a character outside the base58btc alphabet.
    #[error("invalid base58btc character")]
    InvalidCharacter,
    /// The input contained a non-ASCII character.
    #[error("non-ascii base58btc character")]
    NonAsciiCharacter,
    /// The input exceeded the accepted base58btc text length.
    #[error("base58btc input too large")]
    InputTooLarge,
}

impl From<Bs58DecodeError> for Base58Error {
    fn from(value: Bs58DecodeError) -> Self {
        match value {
            Bs58DecodeError::BufferTooSmall => Self::BufferTooSmall,
            Bs58DecodeError::InvalidCharacter { .. } => Self::InvalidCharacter,
            Bs58DecodeError::NonAsciiCharacter { .. } => Self::NonAsciiCharacter,
            _ => Self::DecodeFailed,
        }
    }
}

/// Encodes bytes as a base58btc string.
pub fn base58btc_encode(bytes: &[u8]) -> Result<String, Base58Error> {
    if bytes.len() > MAX_BASE58BTC_INPUT_LEN {
        return Err(Base58Error::InputTooLarge);
    }
    Ok(bs58::encode(bytes).into_string())
}

/// Decodes a base58btc string into bytes.
///
/// Fails closed: returns an error on any invalid or non-ASCII character.
pub fn base58btc_decode(s: &str) -> Result<Vec<u8>, Base58Error> {
    if s.len() > MAX_BASE58BTC_INPUT_LEN {
        return Err(Base58Error::InputTooLarge);
    }
    bs58::decode(s).into_vec().map_err(Base58Error::from)
}
