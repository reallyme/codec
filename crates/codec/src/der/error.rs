// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use thiserror::Error;

/// Stable, payload-free reason for rejecting a DER input or policy.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[non_exhaustive]
pub enum DerErrorReason {
    /// A document must contain one value.
    #[error("empty document")]
    EmptyInput,
    /// Limits must be nonzero and must not exceed the hard limits.
    #[error("invalid limits")]
    InvalidLimits,
    /// The document exceeded its configured input limit.
    #[error("input limit exceeded")]
    InputTooLarge,
    /// Constructed nesting exceeded its configured limit.
    #[error("nesting limit exceeded")]
    NestingTooDeep,
    /// The shared document element budget was exhausted.
    #[error("element limit exceeded")]
    TooManyElements,
    /// INTEGER contents exceeded their configured magnitude limit.
    #[error("integer limit exceeded")]
    IntegerTooLarge,
    /// The input ended before a declared field was complete.
    #[error("truncated input")]
    TruncatedInput,
    /// Indefinite lengths are forbidden in DER.
    #[error("indefinite length")]
    IndefiniteLength,
    /// A value used a nonminimal or otherwise noncanonical encoding.
    #[error("noncanonical encoding")]
    NonCanonicalEncoding,
    /// The selected unsigned or signed INTEGER representation was invalid.
    #[error("invalid integer")]
    InvalidInteger,
    /// The tag was malformed or outside the supported tag profile.
    #[error("invalid tag")]
    InvalidTag,
    /// The input tag did not match the selected schema field.
    #[error("unexpected tag")]
    UnexpectedTag,
    /// A field length was invalid for its ASN.1 type.
    #[error("invalid length")]
    InvalidLength,
    /// The field contents did not represent a valid value of their type.
    #[error("invalid value")]
    InvalidValue,
    /// A document or container had unconsumed data or multiple root values.
    #[error("trailing data")]
    TrailingData,
    /// SET OF elements were not in lexicographic DER encoding order.
    #[error("noncanonical set ordering")]
    SetOrdering,
    /// A bounded length or counter could not be represented.
    #[error("length arithmetic overflow")]
    ArithmeticOverflow,
}

/// Deterministic DER error containing no input, offsets, or backend text.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("DER decoding failed: {reason}")]
pub struct DerError {
    reason: DerErrorReason,
}

impl DerError {
    /// Construct a typed error without attaching document contents.
    #[must_use]
    pub const fn new(reason: DerErrorReason) -> Self {
        Self { reason }
    }

    /// Return the stable reason for rejection.
    #[must_use]
    pub const fn reason(self) -> DerErrorReason {
        self.reason
    }
}

pub(super) fn map_backend(error: der_core::Error) -> DerError {
    use der_core::{ErrorKind, Tag};
    let reason = match error.kind() {
        ErrorKind::Incomplete { .. } => DerErrorReason::TruncatedInput,
        ErrorKind::IndefiniteLength => DerErrorReason::IndefiniteLength,
        ErrorKind::Noncanonical { .. } => DerErrorReason::NonCanonicalEncoding,
        ErrorKind::Value { tag: Tag::Integer } => DerErrorReason::InvalidInteger,
        ErrorKind::TagUnexpected { .. } => DerErrorReason::UnexpectedTag,
        ErrorKind::TagNumberInvalid | ErrorKind::TagUnknown { .. } => DerErrorReason::InvalidTag,
        ErrorKind::Length { .. } => DerErrorReason::InvalidLength,
        ErrorKind::TrailingData { .. } => DerErrorReason::TrailingData,
        ErrorKind::SetOrdering => DerErrorReason::SetOrdering,
        ErrorKind::Overflow | ErrorKind::Overlength => DerErrorReason::ArithmeticOverflow,
        _ => DerErrorReason::InvalidValue,
    };
    DerError::new(reason)
}
