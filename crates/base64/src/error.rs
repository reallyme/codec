// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use thiserror::Error;

/// Error returned when base64 decoding fails.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Base64Error {
    /// The input was not valid base64.
    #[error("invalid base64")]
    Invalid,
    /// Encoded text exceeded the supported resource limit.
    #[error("base64 input too large")]
    InputTooLarge,
    /// Decoded bytes would exceed the caller's output budget.
    #[error("base64 output too large")]
    OutputTooLarge,
    /// A supplied budget cannot describe an addressable buffer.
    #[error("invalid base64 limits")]
    InvalidLimits,
    /// Checked length arithmetic could not represent the decoded size.
    #[error("base64 length overflow")]
    LengthOverflow,
    /// The validated output reservation could not be satisfied.
    #[error("base64 allocation failed")]
    AllocationFailure,
}

impl Base64Error {
    /// The schema-owned failure reason shared by native and SDK adapters.
    #[must_use]
    pub const fn reason(
        &self,
    ) -> codec_proto::generated::proto::reallyme::codec::v1::CodecErrorReason {
        use codec_proto::generated::proto::reallyme::codec::v1::CodecErrorReason as R;
        match self {
            Self::Invalid => R::CODEC_ERROR_REASON_BASE_INVALID_BASE64,
            Self::InputTooLarge => R::CODEC_ERROR_REASON_BASE_INPUT_TOO_LARGE,
            Self::OutputTooLarge => R::CODEC_ERROR_REASON_BASE_OUTPUT_TOO_LARGE,
            Self::InvalidLimits => R::CODEC_ERROR_REASON_BASE_INVALID_LIMITS,
            Self::LengthOverflow => R::CODEC_ERROR_REASON_BASE_LENGTH_OVERFLOW,
            Self::AllocationFailure => R::CODEC_ERROR_REASON_BASE_ALLOCATION_FAILURE,
        }
    }
}
