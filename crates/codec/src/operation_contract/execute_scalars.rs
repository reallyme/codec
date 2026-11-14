// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::{CodecWireError, CodecWireErrorBranch};
use proto::codec_api_operation_request::Operation;
use proto::codec_api_operation_result::Result as OperationResult;
use proto::CodecErrorReason as Reason;

use super::{internal_wire_error, malformed_request_wire_error, scalar_ops as scalar, wire_error};

// Scalar transport requests use the same aggregate budget as the legacy ABI.
const MAX_SCALAR_INPUT_BYTES: usize = codec_proto::MAX_CODEC_PROVIDER_INPUT_BYTES;

pub(super) fn execute(operation: &mut Operation) -> Result<OperationResult, CodecWireError> {
    match operation {
        Operation::Base64Encode(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::Base64Encode(Box::new(
                proto::CodecTextResult {
                    value: scalar::encode_base64(&request.value).into(),
                },
            )))
        }
        Operation::Base64Decode(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::Base64Decode(Box::new(
                proto::CodecBytesResult {
                    value: scalar::decode_base64(&request.value)
                        .map_err(base64_error)?
                        .into(),
                },
            )))
        }
        Operation::Base64urlEncode(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::Base64urlEncode(Box::new(
                proto::CodecTextResult {
                    value: scalar::encode_base64url(&request.value).into(),
                },
            )))
        }
        Operation::Base64urlDecode(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::Base64urlDecode(Box::new(
                proto::CodecBytesResult {
                    value: scalar::decode_base64url(&request.value)
                        .map_err(base64url_error)?
                        .into(),
                },
            )))
        }
        Operation::Base64urlDecodeBytes(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::Base64urlDecodeBytes(Box::new(
                proto::CodecBytesResult {
                    value: scalar::decode_base64url_bytes(&request.value)
                        .map_err(base64url_error)?
                        .into(),
                },
            )))
        }
        Operation::LowerHexEncode(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::LowerHexEncode(Box::new(
                proto::CodecTextResult {
                    value: scalar::encode_lower_hex(&request.value).into(),
                },
            )))
        }
        Operation::LowerHexDecode(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::LowerHexDecode(Box::new(
                proto::CodecBytesResult {
                    value: scalar::decode_lower_hex(&request.value)
                        .map_err(hex_error)?
                        .into(),
                },
            )))
        }
        Operation::Base58btcEncode(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::Base58btcEncode(Box::new(
                proto::CodecTextResult {
                    value: scalar::encode_base58btc(&request.value)
                        .map_err(base58_error)?
                        .into(),
                },
            )))
        }
        Operation::Base58btcDecode(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::Base58btcDecode(Box::new(
                proto::CodecBytesResult {
                    value: scalar::decode_base58btc(&request.value)
                        .map_err(base58_error)?
                        .into(),
                },
            )))
        }
        Operation::MultibaseBase58btcEncode(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::MultibaseBase58btcEncode(Box::new(
                proto::CodecTextResult {
                    value: scalar::encode_multibase_base58btc(&request.value)
                        .map_err(base58_error)?
                        .into(),
                },
            )))
        }
        Operation::MultibaseBase64urlEncode(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::MultibaseBase64urlEncode(Box::new(
                proto::CodecTextResult {
                    value: scalar::encode_multibase_base64url(&request.value)
                        .map_err(multibase_error)?
                        .into(),
                },
            )))
        }
        Operation::MultibaseDecode(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::MultibaseDecode(Box::new(
                proto::CodecBytesResult {
                    value: scalar::decode_multibase(&request.value)
                        .map_err(multibase_error)?
                        .into(),
                },
            )))
        }
        Operation::CanonicalizeJson(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::CanonicalizeJson(Box::new(
                proto::CodecTextResult {
                    value: scalar::canonicalize_json(&request.value)
                        .map_err(jcs_error)?
                        .into(),
                },
            )))
        }
        Operation::DagCborComputeCid(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::DagCborComputeCid(Box::new(
                proto::CodecTextResult {
                    value: scalar::compute_dag_cbor_cid(&request.value)
                        .map_err(scalar_error)?
                        .into(),
                },
            )))
        }
        Operation::DagCborMultihash(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::DagCborMultihash(Box::new(
                proto::CodecBytesResult {
                    value: scalar::dag_cbor_multihash_value(&request.value)
                        .map_err(scalar_error)?
                        .to_bytes()
                        .into(),
                },
            )))
        }
        Operation::DagCborSha256ContentHash(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::DagCborSha256ContentHash(Box::new(
                proto::CodecBytesResult {
                    value: super::try_copy_result_bytes(
                        &scalar::dag_cbor_content_hash(&request.value).map_err(scalar_error)?,
                    )?
                    .into(),
                },
            )))
        }
        Operation::MulticodecStripPrefix(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::MulticodecStripPrefix(Box::new(
                proto::CodecBytesResult {
                    value: super::try_copy_result_bytes(
                        crate::multicodec::strip_prefix(&request.value)
                            .map_err(super::multicodec_boundary_error)?,
                    )?
                    .into(),
                },
            )))
        }
        Operation::IsValidCidString(request) => {
            check_lengths(&[request.value.len()])?;
            Ok(OperationResult::IsValidCidString(Box::new(
                proto::CodecBoolResult {
                    value: scalar::valid_cid(&request.value),
                },
            )))
        }
        Operation::TryParseCid(request) => {
            check_lengths(&[request.value.len()])?;
            let value = scalar::parse_cid(&request.value).map(|value| proto::CodecTextResult {
                value: value.into(),
            });
            Ok(OperationResult::TryParseCid(Box::new(
                proto::CodecOptionalTextResult {
                    value: value.into(),
                },
            )))
        }
        Operation::DagCborCodecCode(_) => Ok(OperationResult::DagCborCodecCode(Box::new(
            proto::CodecUint32Result {
                value: scalar::dag_cbor_codec_code(),
            },
        ))),
        Operation::MultikeyEncode(request) => {
            check_lengths(&[request.codec_name.len(), request.public_key.len()])?;
            let value = scalar::encode_multikey(&request.codec_name, &request.public_key)
                .map_err(multikey_error)?;
            Ok(OperationResult::MultikeyEncode(Box::new(
                proto::CodecTextResult {
                    value: value.into(),
                },
            )))
        }
        Operation::BindingTypeMatchesCodec(request) => {
            check_lengths(&[request.binding_type.len(), request.codec_name.len()])?;
            Ok(OperationResult::BindingTypeMatchesCodec(Box::new(
                proto::CodecBoolResult {
                    value: scalar::binding_matches_codec(
                        &request.binding_type,
                        &request.codec_name,
                    ),
                },
            )))
        }
        Operation::ValidateKeyBinding(request) => {
            check_lengths(&[
                request.binding_type.len(),
                request.algorithm.as_ref().map_or(0, |text| text.len()),
                request.multikey.len(),
            ])?;
            scalar::validate_encoded_binding(
                &request.binding_type,
                request.algorithm.as_deref(),
                &request.multikey,
            )
            .map_err(multikey_error)?;
            Ok(OperationResult::ValidateKeyBinding(Box::default()))
        }
        Operation::RequireSupportedMulticodec(request) => {
            check_lengths(&[request.value.len()])?;
            let spec = crate::multicodec::prefix_for_name(&request.value)
                .map_err(super::multicodec_boundary_error)?;
            Ok(OperationResult::RequireSupportedMulticodec(Box::new(
                proto::CodecTextResult {
                    value: super::try_copy_result_string(spec.name())?.into(),
                },
            )))
        }
        Operation::ProviderInfo(_) => Ok(OperationResult::ProviderInfo(Box::new(provider_info()?))),
        _ => Err(malformed_request_wire_error()),
    }
}

pub(super) fn check_lengths(lengths: &[usize]) -> Result<(), CodecWireError> {
    let mut total = 0usize;
    for length in lengths {
        total = total.checked_add(*length).ok_or_else(input_limit)?;
        if total > MAX_SCALAR_INPUT_BYTES {
            return Err(input_limit());
        }
    }
    Ok(())
}

fn provider_info() -> Result<proto::CodecProviderInfo, CodecWireError> {
    Ok(proto::CodecProviderInfo {
        abi_version: codec_proto::CODEC_PROVIDER_ABI_VERSION,
        package_major: env!("CARGO_PKG_VERSION_MAJOR")
            .parse()
            .map_err(|_| internal_wire_error())?,
        package_minor: env!("CARGO_PKG_VERSION_MINOR")
            .parse()
            .map_err(|_| internal_wire_error())?,
        package_patch: env!("CARGO_PKG_VERSION_PATCH")
            .parse()
            .map_err(|_| internal_wire_error())?,
        max_input_bytes: u64::try_from(MAX_SCALAR_INPUT_BYTES)
            .map_err(|_| internal_wire_error())?,
        max_output_bytes: u64::try_from(codec_proto::MAX_CODEC_PROVIDER_OUTPUT_BYTES)
            .map_err(|_| internal_wire_error())?,
        max_operation_response_bytes: u64::try_from(codec_proto::MAX_CODEC_PROTO_MESSAGE_BYTES)
            .map_err(|_| internal_wire_error())?,
    })
}

fn input_limit() -> CodecWireError {
    wire_error(
        CodecWireErrorBranch::BaseEncoding,
        Reason::CODEC_ERROR_REASON_BASE_INPUT_TOO_LARGE,
    )
}
fn scalar_error(error: scalar::ScalarOperationError) -> CodecWireError {
    match error {
        scalar::ScalarOperationError::InputTooLarge => input_limit(),
    }
}
fn base_error(reason: Reason) -> CodecWireError {
    wire_error(CodecWireErrorBranch::BaseEncoding, reason)
}
fn base64_error(error: codec_base64::Base64Error) -> CodecWireError {
    match error {
        codec_base64::Base64Error::InputTooLarge => input_limit(),
        codec_base64::Base64Error::Invalid => {
            base_error(Reason::CODEC_ERROR_REASON_BASE_INVALID_BASE64)
        }
        _ => internal_wire_error(),
    }
}
fn base64url_error(error: codec_base64url::Base64UrlError) -> CodecWireError {
    match error {
        codec_base64url::Base64UrlError::InputTooLarge => input_limit(),
        codec_base64url::Base64UrlError::Invalid => {
            base_error(Reason::CODEC_ERROR_REASON_BASE_INVALID_BASE64URL)
        }
        _ => internal_wire_error(),
    }
}
fn hex_error(error: codec_hex::HexError) -> CodecWireError {
    match error {
        codec_hex::HexError::Uppercase => {
            base_error(Reason::CODEC_ERROR_REASON_BASE_NON_CANONICAL_HEX)
        }
        codec_hex::HexError::OddLength => {
            base_error(Reason::CODEC_ERROR_REASON_BASE_INVALID_LENGTH)
        }
        codec_hex::HexError::InvalidCharacter => {
            base_error(Reason::CODEC_ERROR_REASON_BASE_INVALID_HEX)
        }
        codec_hex::HexError::OutputCapacityExceeded => {
            base_error(Reason::CODEC_ERROR_REASON_BASE_OUTPUT_TOO_LARGE)
        }
        _ => internal_wire_error(),
    }
}
fn base58_error(error: codec_multibase::Base58Error) -> CodecWireError {
    match error {
        codec_multibase::Base58Error::InputTooLarge => input_limit(),
        codec_multibase::Base58Error::BufferTooSmall => {
            base_error(Reason::CODEC_ERROR_REASON_BASE_OUTPUT_TOO_LARGE)
        }
        codec_multibase::Base58Error::InvalidCharacter
        | codec_multibase::Base58Error::NonAsciiCharacter
        | codec_multibase::Base58Error::DecodeFailed => {
            base_error(Reason::CODEC_ERROR_REASON_BASE_INVALID_ENCODING)
        }
        _ => internal_wire_error(),
    }
}
fn multibase_error(error: codec_multibase::MultibaseError) -> CodecWireError {
    match error {
        codec_multibase::MultibaseError::Base58(error) => base58_error(error),
        codec_multibase::MultibaseError::Base64Url => {
            base_error(Reason::CODEC_ERROR_REASON_BASE_INVALID_BASE64URL)
        }
        codec_multibase::MultibaseError::InputTooLarge => input_limit(),
        codec_multibase::MultibaseError::LengthOverflow => {
            base_error(Reason::CODEC_ERROR_REASON_BASE_OUTPUT_TOO_LARGE)
        }
        codec_multibase::MultibaseError::TooShort
        | codec_multibase::MultibaseError::UnsupportedPrefix => wire_error(
            CodecWireErrorBranch::Multiformat,
            Reason::CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTIBASE_PREFIX,
        ),
        _ => internal_wire_error(),
    }
}
fn multikey_error(error: codec_multikey::MultikeyError) -> CodecWireError {
    let reason = match error {
        codec_multikey::MultikeyError::UnknownCodecName { .. }
        | codec_multikey::MultikeyError::UnknownCodecPrefix => {
            Reason::CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC
        }
        _ => Reason::CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTIKEY,
    };
    wire_error(CodecWireErrorBranch::Multiformat, reason)
}
fn jcs_error(error: codec_jcs::JcsError) -> CodecWireError {
    let reason = match error {
        codec_jcs::JcsError::DuplicateProperty => {
            Reason::CODEC_ERROR_REASON_CANONICAL_NON_CANONICAL_JSON
        }
        codec_jcs::JcsError::SerializationError
        | codec_jcs::JcsError::UnsupportedNumberRepresentation => {
            Reason::CODEC_ERROR_REASON_CANONICAL_INTERNAL
        }
        _ => Reason::CODEC_ERROR_REASON_CANONICAL_INVALID_JSON,
    };
    wire_error(CodecWireErrorBranch::Canonicalization, reason)
}
