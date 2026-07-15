// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// Error returned when decoding fails to conform to the DAG-CBOR profile.
///
/// Decoding fails closed: any malformed or non-canonical input yields one of
/// these variants rather than a partial or best-effort value.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum CborError {
    /// Extra bytes remain after decoding the top-level value.
    #[error("CBOR: trailing bytes after top-level value")]
    TrailingBytes,

    /// Input ended before the current value was fully decoded.
    #[error("CBOR: unexpected end of input")]
    UnexpectedEnd,

    /// The encoded input exceeded the absolute DAG-CBOR boundary cap.
    #[error("CBOR: input too large")]
    InputTooLarge,

    /// Encoding would exceed the absolute DAG-CBOR boundary cap.
    #[error("CBOR: encoded output too large")]
    OutputTooLarge,

    /// Computing an offset into the input would overflow.
    #[error("CBOR: offset arithmetic overflow")]
    OffsetOverflow,

    /// A declared length is too large to represent as a platform `usize`.
    #[error("CBOR: length does not fit in platform usize")]
    LengthTooLarge,

    /// An encoded integer falls outside the supported `i64` range.
    #[error("CBOR: integer is outside supported i64 range")]
    IntegerOutOfRange,

    /// An integer used a longer-than-minimal (non-canonical) encoding.
    #[error("CBOR: non-canonical integer encoding")]
    NonCanonicalInteger,

    /// The numeric argument (length/value) bytes were truncated.
    #[error("CBOR: truncated numeric argument")]
    TruncatedArgument,

    /// A byte string ended before its declared length was read.
    #[error("CBOR: truncated byte string")]
    TruncatedBytes,

    /// A text string contained bytes that are not valid UTF-8.
    #[error("CBOR: invalid UTF-8 string")]
    InvalidUtf8,

    /// A map key was not a text string, which DAG-CBOR requires.
    #[error("CBOR: map key must be text string")]
    MapKeyMustBeString,

    /// Map keys were not in the canonical (sorted) order DAG-CBOR requires.
    #[error("CBOR: map keys out of canonical order")]
    MapKeysOutOfOrder,

    /// A CBOR simple value not permitted by the DAG-CBOR profile was found.
    #[error("CBOR: simple value not allowed in DAG-CBOR")]
    DisallowedSimpleValue {
        /// The CBOR simple-value code that was rejected.
        value: u64,
    },

    /// A CBOR major type not permitted by the DAG-CBOR profile was found.
    #[error("CBOR: major type not allowed in DAG-CBOR")]
    DisallowedMajorType {
        /// The CBOR major-type number that was rejected.
        major: u8,
    },

    /// An oversized or indefinite-length additional-info value was found.
    #[error("CBOR: value too large or indefinite length not allowed")]
    UnsupportedAdditionalInfo,

    /// A declared array/map element count is larger than the remaining
    /// input could possibly satisfy. Rejected before allocating so an
    /// attacker-chosen length prefix cannot drive an out-of-memory abort.
    #[error("CBOR: declared container length exceeds remaining input")]
    ContainerLengthExceedsInput,

    /// Nesting exceeded [`MAX_NESTING_DEPTH`](crate::MAX_NESTING_DEPTH).
    /// Bounds recursion so a deeply nested input cannot overflow the stack.
    #[error("CBOR: nesting depth limit exceeded")]
    DepthExceeded,
}
