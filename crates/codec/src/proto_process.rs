// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Executable protobuf and generated ProtoJSON adapter lane.
//!
//! Native Rust callers should continue to use the typed codec modules. This
//! adapter exists for FFI, WASM, generated SDK, CLI, and transport boundaries
//! that need one self-describing [`CodecOperationRequest`] and one binary
//! [`CodecProtoResultEnvelope`].
//!
//! [`CodecProtoResultEnvelope`]: codec_proto::generated::proto::reallyme::codec::v1::CodecProtoResultEnvelope
//! [`CodecOperationRequest`]: codec_proto::generated::proto::reallyme::codec::v1::CodecOperationRequest

use buffa::EnumValue;
use codec_cbor::{verify_dag_cbor_cid, MAX_DAG_CBOR_INPUT_LEN};
use codec_multicodec::{
    lookup_codec_prefix, CodecSpec, CodecTag as MulticodecTag, KeyMaterialKind, MULTICODEC_TABLE,
    VARIABLE_KEY_LENGTH,
};
use codec_multikey::{parse_multikey, MultikeyError};
use codec_pem::{decode_pem, PemDecodePolicy, PemError, PemLabel};
use codec_proto::generated::proto::reallyme::codec::v1::{
    __buffa::oneof::codec_operation_request::Operation as CodecOperation,
    CodecDagCborVerifyCidResult, CodecErrorReason, CodecKeyMaterialKind,
    CodecMulticodecLookupResult, CodecMulticodecSpec, CodecMulticodecTableResult,
    CodecMultikeyParseResult, CodecOperationRequest, CodecPemDecodeOptions, CodecPemDecodeResult,
    CodecPemLabel, CodecTag as ProtoCodecTag,
};
use codec_proto::{
    decode_json, decode_protobuf, encode_proto_result_envelope_or_error, CodecProtoResult,
    CodecWireError, CodecWireErrorBranch,
};
use zeroize::Zeroizing;

/// Executes a binary generated-protobuf codec request.
#[must_use]
pub fn process_proto(request_bytes: &[u8]) -> Zeroizing<Vec<u8>> {
    encode_proto_result_envelope_or_error(&process_proto_output(request_bytes))
}

/// Executes a binary request and retains its structured Rust result.
#[must_use]
pub fn process_proto_output(request_bytes: &[u8]) -> CodecProtoResult {
    output_from_result(
        decode_protobuf::<CodecOperationRequest>(request_bytes).and_then(process_operation_request),
    )
}

/// Executes a generated ProtoJSON request and returns a binary result envelope.
#[must_use]
pub fn process_proto_json(request_json: &[u8]) -> Zeroizing<Vec<u8>> {
    encode_proto_result_envelope_or_error(&process_proto_json_output(request_json))
}

/// Executes a generated ProtoJSON request and retains its structured Rust result.
#[must_use]
pub fn process_proto_json_output(request_json: &[u8]) -> CodecProtoResult {
    output_from_result(
        decode_json::<CodecOperationRequest>(request_json).and_then(process_operation_request),
    )
}

fn output_from_result(result: Result<CodecProtoResult, CodecWireError>) -> CodecProtoResult {
    match result {
        Ok(output) => output,
        Err(error) => CodecProtoResult::codec_error(error),
    }
}

fn process_operation_request(
    mut request: CodecOperationRequest,
) -> Result<CodecProtoResult, CodecWireError> {
    let Some(operation) = request.operation.take() else {
        return Err(wire_error(
            CodecWireErrorBranch::Boundary,
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MISSING_OPERATION,
        ));
    };

    match operation {
        CodecOperation::MulticodecPrefixForName(request) => {
            process_multicodec_prefix_for_name(&request.name)
        }
        CodecOperation::MulticodecLookupPrefix(request) => {
            process_multicodec_lookup_prefix(&request.value)
        }
        CodecOperation::MulticodecTable(_) => process_multicodec_table(),
        CodecOperation::MultikeyParse(request) => process_multikey_parse(&request.multikey),
        CodecOperation::DagCborVerifyCid(request) => {
            process_dag_cbor_verify_cid(&request.cid, &request.payload)
        }
        CodecOperation::PemDecode(request) => {
            process_pem_decode(&request.pem, request.options.as_option())
        }
    }
}

fn process_multicodec_prefix_for_name(name: &str) -> Result<CodecProtoResult, CodecWireError> {
    let Some((canonical_name, spec)) = find_codec_spec(name) else {
        return Err(wire_error(
            CodecWireErrorBranch::Multiformat,
            CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC,
        ));
    };
    Ok(CodecProtoResult::from_message(&codec_spec_proto(
        canonical_name,
        spec,
    )?))
}

fn process_multicodec_lookup_prefix(value: &[u8]) -> Result<CodecProtoResult, CodecWireError> {
    let Some(found) = lookup_codec_prefix(value) else {
        return Err(wire_error(
            CodecWireErrorBranch::Multiformat,
            CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTICODEC_PREFIX,
        ));
    };
    let spec = CodecSpec {
        tag: found.tag,
        key_material: found.key_material,
        alg: found.alg,
        codec: found.codec,
        key_length: found.key_length,
    };
    let result = CodecMulticodecLookupResult {
        name: found.name.to_owned(),
        prefix_length: usize_to_u32(found.codec.len())?,
        metadata: codec_spec_proto(found.name, &spec)?.into(),
        __buffa_unknown_fields: Default::default(),
    };
    Ok(CodecProtoResult::from_message(&result))
}

fn process_multicodec_table() -> Result<CodecProtoResult, CodecWireError> {
    let mut entries = Vec::with_capacity(MULTICODEC_TABLE.len());
    for (name, spec) in MULTICODEC_TABLE {
        entries.push(codec_spec_proto(name, spec)?);
    }
    Ok(CodecProtoResult::from_message(
        &CodecMulticodecTableResult {
            entries,
            __buffa_unknown_fields: Default::default(),
        },
    ))
}

fn process_multikey_parse(multikey: &str) -> Result<CodecProtoResult, CodecWireError> {
    let parsed = parse_multikey(multikey).map_err(multikey_boundary_error)?;
    let variable_public_key_length = parsed.key_length == VARIABLE_KEY_LENGTH;
    let expected_public_key_length = if variable_public_key_length {
        0
    } else {
        usize_to_u32(parsed.key_length)?
    };
    let result = CodecMultikeyParseResult {
        codec_name: parsed.codec_name.to_owned(),
        algorithm_name: parsed.alg.to_owned(),
        public_key: parsed.public_key,
        expected_public_key_length,
        variable_public_key_length,
        __buffa_unknown_fields: Default::default(),
    };
    Ok(CodecProtoResult::from_message(&result))
}

fn process_dag_cbor_verify_cid(
    cid: &str,
    payload: &[u8],
) -> Result<CodecProtoResult, CodecWireError> {
    if payload.len() > MAX_DAG_CBOR_INPUT_LEN {
        return Err(wire_error(
            CodecWireErrorBranch::Canonicalization,
            CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_INVALID_CBOR,
        ));
    }
    let (valid, expected_cid, actual_cid) = verify_dag_cbor_cid(cid, payload);
    Ok(CodecProtoResult::from_message(
        &CodecDagCborVerifyCidResult {
            valid,
            expected_cid,
            actual_cid,
            __buffa_unknown_fields: Default::default(),
        },
    ))
}

fn process_pem_decode(
    pem: &[u8],
    options: Option<&CodecPemDecodeOptions>,
) -> Result<CodecProtoResult, CodecWireError> {
    let input = core::str::from_utf8(pem).map_err(|_| {
        wire_error(
            CodecWireErrorBranch::Pem,
            CodecErrorReason::CODEC_ERROR_REASON_PEM_INVALID_BODY,
        )
    })?;
    let defaults = PemDecodePolicy::default();
    let mut labels = Vec::new();
    if let Some(options) = options {
        labels.reserve(options.allowed_labels.len());
        for label in &options.allowed_labels {
            labels.push(pem_label(label.as_known())?);
        }
    }
    let allowed_labels = if labels.is_empty() {
        defaults.allowed_labels
    } else {
        labels.as_slice()
    };
    let max_input_len = option_limit(
        options.map_or(0, |value| value.max_input_len),
        defaults.max_input_len,
    )?;
    let max_der_len = option_limit(
        options.map_or(0, |value| value.max_der_len),
        defaults.max_der_len,
    )?;
    let decoded = decode_pem(
        input,
        PemDecodePolicy {
            allowed_labels,
            max_input_len,
            max_der_len,
        },
    )
    .map_err(pem_boundary_error)?;
    let result = CodecPemDecodeResult {
        label: decoded.label.as_str().to_owned(),
        der: decoded.der.as_slice().to_vec(),
        __buffa_unknown_fields: Default::default(),
    };
    Ok(CodecProtoResult::from_message(&result))
}

fn option_limit(value: u32, default: usize) -> Result<usize, CodecWireError> {
    if value == 0 {
        return Ok(default);
    }
    usize::try_from(value).map_err(|_| {
        wire_error(
            CodecWireErrorBranch::Boundary,
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
        )
    })
}

fn pem_label(label: Option<CodecPemLabel>) -> Result<PemLabel, CodecWireError> {
    match label {
        Some(CodecPemLabel::CODEC_PEM_LABEL_PRIVATE_KEY) => Ok(PemLabel::PrivateKey),
        Some(CodecPemLabel::CODEC_PEM_LABEL_EC_PRIVATE_KEY) => Ok(PemLabel::EcPrivateKey),
        Some(CodecPemLabel::CODEC_PEM_LABEL_PUBLIC_KEY) => Ok(PemLabel::PublicKey),
        Some(CodecPemLabel::CODEC_PEM_LABEL_UNSPECIFIED) | None => Err(wire_error(
            CodecWireErrorBranch::Pem,
            CodecErrorReason::CODEC_ERROR_REASON_PEM_UNSUPPORTED_LABEL,
        )),
    }
}

fn find_codec_spec(codec_name: &str) -> Option<(&'static str, &'static CodecSpec)> {
    MULTICODEC_TABLE
        .iter()
        .find(|(name, _)| *name == codec_name)
        .map(|(name, spec)| (*name, spec))
}

fn codec_spec_proto(name: &str, spec: &CodecSpec) -> Result<CodecMulticodecSpec, CodecWireError> {
    let variable_length = spec.key_length == VARIABLE_KEY_LENGTH;
    let fixed_length = if variable_length {
        0
    } else {
        usize_to_u32(spec.key_length)?
    };
    Ok(CodecMulticodecSpec {
        name: name.to_owned(),
        code: spec.codec.to_vec(),
        prefix: spec.codec.to_vec(),
        tag: EnumValue::from(codec_tag_proto(spec.tag)?),
        key_material_kind: EnumValue::from(key_material_kind_proto(spec.key_material)?),
        fixed_length,
        variable_length,
        algorithm_name: spec.alg.to_owned(),
        __buffa_unknown_fields: Default::default(),
    })
}

fn codec_tag_proto(tag: MulticodecTag) -> Result<ProtoCodecTag, CodecWireError> {
    let tag = match tag {
        MulticodecTag::Encryption => ProtoCodecTag::CODEC_TAG_ENCRYPTION,
        MulticodecTag::Key => ProtoCodecTag::CODEC_TAG_KEY,
        MulticodecTag::Hash => ProtoCodecTag::CODEC_TAG_HASH,
        MulticodecTag::Multihash => ProtoCodecTag::CODEC_TAG_MULTIHASH,
        MulticodecTag::Multikey => ProtoCodecTag::CODEC_TAG_MULTIKEY,
        _ => return Err(internal_wire_error()),
    };
    Ok(tag)
}

fn key_material_kind_proto(kind: KeyMaterialKind) -> Result<CodecKeyMaterialKind, CodecWireError> {
    let kind = match kind {
        KeyMaterialKind::PublicKey => CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_PUBLIC_KEY,
        KeyMaterialKind::PrivateKey => CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_PRIVATE_KEY,
        KeyMaterialKind::SymmetricKey => {
            CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_SYMMETRIC_KEY
        }
        KeyMaterialKind::NotKey => CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_NOT_KEY,
        _ => return Err(internal_wire_error()),
    };
    Ok(kind)
}

fn usize_to_u32(value: usize) -> Result<u32, CodecWireError> {
    u32::try_from(value).map_err(|_| {
        wire_error(
            CodecWireErrorBranch::Backend,
            CodecErrorReason::CODEC_ERROR_REASON_BACKEND_INTERNAL,
        )
    })
}

fn pem_boundary_error(error: PemError) -> CodecWireError {
    match error {
        PemError::InputTooLarge => wire_error(
            CodecWireErrorBranch::BaseEncoding,
            CodecErrorReason::CODEC_ERROR_REASON_BASE_INPUT_TOO_LARGE,
        ),
        PemError::DerTooLarge => wire_error(
            CodecWireErrorBranch::Pem,
            CodecErrorReason::CODEC_ERROR_REASON_PEM_DER_TOO_LARGE,
        ),
        PemError::MissingBegin
        | PemError::MissingEnd
        | PemError::InvalidBoundary
        | PemError::InvalidOptions => wire_error(
            CodecWireErrorBranch::Pem,
            CodecErrorReason::CODEC_ERROR_REASON_PEM_INVALID_BOUNDARY,
        ),
        PemError::LabelMismatch => wire_error(
            CodecWireErrorBranch::Pem,
            CodecErrorReason::CODEC_ERROR_REASON_PEM_LABEL_MISMATCH,
        ),
        PemError::UnsupportedLabel => wire_error(
            CodecWireErrorBranch::Pem,
            CodecErrorReason::CODEC_ERROR_REASON_PEM_UNSUPPORTED_LABEL,
        ),
        PemError::InvalidBase64 | PemError::InvalidBody => wire_error(
            CodecWireErrorBranch::Pem,
            CodecErrorReason::CODEC_ERROR_REASON_PEM_INVALID_BODY,
        ),
        _ => internal_wire_error(),
    }
}

fn multikey_boundary_error(error: MultikeyError) -> CodecWireError {
    let reason = match error {
        MultikeyError::UnknownCodecPrefix | MultikeyError::UnknownCodecName { .. } => {
            CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC
        }
        MultikeyError::InvalidMultibase
        | MultikeyError::DecodedTooShort(_)
        | MultikeyError::KeyLengthMismatch { .. }
        | MultikeyError::KeyTooLarge { .. }
        | MultikeyError::EncodedPayloadTooLarge
        | MultikeyError::BindingTypeCodecMismatch { .. }
        | MultikeyError::BindingAlgorithmMismatch { .. }
        | MultikeyError::BindingAlgorithmMissing { .. } => {
            CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTIKEY
        }
        _ => return internal_wire_error(),
    };
    wire_error(CodecWireErrorBranch::Multiformat, reason)
}

fn internal_wire_error() -> CodecWireError {
    wire_error(
        CodecWireErrorBranch::Backend,
        CodecErrorReason::CODEC_ERROR_REASON_BACKEND_INTERNAL,
    )
}

fn wire_error(branch: CodecWireErrorBranch, reason: CodecErrorReason) -> CodecWireError {
    match CodecWireError::try_new(branch, reason) {
        Ok(error) => error,
        Err(_) => CodecWireError::malformed_protobuf(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use buffa::{Enumeration, Message};
    use codec_proto::decode_proto_result_envelope;
    use codec_proto::generated::proto::reallyme::codec::v1::{
        CodecMulticodecTableRequest, CodecProtoResultStatus,
    };

    use super::*;

    fn table_request() -> CodecOperationRequest {
        CodecOperationRequest {
            operation: Some(CodecOperation::MulticodecTable(Box::new(
                CodecMulticodecTableRequest {
                    __buffa_unknown_fields: Default::default(),
                },
            ))),
            __buffa_unknown_fields: Default::default(),
        }
    }

    #[test]
    fn binary_and_proto_json_dispatch_match() {
        let request = table_request();
        let binary = process_proto(&request.encode_to_vec());
        let json = serde_json::to_vec(&request).unwrap();
        let from_json = process_proto_json(&json);
        assert_eq!(binary.as_slice(), from_json.as_slice());

        let decoded = decode_proto_result_envelope(&binary);
        assert!(decoded.is_ok());
        let result = decoded.ok().unwrap();
        assert_eq!(result.status(), codec_proto::CodecProtoStatus::Result);
    }

    #[test]
    fn missing_operation_is_a_structured_boundary_error() {
        let request = CodecOperationRequest::default();
        let envelope = process_proto(&request.encode_to_vec());
        let decoded = decode_proto_result_envelope(&envelope);
        assert!(decoded.is_ok());
        let result = decoded.ok().unwrap();
        assert_eq!(result.status(), codec_proto::CodecProtoStatus::CodecError);
        let error = codec_proto::decode_codec_error_payload(result.bytes()).unwrap();
        assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
        assert_eq!(
            error.reason(),
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MISSING_OPERATION
        );
    }

    #[test]
    fn malformed_binary_is_a_structured_boundary_error() {
        let envelope = process_proto(&[0xff]);
        let decoded = decode_proto_result_envelope(&envelope);
        assert!(decoded.is_ok());
        let result = decoded.ok().unwrap();
        let error = codec_proto::decode_codec_error_payload(result.bytes()).unwrap();
        assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
        assert_eq!(
            error.reason(),
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_PROTOBUF
        );
    }

    #[test]
    fn result_status_enum_numbers_are_stable() {
        assert_eq!(
            CodecProtoResultStatus::CODEC_PROTO_RESULT_STATUS_RESULT.to_i32(),
            1
        );
        assert_eq!(
            CodecProtoResultStatus::CODEC_PROTO_RESULT_STATUS_CODEC_ERROR.to_i32(),
            2
        );
    }
}
