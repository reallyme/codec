// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use thiserror::Error;

/// Schema-generated reasons, without source bytes, offsets, or backend text.
pub use codec_proto::generated::proto::reallyme::codec::v1::CodecTlvErrorReason as ErrorReason;

/// A deterministic TLV framing failure.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("TLV framing failed: {reason}")]
pub struct TlvError {
    reason: ErrorReason,
}

impl TlvError {
    pub(super) const fn new(reason: ErrorReason) -> Self {
        Self { reason }
    }

    /// Return the fixed schema reason for rejection.
    #[must_use]
    pub const fn reason(self) -> ErrorReason {
        self.reason
    }
}

pub(super) fn overflow() -> TlvError {
    TlvError::new(ErrorReason::ArithmeticOverflow)
}

pub(super) fn size(value: usize) -> Result<u64, TlvError> {
    u64::try_from(value).map_err(|_| overflow())
}
