// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::fmt;
use thiserror::Error;

/// Canonical schema-owned reasons, without input data or backend diagnostics.
pub use codec_proto::generated::proto::reallyme::codec::v1::CodecErrorReason as ErrorReason;

/// A fixed typed JSON failure. Diagnostic construction never copies input.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("guarded JSON decoding failed")]
pub struct JsonError {
    reason: ErrorReason,
}

impl JsonError {
    pub(super) const fn new(reason: ErrorReason) -> Self {
        Self { reason }
    }

    /// Return the canonical reason identifying the failure.
    #[must_use]
    pub const fn reason(self) -> ErrorReason {
        self.reason
    }
}

impl serde::de::Error for JsonError {
    fn custom<T: fmt::Display>(_message: T) -> Self {
        // Visitors may supply field names or values. Preserve only a fixed
        // schema reason; never format or retain the visitor's diagnostic.
        Self::new(ErrorReason::CODEC_ERROR_REASON_CANONICAL_JSON_INVALID_TARGET)
    }
}

pub(super) const fn invalid_json() -> JsonError {
    JsonError::new(ErrorReason::CODEC_ERROR_REASON_CANONICAL_INVALID_JSON)
}

pub(super) const fn overflow() -> JsonError {
    JsonError::new(ErrorReason::CODEC_ERROR_REASON_CANONICAL_JSON_ARITHMETIC_OVERFLOW)
}
