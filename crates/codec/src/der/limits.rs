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
        if input_len == 0
            || input_len > MAX_DER_INPUT_LEN
            || nesting_depth == 0
            || nesting_depth > MAX_DER_NESTING_DEPTH
            || elements == 0
            || elements > MAX_DER_ELEMENTS
            || integer_len == 0
            || integer_len > MAX_DER_INTEGER_LEN
        {
            return Err(DerError::new(DerErrorReason::InvalidLimits));
        }
        Ok(Self {
            input_len,
            nesting_depth,
            elements,
            integer_len,
        })
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
