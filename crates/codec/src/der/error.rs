// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use thiserror::Error;

/// Schema-generated, fixed reasons for the compatibility error facade.
pub use codec_proto::generated::proto::reallyme::codec::v1::CodecDerErrorReason as DerErrorReason;

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
