// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use buffa::{EnumValue, Enumeration};
use thiserror::Error;

use crate::generated::proto::reallyme::codec::v1::{
    __buffa::oneof::codec_error::Error as CodecErrorBranchProto, CodecBackendError,
    CodecBaseEncodingError, CodecBoundaryError, CodecCanonicalizationError, CodecError,
    CodecErrorOrigin as CodecErrorOriginProto, CodecErrorReason, CodecMultiformatError,
    CodecPemError,
};

/// Stable codec wire-error branch.
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

/// Stable attribution for a codec wire failure.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub enum CodecWireErrorOrigin {
    /// The caller supplied an invalid, unsupported, or oversized request.
    Caller,
    /// The provider violated an invariant or failed internally.
    Provider,
}

/// Validated public codec wire error.
#[derive(Debug, Clone, Copy, Eq, Error, PartialEq)]
#[error("codec wire boundary error")]
pub struct CodecWireError {
    branch: CodecWireErrorBranch,
    reason: CodecErrorReason,
    origin: CodecWireErrorOrigin,
}

/// Error returned when constructing a codec wire error with an invalid reason.
#[derive(Debug, Clone, Copy, Eq, Error, PartialEq)]
#[non_exhaustive]
pub enum CodecWireErrorConstructionError {
    /// The reason is unspecified, unknown, or belongs to a different branch.
    #[error("codec error reason does not belong to the selected branch")]
    BranchReasonMismatch,
}

/// Result alias for codec protobuf boundary helpers.
pub type CodecWireResult<T> = Result<T, CodecWireError>;

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

    /// Returns whether the failure is attributable to the caller or provider.
    #[must_use]
    pub const fn origin(self) -> CodecWireErrorOrigin {
        self.origin
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
        Ok(Self {
            branch,
            reason,
            origin: error_origin(branch, reason),
        })
    }

    /// Internal constructor for crate-owned known-good mappings.
    #[must_use]
    pub(crate) const fn known_good(branch: CodecWireErrorBranch, reason: CodecErrorReason) -> Self {
        Self {
            branch,
            reason,
            origin: error_origin(branch, reason),
        }
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

/// Builds the structured `CodecError` protobuf message for a validated error.
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
        origin: EnumValue::from(match error.origin() {
            CodecWireErrorOrigin::Caller => CodecErrorOriginProto::CODEC_ERROR_ORIGIN_CALLER,
            CodecWireErrorOrigin::Provider => CodecErrorOriginProto::CODEC_ERROR_ORIGIN_PROVIDER,
        }),
        __buffa_unknown_fields: Default::default(),
    }
}

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

const fn error_origin(
    branch: CodecWireErrorBranch,
    reason: CodecErrorReason,
) -> CodecWireErrorOrigin {
    match (branch, reason) {
        (CodecWireErrorBranch::Backend, _)
        | (
            CodecWireErrorBranch::Canonicalization,
            CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_INTERNAL,
        ) => CodecWireErrorOrigin::Provider,
        _ => CodecWireErrorOrigin::Caller,
    }
}

pub(crate) fn malformed_protobuf_error() -> CodecWireError {
    CodecWireError::malformed_protobuf()
}

pub(crate) fn malformed_json_error() -> CodecWireError {
    CodecWireError::known_good(
        CodecWireErrorBranch::Boundary,
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_JSON,
    )
}

pub(crate) fn resource_limit_error() -> CodecWireError {
    CodecWireError::known_good(
        CodecWireErrorBranch::Boundary,
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
    )
}
