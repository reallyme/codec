// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use thiserror::Error;

/// Schema-generated, fixed reasons for the compatibility error facade.
pub use codec_proto::generated::proto::reallyme::codec::v1::CodecGeneralCborErrorReason as ErrorReason;

/// General CBOR error with no input, offsets, strings, or dependency diagnostics.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("general CBOR: {reason}")]
pub struct CborError {
    reason: ErrorReason,
}

impl CborError {
    /// Construct a fixed typed error.
    #[must_use]
    pub const fn new(reason: ErrorReason) -> Self {
        Self { reason }
    }

    /// Return the stable rejection reason.
    #[must_use]
    pub const fn reason(self) -> ErrorReason {
        self.reason
    }
}

pub(super) const fn error(reason: ErrorReason) -> CborError {
    CborError::new(reason)
}
