// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{DerError, DerErrorReason};

/// Hard input limit: one MiB, independent of attacker-supplied length headers.
pub const MAX_DER_INPUT_LEN: usize = 1_048_576;
/// Hard bound on simultaneously open SEQUENCE/SET OF containers.
pub const MAX_DER_NESTING_DEPTH: usize = 32;
/// Hard bound on all values, including containers, in one document.
pub const MAX_DER_ELEMENTS: usize = 4_096;
/// Hard bound on unsigned magnitude bytes or signed INTEGER content bytes.
pub const MAX_DER_INTEGER_LEN: usize = 4_096;

/// Immutable resource limits. Caller limits can only tighten the hard limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DerLimits {
    pub(super) input_len: usize,
    pub(super) nesting_depth: usize,
    pub(super) elements: usize,
    pub(super) integer_len: usize,
}

impl DerLimits {
    /// Configure input bytes, container depth, total elements, and INTEGER bytes.
    ///
    /// Unsigned INTEGER limits apply to the returned magnitude. One additional
    /// leading sign octet is permitted only when required by DER. Signed INTEGER
    /// limits apply to the canonical two's-complement contents. Syntax-layer zero
    /// is valid and has the one-byte magnitude `[0]`.
    pub fn new(
        input_len: usize,
        nesting_depth: usize,
        elements: usize,
        integer_len: usize,
    ) -> Result<Self, DerError> {
        let invalid = || DerError::new(DerErrorReason::InvalidLimits);
        Self::try_from(
            &codec_proto::generated::proto::reallyme::codec::v1::CodecDerLimits {
                input_len: Some(u64::try_from(input_len).map_err(|_| invalid())?),
                nesting_depth: Some(u64::try_from(nesting_depth).map_err(|_| invalid())?),
                elements: Some(u64::try_from(elements).map_err(|_| invalid())?),
                integer_len: Some(u64::try_from(integer_len).map_err(|_| invalid())?),
            },
        )
    }
}

impl Default for DerLimits {
    fn default() -> Self {
        Self {
            input_len: MAX_DER_INPUT_LEN,
            nesting_depth: MAX_DER_NESTING_DEPTH,
            elements: MAX_DER_ELEMENTS,
            integer_len: MAX_DER_INTEGER_LEN,
        }
    }
}

impl TryFrom<&codec_proto::generated::proto::reallyme::codec::v1::CodecDerLimits> for DerLimits {
    type Error = DerError;
    fn try_from(
        value: &codec_proto::generated::proto::reallyme::codec::v1::CodecDerLimits,
    ) -> Result<Self, Self::Error> {
        fn limit(value: Option<u64>, maximum: usize) -> Result<usize, DerError> {
            let value = match value {
                None => maximum,
                Some(value) => usize::try_from(value)
                    .map_err(|_| DerError::new(DerErrorReason::InvalidLimits))?,
            };
            if value == 0 || value > maximum {
                return Err(DerError::new(DerErrorReason::InvalidLimits));
            }
            Ok(value)
        }
        Ok(Self {
            input_len: limit(value.input_len, MAX_DER_INPUT_LEN)?,
            nesting_depth: limit(value.nesting_depth, MAX_DER_NESTING_DEPTH)?,
            elements: limit(value.elements, MAX_DER_ELEMENTS)?,
            integer_len: limit(value.integer_len, MAX_DER_INTEGER_LEN)?,
        })
    }
}
