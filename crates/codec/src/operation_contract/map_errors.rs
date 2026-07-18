// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0


fn multicodec_boundary_error(error: MulticodecOperationError) -> CodecWireError {
    match error {
        MulticodecOperationError::UnknownName => wire_error(
            CodecWireErrorBranch::Multiformat,
            CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC,
        ),
        MulticodecOperationError::InvalidPrefix => wire_error(
            CodecWireErrorBranch::Multiformat,
            CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTICODEC_PREFIX,
        ),
        MulticodecOperationError::RegistryInvariant
        | MulticodecOperationError::AllocationFailure => internal_wire_error(),
    }
}

fn dag_cbor_boundary_error(error: DagCborOperationError) -> CodecWireError {
    match error {
        DagCborOperationError::PayloadTooLarge => wire_error(
            CodecWireErrorBranch::Boundary,
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
        ),
    }
}

fn dag_cbor_cbor_wire_error(error: CborError) -> CodecWireError {
    match error {
        CborError::InputTooLarge
        | CborError::OutputTooLarge
        | CborError::OffsetOverflow
        | CborError::LengthTooLarge
        | CborError::ContainerLengthExceedsInput
        | CborError::DepthExceeded => wire_error(
            CodecWireErrorBranch::Boundary,
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
        ),
        CborError::NonCanonicalInteger
        | CborError::DuplicateMapKey
        | CborError::MapKeysOutOfOrder
        | CborError::TrailingBytes => wire_error(
            CodecWireErrorBranch::Canonicalization,
            CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_NON_CANONICAL_CBOR,
        ),
        CborError::UnexpectedEnd
        | CborError::IntegerOutOfRange
        | CborError::TruncatedArgument
        | CborError::TruncatedBytes
        | CborError::InvalidUtf8
        | CborError::MapKeyMustBeString
        | CborError::DisallowedSimpleValue { .. }
        | CborError::DisallowedMajorType { .. }
        | CborError::UnsupportedAdditionalInfo => wire_error(
            CodecWireErrorBranch::Canonicalization,
            CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_INVALID_CBOR,
        ),
        _ => internal_wire_error(),
    }
}

fn pem_boundary_error(error: PemOperationError) -> CodecWireError {
    match error {
        PemOperationError::InputTooLarge => wire_error(
            CodecWireErrorBranch::BaseEncoding,
            CodecErrorReason::CODEC_ERROR_REASON_BASE_INPUT_TOO_LARGE,
        ),
        PemOperationError::DerTooLarge => wire_error(
            CodecWireErrorBranch::Pem,
            CodecErrorReason::CODEC_ERROR_REASON_PEM_DER_TOO_LARGE,
        ),
        PemOperationError::InvalidBoundary | PemOperationError::InvalidPolicy => wire_error(
            CodecWireErrorBranch::Pem,
            CodecErrorReason::CODEC_ERROR_REASON_PEM_INVALID_BOUNDARY,
        ),
        PemOperationError::LabelMismatch => wire_error(
            CodecWireErrorBranch::Pem,
            CodecErrorReason::CODEC_ERROR_REASON_PEM_LABEL_MISMATCH,
        ),
        PemOperationError::UnsupportedLabel => wire_error(
            CodecWireErrorBranch::Pem,
            CodecErrorReason::CODEC_ERROR_REASON_PEM_UNSUPPORTED_LABEL,
        ),
        PemOperationError::InvalidBody => wire_error(
            CodecWireErrorBranch::Pem,
            CodecErrorReason::CODEC_ERROR_REASON_PEM_INVALID_BODY,
        ),
        _ => internal_wire_error(),
    }
}

fn multikey_boundary_error(error: MultikeyOperationError) -> CodecWireError {
    let reason = match error {
        MultikeyOperationError::UnknownCodec => {
            CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC
        }
        MultikeyOperationError::InvalidMultikey => {
            CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTIKEY
        }
        MultikeyOperationError::OperationInvariant => return internal_wire_error(),
    };
    wire_error(CodecWireErrorBranch::Multiformat, reason)
}

fn deterministic_cbor_wire_error(error: DeterministicCborError) -> CodecWireError {
    match error {
        DeterministicCborError::InputTooLarge
        | DeterministicCborError::OutputTooLarge
        | DeterministicCborError::LengthTooLarge
        | DeterministicCborError::DepthExceeded
        | DeterministicCborError::NodeLimitExceeded
        | DeterministicCborError::ContainerEntriesExceeded
        | DeterministicCborError::AggregateTextBytesExceeded
        | DeterministicCborError::AggregateByteStringBytesExceeded
        | DeterministicCborError::ContainerLengthExceedsInput => wire_error(
            CodecWireErrorBranch::Boundary,
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
        ),
        DeterministicCborError::NonCanonicalInteger
        | DeterministicCborError::DuplicateMapKey
        | DeterministicCborError::MapKeysOutOfOrder
        | DeterministicCborError::TrailingBytes => wire_error(
            CodecWireErrorBranch::Canonicalization,
            CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_NON_CANONICAL_CBOR,
        ),
        DeterministicCborError::PreflightLengthMismatch
        | DeterministicCborError::OffsetOverflow
        | DeterministicCborError::AllocationFailure => internal_wire_error(),
        DeterministicCborError::UnexpectedEnd
        | DeterministicCborError::NegativeIntegerOutOfRange
        | DeterministicCborError::NegativeIntegerMustBeNegative
        | DeterministicCborError::TruncatedArgument
        | DeterministicCborError::TruncatedBytes
        | DeterministicCborError::InvalidUtf8
        | DeterministicCborError::UnsupportedMapKeyType
        | DeterministicCborError::UnsupportedSimpleValue
        | DeterministicCborError::UnsupportedMajorType
        | DeterministicCborError::UnsupportedAdditionalInfo => wire_error(
            CodecWireErrorBranch::Canonicalization,
            CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_INVALID_CBOR,
        ),
        _ => internal_wire_error(),
    }
}

fn internal_wire_error() -> CodecWireError {
    wire_error(
        CodecWireErrorBranch::Backend,
        CodecErrorReason::CODEC_ERROR_REASON_BACKEND_INTERNAL,
    )
}

fn malformed_request_wire_error() -> CodecWireError {
    wire_error(
        CodecWireErrorBranch::Boundary,
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_PROTOBUF,
    )
}

fn wire_error(branch: CodecWireErrorBranch, reason: CodecErrorReason) -> CodecWireError {
    match CodecWireError::try_new(branch, reason) {
        Ok(error) => error,
        Err(_) => CodecWireError::malformed_protobuf(),
    }
}
