// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! ReallyMe codec protobuf error envelopes with generated Buffa bindings.

#[cfg(feature = "generated")]
use buffa::{DecodeOptions, EnumValue, Enumeration, Message};
#[cfg(feature = "generated")]
use serde::de::DeserializeOwned;
#[cfg(feature = "generated")]
use thiserror::Error;
#[cfg(feature = "generated")]
use zeroize::{Zeroize, Zeroizing};

/// Generated protobuf boundary.
pub mod generated;

/// Maximum accepted binary protobuf message size at codec wire boundaries.
#[cfg(feature = "generated")]
pub const MAX_CODEC_PROTO_MESSAGE_BYTES: usize = 1024 * 1024;

/// Maximum accepted `CodecError` envelope size at codec wire boundaries.
#[cfg(feature = "generated")]
pub const MAX_CODEC_PROTO_ERROR_ENVELOPE_BYTES: usize = 4096;

/// Maximum accepted proto3 JSON message size at codec wire boundaries.
#[cfg(feature = "generated")]
pub const MAX_CODEC_PROTO_JSON_BYTES: usize = 1_572_864;

#[cfg(feature = "generated")]
const CODEC_PROTO_RECURSION_LIMIT: u32 = 64;

#[cfg(feature = "generated")]
const CODEC_PROTO_UNKNOWN_FIELD_LIMIT: usize = 0;

#[cfg(feature = "generated")]
/// Maximum framing overhead added around a bounded codec result payload.
#[cfg(feature = "generated")]
const MAX_CODEC_PROTO_ENVELOPE_OVERHEAD_BYTES: usize = 16;

/// Maximum encoded size of a codec result envelope.
///
/// This is the single source of truth consumed by native adapters before they
/// allocate memory from a provider-reported result length.
#[cfg(feature = "generated")]
pub const MAX_CODEC_PROTO_RESULT_ENVELOPE_BYTES: usize =
    max_codec_proto_result_envelope_bytes_const();

#[cfg(feature = "generated")]
const fn max_codec_proto_result_envelope_bytes_const() -> usize {
    match MAX_CODEC_PROTO_MESSAGE_BYTES.checked_add(MAX_CODEC_PROTO_ENVELOPE_OVERHEAD_BYTES) {
        Some(limit) => limit,
        // The constants above are compile-time protocol limits. Returning zero
        // makes a future overflow fail closed in every consuming adapter.
        None => 0,
    }
}

// This assertion is evaluated by the compiler and has no runtime panic path.
// It prevents a future protocol-limit edit from silently disabling the cap.
#[cfg(feature = "generated")]
const _: () = assert!(MAX_CODEC_PROTO_RESULT_ENVELOPE_BYTES != 0);

#[cfg(feature = "generated")]
use generated::proto::reallyme::codec::v1::{
    __buffa::oneof::codec_error::Error as CodecErrorBranchProto, CodecBackendError,
    CodecBaseEncodingError, CodecBoundaryError, CodecCanonicalizationError, CodecError,
    CodecErrorReason, CodecMultiformatError, CodecPemError, CodecProtoResultEnvelope,
    CodecProtoResultStatus,
};

/// Stable codec wire-error branch.
#[cfg(feature = "generated")]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub enum CodecWireErrorBranch {
    /// Base encodings such as base64, base64url, base58btc, and hex.
    BaseEncoding,
    /// PEM armor and DER envelope errors.
    Pem,
    /// Multibase, multicodec, and multikey errors.
    Multiformat,
    /// CBOR, DAG-CBOR, JCS, and JSON canonicalization errors.
    Canonicalization,
    /// Serialization, protobuf, dispatch, FFI, WASM, and internal failures.
    Backend,
    /// Malformed, oversized, or incomplete caller-controlled wire requests.
    Boundary,
}

/// Stable status of a codec protobuf result payload.
#[cfg(feature = "generated")]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub enum CodecProtoStatus {
    /// Payload contains the operation-specific result message.
    Result,
    /// Payload contains a structured [`CodecError`].
    CodecError,
}

/// Owned result returned by the executable protobuf adapter lane.
#[cfg(feature = "generated")]
pub struct CodecProtoResult {
    status: CodecProtoStatus,
    bytes: Zeroizing<Vec<u8>>,
}

#[cfg(feature = "generated")]
impl CodecProtoResult {
    /// Constructs an operation result.
    #[must_use]
    pub fn result(bytes: Vec<u8>) -> Self {
        Self {
            status: CodecProtoStatus::Result,
            bytes: Zeroizing::new(bytes),
        }
    }

    /// Encodes an operation-specific result message.
    #[must_use]
    pub fn from_message<M: Message>(message: &M) -> Self {
        Self {
            status: CodecProtoStatus::Result,
            bytes: encode_protobuf(message),
        }
    }

    /// Constructs a structured codec error result.
    #[must_use]
    pub fn codec_error(error: CodecWireError) -> Self {
        Self {
            status: CodecProtoStatus::CodecError,
            bytes: codec_error_bytes(error),
        }
    }

    /// Returns the payload status.
    #[must_use]
    pub const fn status(&self) -> CodecProtoStatus {
        self.status
    }

    /// Returns the protobuf payload bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    /// Returns the payload length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns whether the payload is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

/// Validated public codec wire error.
#[cfg(feature = "generated")]
#[derive(Debug, Clone, Copy, Eq, Error, PartialEq)]
#[error("codec wire boundary error")]
pub struct CodecWireError {
    branch: CodecWireErrorBranch,
    reason: CodecErrorReason,
}

/// Error returned when constructing a codec wire error with an invalid reason.
#[cfg(feature = "generated")]
#[derive(Debug, Clone, Copy, Eq, Error, PartialEq)]
#[non_exhaustive]
pub enum CodecWireErrorConstructionError {
    /// The reason is unspecified, unknown, or belongs to a different branch.
    #[error("codec error reason does not belong to the selected branch")]
    BranchReasonMismatch,
}

/// Result alias for codec protobuf boundary helpers.
#[cfg(feature = "generated")]
pub type CodecWireResult<T> = Result<T, CodecWireError>;

#[cfg(feature = "generated")]
impl CodecWireError {
    /// Returns the stable error branch.
    #[must_use]
    pub const fn branch(self) -> CodecWireErrorBranch {
        self.branch
    }

    /// Returns the stable branch-specific reason.
    #[must_use]
    pub const fn reason(self) -> CodecErrorReason {
        self.reason
    }

    /// Constructs a public wire error only when branch and reason agree.
    ///
    /// # Errors
    ///
    /// Returns [`CodecWireErrorConstructionError::BranchReasonMismatch`] when
    /// the reason is unspecified, unknown, or owned by another branch.
    pub fn try_new(
        branch: CodecWireErrorBranch,
        reason: CodecErrorReason,
    ) -> Result<Self, CodecWireErrorConstructionError> {
        if !reason_is_valid_for_branch(branch, reason) {
            return Err(CodecWireErrorConstructionError::BranchReasonMismatch);
        }
        Ok(Self { branch, reason })
    }

    /// Internal constructor for crate-owned known-good mappings.
    #[must_use]
    const fn known_good(branch: CodecWireErrorBranch, reason: CodecErrorReason) -> Self {
        Self { branch, reason }
    }

    /// Returns the deterministic malformed-protobuf boundary error.
    #[must_use]
    pub const fn malformed_protobuf() -> Self {
        Self::known_good(
            CodecWireErrorBranch::Boundary,
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_PROTOBUF,
        )
    }
}

/// Encodes a protobuf message with Buffa.
#[cfg(feature = "generated")]
#[must_use]
pub fn encode_protobuf<M: Message>(message: &M) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(message.encode_to_vec())
}

/// Decodes a bounded protobuf message from untrusted bytes.
///
/// # Errors
///
/// Returns a boundary wire error when input exceeds the size limit or fails
/// protobuf decoding.
#[cfg(feature = "generated")]
pub fn decode_protobuf<M: Message>(bytes: &[u8]) -> CodecWireResult<M> {
    decode_protobuf_with_limit(bytes, MAX_CODEC_PROTO_MESSAGE_BYTES)
}

/// Decodes a generated protobuf message from proto3-compatible JSON bytes.
///
/// The decoded message is immediately re-encoded to protobuf so compact JSON
/// that expands past the binary protobuf cap is rejected at the same boundary
/// as native protobuf input.
///
/// # Errors
///
/// Returns a boundary wire error for oversized or malformed
/// JSON and for JSON messages whose binary encoding exceeds the protobuf cap.
#[cfg(feature = "generated")]
pub fn decode_json<M: DeserializeOwned + Message>(bytes: &[u8]) -> CodecWireResult<M> {
    if bytes.len() > MAX_CODEC_PROTO_JSON_BYTES {
        return Err(resource_limit_error());
    }

    let message: M = serde_json::from_slice(bytes).map_err(|_| {
        CodecWireError::known_good(
            CodecWireErrorBranch::Boundary,
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_JSON,
        )
    })?;
    let encoded = encode_protobuf(&message);
    if encoded.len() > MAX_CODEC_PROTO_MESSAGE_BYTES {
        return Err(resource_limit_error());
    }
    Ok(message)
}

/// Builds the structured `CodecError` protobuf message for a validated error.
#[cfg(feature = "generated")]
#[must_use]
pub fn codec_error(error: CodecWireError) -> CodecError {
    let reason = EnumValue::from(error.reason());
    let branch = match error.branch() {
        CodecWireErrorBranch::BaseEncoding => {
            CodecErrorBranchProto::BaseEncoding(Box::new(CodecBaseEncodingError {
                reason,
                __buffa_unknown_fields: Default::default(),
            }))
        }
        CodecWireErrorBranch::Pem => CodecErrorBranchProto::Pem(Box::new(CodecPemError {
            reason,
            __buffa_unknown_fields: Default::default(),
        })),
        CodecWireErrorBranch::Multiformat => {
            CodecErrorBranchProto::Multiformat(Box::new(CodecMultiformatError {
                reason,
                __buffa_unknown_fields: Default::default(),
            }))
        }
        CodecWireErrorBranch::Canonicalization => {
            CodecErrorBranchProto::Canonicalization(Box::new(CodecCanonicalizationError {
                reason,
                __buffa_unknown_fields: Default::default(),
            }))
        }
        CodecWireErrorBranch::Backend => {
            CodecErrorBranchProto::Backend(Box::new(CodecBackendError {
                reason,
                __buffa_unknown_fields: Default::default(),
            }))
        }
        CodecWireErrorBranch::Boundary => {
            CodecErrorBranchProto::Boundary(Box::new(CodecBoundaryError {
                reason,
                __buffa_unknown_fields: Default::default(),
            }))
        }
    };

    CodecError {
        error: Some(branch),
        __buffa_unknown_fields: Default::default(),
    }
}

/// Encodes a structured `CodecError` as protobuf bytes.
#[cfg(feature = "generated")]
#[must_use]
pub fn codec_error_bytes(error: CodecWireError) -> Zeroizing<Vec<u8>> {
    encode_protobuf(&codec_error(error))
}

/// Decodes and validates a public `CodecError` payload.
///
/// # Errors
///
/// Returns a deterministic malformed-protobuf canonicalization error when the
/// payload does not contain exactly one decodable branch with a concrete reason
/// valid for that branch.
#[cfg(feature = "generated")]
pub fn decode_codec_error_payload(payload: &[u8]) -> CodecWireResult<CodecWireError> {
    let error =
        decode_protobuf_with_limit::<CodecError>(payload, MAX_CODEC_PROTO_ERROR_ENVELOPE_BYTES)?;
    let (branch, reason) = match error.error {
        Some(CodecErrorBranchProto::BaseEncoding(error)) => {
            (CodecWireErrorBranch::BaseEncoding, error.reason)
        }
        Some(CodecErrorBranchProto::Pem(error)) => (CodecWireErrorBranch::Pem, error.reason),
        Some(CodecErrorBranchProto::Multiformat(error)) => {
            (CodecWireErrorBranch::Multiformat, error.reason)
        }
        Some(CodecErrorBranchProto::Canonicalization(error)) => {
            (CodecWireErrorBranch::Canonicalization, error.reason)
        }
        Some(CodecErrorBranchProto::Backend(error)) => {
            (CodecWireErrorBranch::Backend, error.reason)
        }
        Some(CodecErrorBranchProto::Boundary(error)) => {
            (CodecWireErrorBranch::Boundary, error.reason)
        }
        None => return Err(malformed_protobuf_error()),
    };

    match reason.as_known() {
        Some(reason) if reason_is_valid_for_branch(branch, reason) => {
            Ok(CodecWireError { branch, reason })
        }
        Some(CodecErrorReason::CODEC_ERROR_REASON_UNSPECIFIED) | Some(_) | None => {
            Err(malformed_protobuf_error())
        }
    }
}

/// Encodes a result/error payload into the single bounded binary envelope.
///
/// # Errors
///
/// Returns a structured resource-limit result when the payload or final
/// envelope exceeds the public protobuf boundary.
#[cfg(feature = "generated")]
pub fn encode_proto_result_envelope(
    result: &CodecProtoResult,
) -> Result<Zeroizing<Vec<u8>>, CodecProtoResult> {
    if result.len() > MAX_CODEC_PROTO_MESSAGE_BYTES {
        return Err(resource_limit_result());
    }

    let mut envelope = proto_result_envelope_from_result(result);
    let encoded = encode_protobuf(&envelope);
    envelope.payload.zeroize();
    if encoded.len() > max_codec_proto_result_envelope_bytes()? {
        return Err(resource_limit_result());
    }
    Ok(encoded)
}

/// Encodes a result envelope with a deterministic structured fallback.
#[cfg(feature = "generated")]
#[must_use]
pub fn encode_proto_result_envelope_or_error(result: &CodecProtoResult) -> Zeroizing<Vec<u8>> {
    match encode_proto_result_envelope(result) {
        Ok(encoded) => encoded,
        Err(error) => encode_proto_result_envelope_unchecked(&error),
    }
}

/// Decodes and validates a bounded binary result envelope.
///
/// # Errors
///
/// Returns a structured codec result when the envelope is malformed,
/// oversized, has no concrete status, or contains an invalid error payload.
#[cfg(feature = "generated")]
pub fn decode_proto_result_envelope(bytes: &[u8]) -> Result<CodecProtoResult, CodecProtoResult> {
    let max_bytes = max_codec_proto_result_envelope_bytes()?;
    if bytes.len() > max_bytes {
        return Err(resource_limit_result());
    }
    let envelope = decode_protobuf_with_limit::<CodecProtoResultEnvelope>(bytes, max_bytes)
        .map_err(CodecProtoResult::codec_error)?;
    codec_proto_result_from_envelope(envelope)
}

#[cfg(feature = "generated")]
fn codec_proto_result_from_envelope(
    mut envelope: CodecProtoResultEnvelope,
) -> Result<CodecProtoResult, CodecProtoResult> {
    if envelope.payload.len() > MAX_CODEC_PROTO_MESSAGE_BYTES {
        envelope.payload.zeroize();
        return Err(resource_limit_result());
    }

    let status = match envelope.status.as_known() {
        Some(CodecProtoResultStatus::CODEC_PROTO_RESULT_STATUS_RESULT) => CodecProtoStatus::Result,
        Some(CodecProtoResultStatus::CODEC_PROTO_RESULT_STATUS_CODEC_ERROR) => {
            if decode_codec_error_payload(&envelope.payload).is_err() {
                envelope.payload.zeroize();
                return Err(CodecProtoResult::codec_error(malformed_protobuf_error()));
            }
            CodecProtoStatus::CodecError
        }
        Some(CodecProtoResultStatus::CODEC_PROTO_RESULT_STATUS_UNSPECIFIED) | None => {
            envelope.payload.zeroize();
            return Err(CodecProtoResult::codec_error(malformed_protobuf_error()));
        }
    };

    let bytes = core::mem::take(&mut envelope.payload);
    Ok(CodecProtoResult {
        status,
        bytes: Zeroizing::new(bytes),
    })
}

#[cfg(feature = "generated")]
fn proto_result_envelope_from_result(result: &CodecProtoResult) -> CodecProtoResultEnvelope {
    let status = match result.status {
        CodecProtoStatus::Result => CodecProtoResultStatus::CODEC_PROTO_RESULT_STATUS_RESULT,
        CodecProtoStatus::CodecError => {
            CodecProtoResultStatus::CODEC_PROTO_RESULT_STATUS_CODEC_ERROR
        }
    };
    CodecProtoResultEnvelope {
        status: EnumValue::from(status),
        payload: result.bytes().to_vec(),
        __buffa_unknown_fields: Default::default(),
    }
}

#[cfg(feature = "generated")]
fn encode_proto_result_envelope_unchecked(result: &CodecProtoResult) -> Zeroizing<Vec<u8>> {
    let mut envelope = proto_result_envelope_from_result(result);
    let encoded = encode_protobuf(&envelope);
    envelope.payload.zeroize();
    encoded
}

#[cfg(feature = "generated")]
fn max_codec_proto_result_envelope_bytes() -> Result<usize, CodecProtoResult> {
    if MAX_CODEC_PROTO_RESULT_ENVELOPE_BYTES == 0 {
        return Err(CodecProtoResult::codec_error(CodecWireError::known_good(
            CodecWireErrorBranch::Backend,
            CodecErrorReason::CODEC_ERROR_REASON_BACKEND_INTERNAL,
        )));
    }
    Ok(MAX_CODEC_PROTO_RESULT_ENVELOPE_BYTES)
}

#[cfg(feature = "generated")]
fn resource_limit_result() -> CodecProtoResult {
    CodecProtoResult::codec_error(resource_limit_error())
}

#[cfg(feature = "generated")]
fn decode_protobuf_with_limit<M: Message>(bytes: &[u8], max_bytes: usize) -> CodecWireResult<M> {
    if bytes.len() > max_bytes {
        return Err(resource_limit_error());
    }

    DecodeOptions::new()
        .with_recursion_limit(CODEC_PROTO_RECURSION_LIMIT)
        .with_max_message_size(max_bytes)
        .with_unknown_field_limit(CODEC_PROTO_UNKNOWN_FIELD_LIMIT)
        .decode_from_slice(bytes)
        .map_err(|_| malformed_protobuf_error())
}

#[cfg(feature = "generated")]
fn reason_is_valid_for_branch(branch: CodecWireErrorBranch, reason: CodecErrorReason) -> bool {
    let value = reason.to_i32();
    match branch {
        CodecWireErrorBranch::BaseEncoding => (100..=199).contains(&value),
        CodecWireErrorBranch::Pem => (200..=299).contains(&value),
        CodecWireErrorBranch::Multiformat => (300..=399).contains(&value),
        CodecWireErrorBranch::Canonicalization => (400..=499).contains(&value),
        CodecWireErrorBranch::Backend => (500..=599).contains(&value),
        CodecWireErrorBranch::Boundary => (600..=699).contains(&value),
    }
}

#[cfg(feature = "generated")]
fn malformed_protobuf_error() -> CodecWireError {
    CodecWireError::malformed_protobuf()
}

#[cfg(feature = "generated")]
fn resource_limit_error() -> CodecWireError {
    CodecWireError::known_good(
        CodecWireErrorBranch::Boundary,
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
    )
}
