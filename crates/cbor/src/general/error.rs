// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use thiserror::Error;

/// Fixed, payload-free reasons for general CBOR rejection.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[non_exhaustive]
pub enum ErrorReason {
    /// No top-level item was available.
    #[error("empty input")]
    EmptyInput,
    /// A header or payload ended prematurely.
    #[error("truncated item")]
    Truncated,
    /// Additional-information values 28 through 30 are reserved.
    #[error("reserved additional information")]
    ReservedAdditionalInformation,
    /// Integers and tags cannot have indefinite arguments.
    #[error("invalid indefinite argument")]
    InvalidIndefiniteArgument,
    /// A break occurred outside its permitted container position.
    #[error("unexpected break")]
    UnexpectedBreak,
    /// A simple value used a forbidden two-byte representation.
    #[error("invalid simple value encoding")]
    InvalidSimpleValue,
    /// A chunk was indefinite or had the wrong string type.
    #[error("invalid string chunk")]
    InvalidChunk,
    /// The selected profile permits definite lengths only.
    #[error("indefinite length is not allowed")]
    IndefiniteNotAllowed,
    /// An argument used more octets than necessary.
    #[error("non-preferred argument encoding")]
    NonPreferredArgument,
    /// A float could be represented exactly at a shorter width.
    #[error("non-preferred floating-point encoding")]
    NonPreferredFloat,
    /// The selected profile rejects floating-point values.
    #[error("floating-point values are not allowed")]
    FloatNotAllowed,
    /// The selected profile rejects infinity and NaN.
    #[error("non-finite floating-point value")]
    NonFiniteFloat,
    /// A text string or an individual text chunk was not UTF-8.
    #[error("invalid UTF-8")]
    InvalidUtf8,
    /// Two map keys are equivalent in the basic generic data model.
    #[error("duplicate map key")]
    DuplicateMapKey,
    /// Received keys violate the explicitly selected ordering rule.
    #[error("map key ordering violation")]
    MapKeyOrder,
    /// More bytes followed the requested complete item.
    #[error("trailing data")]
    TrailingData,
    /// An operation was applied to a different CBOR type.
    #[error("unexpected item type")]
    WrongType,
    /// Embedded-item validation requires the standard encoded-CBOR tag.
    #[error("unexpected tag")]
    WrongTag,
    /// Chunked contents do not form one contiguous borrowed payload.
    #[error("non-contiguous string contents")]
    NonContiguousString,
    /// A limit exceeded its supported hard ceiling.
    #[error("invalid resource limit")]
    InvalidLimit,
    /// The profile combination cannot provide the requested guarantee.
    #[error("unsupported profile combination")]
    UnsupportedPolicy,
    /// Input exceeded the configured transport cap.
    #[error("input limit exceeded")]
    InputLimit,
    /// Output would exceed the configured cap.
    #[error("output limit exceeded")]
    OutputLimit,
    /// Too many nested containers or tags were opened.
    #[error("nesting limit exceeded")]
    DepthLimit,
    /// Too many structural nodes were encountered.
    #[error("node limit exceeded")]
    NodeLimit,
    /// A container has too many entries or chunks.
    #[error("container entry limit exceeded")]
    EntryLimit,
    /// Aggregate byte-string contents exceeded the configured cap.
    #[error("byte-string content limit exceeded")]
    ByteStringLimit,
    /// Aggregate text contents exceeded the configured cap.
    #[error("text content limit exceeded")]
    TextLimit,
    /// Parsing or semantic comparison exhausted its explicit work budget.
    #[error("validation work limit exceeded")]
    WorkLimit,
    /// A declared container cannot fit in the remaining received input.
    #[error("container declaration exceeds remaining input")]
    ContainerExceedsInput,
    /// A size or offset calculation could not be represented.
    #[error("length arithmetic overflow")]
    ArithmeticOverflow,
    /// A bounded output allocation could not be reserved.
    #[error("bounded allocation failed")]
    AllocationFailure,
}

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
