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
use generated::proto::reallyme::codec::v1::{
    __buffa::oneof::codec_error::Error as CodecErrorBranchProto, CodecBackendError,
    CodecBaseEncodingError, CodecCanonicalizationError, CodecError, CodecErrorReason,
    CodecMultiformatError, CodecPemError,
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

    /// Internal constructor for crate-owned known-good backend mappings.
    #[must_use]
    const fn backend_internal(reason: CodecErrorReason) -> Self {
        Self {
            branch: CodecWireErrorBranch::Backend,
            reason,
        }
    }
}

/// Encodes a protobuf message with Buffa.
#[cfg(feature = "generated")]
#[must_use]
pub fn encode_protobuf<M: Message>(message: &M) -> Vec<u8> {
    message.encode_to_vec()
}

/// Decodes a bounded protobuf message from untrusted bytes.
///
/// # Errors
///
/// Returns a backend wire error when input exceeds the boundary size limit or
/// fails protobuf decoding.
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
/// Returns a backend wire error for oversized or malformed
/// JSON and for JSON messages whose binary encoding exceeds the protobuf cap.
#[cfg(feature = "generated")]
pub fn decode_json<M: DeserializeOwned + Message>(bytes: &[u8]) -> CodecWireResult<M> {
    if bytes.len() > MAX_CODEC_PROTO_JSON_BYTES {
        return Err(resource_limit_error());
    }

    let message: M = serde_json::from_slice(bytes).map_err(|_| {
        CodecWireError::backend_internal(
            CodecErrorReason::CODEC_ERROR_REASON_BACKEND_MALFORMED_JSON,
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
    };

    CodecError {
        error: Some(branch),
        __buffa_unknown_fields: Default::default(),
    }
}

/// Encodes a structured `CodecError` as protobuf bytes.
#[cfg(feature = "generated")]
#[must_use]
pub fn codec_error_bytes(error: CodecWireError) -> Vec<u8> {
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

#[cfg(feature = "generated")]
fn decode_protobuf_with_limit<M: Message>(bytes: &[u8], max_bytes: usize) -> CodecWireResult<M> {
    if bytes.len() > max_bytes {
        return Err(resource_limit_error());
    }

    DecodeOptions::new()
        .with_recursion_limit(CODEC_PROTO_RECURSION_LIMIT)
        .with_max_message_size(max_bytes)
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
    }
}

#[cfg(feature = "generated")]
fn malformed_protobuf_error() -> CodecWireError {
    CodecWireError::backend_internal(
        CodecErrorReason::CODEC_ERROR_REASON_BACKEND_MALFORMED_PROTOBUF,
    )
}

#[cfg(feature = "generated")]
fn resource_limit_error() -> CodecWireError {
    CodecWireError::backend_internal(
        CodecErrorReason::CODEC_ERROR_REASON_BACKEND_RESOURCE_LIMIT_EXCEEDED,
    )
}
