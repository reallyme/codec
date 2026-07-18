// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// Errors raised by deterministic-CBOR profile construction and validation.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeterministicCborProfileError {
    /// Negative integer variants must remain strictly negative so each integer
    /// has exactly one semantic representation.
    #[error("deterministic CBOR: negative integer must be less than zero")]
    NegativeIntegerMustBeNegative,
}

/// Error returned by deterministic generic-CBOR encoding and decoding.
///
/// Variants intentionally carry no caller text, byte strings, or raw input.
/// This keeps FFI and SDK error mapping deterministic and prevents accidental
/// PII disclosure in logs or telemetry.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeterministicCborError {
    /// Extra bytes remain after decoding the top-level value.
    #[error("deterministic CBOR: trailing bytes after top-level value")]
    TrailingBytes,

    /// Input ended before the current value was fully decoded.
    #[error("deterministic CBOR: unexpected end of input")]
    UnexpectedEnd,

    /// The encoded input exceeded the deterministic-CBOR boundary cap.
    #[error("deterministic CBOR: input too large")]
    InputTooLarge,

    /// Encoding would exceed the deterministic-CBOR output cap.
    #[error("deterministic CBOR: encoded output too large")]
    OutputTooLarge,

    /// Computing a buffer offset, length, or capacity would overflow.
    #[error("deterministic CBOR: offset arithmetic overflow")]
    OffsetOverflow,

    /// A declared length does not fit in the current platform's `usize`.
    #[error("deterministic CBOR: length does not fit in platform usize")]
    LengthTooLarge,

    /// A negative integer is outside the supported `i64::MIN..=-1` range.
    #[error("deterministic CBOR: negative integer outside supported range")]
    NegativeIntegerOutOfRange,

    /// A negative-integer semantic value was zero or positive.
    #[error("deterministic CBOR: negative integer must be less than zero")]
    NegativeIntegerMustBeNegative,

    /// An integer or length used a longer-than-minimal encoding.
    #[error("deterministic CBOR: non-canonical integer or length encoding")]
    NonCanonicalInteger,

    /// The numeric argument bytes were truncated.
    #[error("deterministic CBOR: truncated numeric argument")]
    TruncatedArgument,

    /// A byte or text string ended before its declared length was read.
    #[error("deterministic CBOR: truncated byte or text string")]
    TruncatedBytes,

    /// A text string contained bytes that are not valid UTF-8.
    #[error("deterministic CBOR: invalid UTF-8 string")]
    InvalidUtf8,

    /// A map key used a type outside the supported deterministic-CBOR key set.
    #[error("deterministic CBOR: unsupported map key type")]
    UnsupportedMapKeyType,

    /// A map contained the same semantic key more than once.
    #[error("deterministic CBOR: duplicate map key")]
    DuplicateMapKey,

    /// Map keys were not in RFC 8949 deterministic order.
    #[error("deterministic CBOR: map keys out of deterministic order")]
    MapKeysOutOfOrder,

    /// A CBOR simple value outside false, true, or null was found.
    #[error("deterministic CBOR: unsupported simple value")]
    UnsupportedSimpleValue,

    /// A CBOR major type outside the supported profile was found.
    #[error("deterministic CBOR: unsupported major type")]
    UnsupportedMajorType,

    /// Indefinite length or another unsupported additional-info value appeared.
    #[error("deterministic CBOR: unsupported additional information")]
    UnsupportedAdditionalInfo,

    /// Nesting exceeded the deterministic-CBOR profile limit.
    #[error("deterministic CBOR: nesting depth limit exceeded")]
    DepthExceeded,

    /// The semantic node count exceeded the deterministic-CBOR profile limit.
    #[error("deterministic CBOR: node limit exceeded")]
    NodeLimitExceeded,

    /// A single array or map exceeded the per-container entry limit.
    #[error("deterministic CBOR: container entry limit exceeded")]
    ContainerEntriesExceeded,

    /// Aggregate UTF-8 text bytes exceeded the deterministic-CBOR profile cap.
    #[error("deterministic CBOR: aggregate text byte limit exceeded")]
    AggregateTextBytesExceeded,

    /// Aggregate byte-string bytes exceeded the deterministic-CBOR profile cap.
    #[error("deterministic CBOR: aggregate byte-string limit exceeded")]
    AggregateByteStringBytesExceeded,

    /// A declared container length is larger than the remaining input can hold.
    #[error("deterministic CBOR: declared container length exceeds remaining input")]
    ContainerLengthExceedsInput,

    /// Encoder preflight and emission disagreed, which indicates an internal bug.
    #[error("deterministic CBOR: preflight length mismatch")]
    PreflightLengthMismatch,

    /// A bounded internal owner could not reserve its required allocation.
    #[error("deterministic CBOR: bounded allocation failed")]
    AllocationFailure,
}

impl From<DeterministicCborProfileError> for DeterministicCborError {
    fn from(error: DeterministicCborProfileError) -> Self {
        match error {
            DeterministicCborProfileError::NegativeIntegerMustBeNegative => {
                Self::NegativeIntegerMustBeNegative
            }
        }
    }
}
