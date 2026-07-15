// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use buffa::{EnumValue, Message};
use codec_cbor::verify_dag_cbor_cid;
use codec_cbor::MAX_DAG_CBOR_INPUT_LEN;
use codec_multicodec::{
    lookup_codec_prefix, CodecSpec, CodecTag as MulticodecTag, KeyMaterialKind, MULTICODEC_TABLE,
    VARIABLE_KEY_LENGTH,
};
use codec_multikey::{parse_multikey, MultikeyError};
use codec_pem::{decode_pem, PemDecodePolicy, PemError, PemLabel};
use codec_proto::generated::proto::reallyme::codec::v1::{
    CodecBaseEncodingError, CodecCanonicalizationError, CodecDagCborVerifyCidResult, CodecError,
    CodecErrorReason, CodecKeyMaterialKind, CodecMulticodecLookupResult, CodecMulticodecSpec,
    CodecMulticodecTableResult, CodecMultiformatError, CodecMultikeyParseResult,
    CodecPemDecodeResult, CodecPemError, CodecTag as ProtoCodecTag,
};
use js_sys::{Object, Uint8Array};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use zeroize::Zeroizing;

use crate::map_error::{invalid_input, provider_failure};
use crate::pem::{label_text, parse_decode_options, parse_label};
use crate::write_js_object::{set_bytes, set_string};

#[derive(Clone, Copy)]
enum CodecBoundaryError {
    BaseEncoding(CodecErrorReason),
    Pem(CodecErrorReason),
    Multiformat(CodecErrorReason),
    Canonicalization(CodecErrorReason),
}

enum ProtoStatus {
    Result,
    CodecError,
}

struct ProtoOutput {
    status: ProtoStatus,
    bytes: Zeroizing<Vec<u8>>,
}

fn codec_tag_proto(tag: MulticodecTag) -> ProtoCodecTag {
    match tag {
        MulticodecTag::Encryption => ProtoCodecTag::CODEC_TAG_ENCRYPTION,
        MulticodecTag::Key => ProtoCodecTag::CODEC_TAG_KEY,
        MulticodecTag::Hash => ProtoCodecTag::CODEC_TAG_HASH,
        MulticodecTag::Multihash => ProtoCodecTag::CODEC_TAG_MULTIHASH,
        MulticodecTag::Multikey => ProtoCodecTag::CODEC_TAG_MULTIKEY,
    }
}

fn key_material_kind_proto(kind: KeyMaterialKind) -> CodecKeyMaterialKind {
    match kind {
        KeyMaterialKind::PublicKey => CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_PUBLIC_KEY,
        KeyMaterialKind::PrivateKey => CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_PRIVATE_KEY,
        KeyMaterialKind::SymmetricKey => {
            CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_SYMMETRIC_KEY
        }
        KeyMaterialKind::NotKey => CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_NOT_KEY,
    }
}

fn usize_to_u32(value: usize) -> Result<u32, JsValue> {
    u32::try_from(value).map_err(|_| provider_failure())
}

fn codec_spec_proto(name: &str, spec: &CodecSpec) -> Result<CodecMulticodecSpec, JsValue> {
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
        tag: EnumValue::from(codec_tag_proto(spec.tag)),
        key_material_kind: EnumValue::from(key_material_kind_proto(spec.key_material)),
        fixed_length,
        variable_length,
        algorithm_name: spec.alg.to_owned(),
        __buffa_unknown_fields: Default::default(),
    })
}

fn encode_proto(message: &impl Message) -> ProtoOutput {
    ProtoOutput {
        status: ProtoStatus::Result,
        bytes: Zeroizing::new(message.encode_to_vec()),
    }
}

fn encode_error_proto(error: CodecBoundaryError) -> ProtoOutput {
    let message = match error {
        CodecBoundaryError::BaseEncoding(reason) => CodecError {
            error: CodecBaseEncodingError {
                reason: EnumValue::from(reason),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
            __buffa_unknown_fields: Default::default(),
        },
        CodecBoundaryError::Pem(reason) => CodecError {
            error: CodecPemError {
                reason: EnumValue::from(reason),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
            __buffa_unknown_fields: Default::default(),
        },
        CodecBoundaryError::Multiformat(reason) => CodecError {
            error: CodecMultiformatError {
                reason: EnumValue::from(reason),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
            __buffa_unknown_fields: Default::default(),
        },
        CodecBoundaryError::Canonicalization(reason) => CodecError {
            error: CodecCanonicalizationError {
                reason: EnumValue::from(reason),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
            __buffa_unknown_fields: Default::default(),
        },
    };
    ProtoOutput {
        status: ProtoStatus::CodecError,
        bytes: Zeroizing::new(message.encode_to_vec()),
    }
}

fn proto_bytes(output: ProtoOutput) -> Result<Uint8Array, JsValue> {
    match output.status {
        ProtoStatus::Result => Ok(Uint8Array::from(output.bytes.as_slice())),
        ProtoStatus::CodecError => Err(invalid_input()),
    }
}

fn proto_result_object(output: ProtoOutput) -> Result<JsValue, JsValue> {
    let object = Object::new();
    let status = match output.status {
        ProtoStatus::Result => "result",
        ProtoStatus::CodecError => "codec-error",
    };
    set_string(&object, "status", status)?;
    set_bytes(&object, "bytes", output.bytes.as_slice())?;
    Ok(object.into())
}

fn pem_boundary_error(error: PemError) -> CodecBoundaryError {
    match error {
        PemError::InputTooLarge => CodecBoundaryError::BaseEncoding(
            CodecErrorReason::CODEC_ERROR_REASON_BASE_INPUT_TOO_LARGE,
        ),
        PemError::DerTooLarge => {
            CodecBoundaryError::Pem(CodecErrorReason::CODEC_ERROR_REASON_PEM_DER_TOO_LARGE)
        }
        PemError::MissingBegin
        | PemError::MissingEnd
        | PemError::InvalidBoundary
        | PemError::InvalidOptions => {
            CodecBoundaryError::Pem(CodecErrorReason::CODEC_ERROR_REASON_PEM_INVALID_BOUNDARY)
        }
        PemError::LabelMismatch => {
            CodecBoundaryError::Pem(CodecErrorReason::CODEC_ERROR_REASON_PEM_LABEL_MISMATCH)
        }
        PemError::UnsupportedLabel => {
            CodecBoundaryError::Pem(CodecErrorReason::CODEC_ERROR_REASON_PEM_UNSUPPORTED_LABEL)
        }
        PemError::InvalidBase64 | PemError::InvalidBody => {
            CodecBoundaryError::Pem(CodecErrorReason::CODEC_ERROR_REASON_PEM_INVALID_BODY)
        }
    }
}

fn multikey_boundary_error(error: MultikeyError) -> CodecBoundaryError {
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
    };
    CodecBoundaryError::Multiformat(reason)
}

fn find_codec_spec(codec_name: &str) -> Option<(&'static str, &'static CodecSpec)> {
    MULTICODEC_TABLE
        .iter()
        .find(|(name, _)| *name == codec_name)
        .map(|(name, spec)| (*name, spec))
}

#[wasm_bindgen(js_name = multicodecPrefixForNameProto)]
/// Return multicodec metadata as a `CodecMulticodecSpec` protobuf message.
pub fn multicodec_prefix_for_name_proto(codec_name: &str) -> Result<Uint8Array, JsValue> {
    proto_bytes(multicodec_prefix_for_name_proto_output(codec_name)?)
}

#[wasm_bindgen(js_name = multicodecPrefixForNameProtoResult)]
/// Return multicodec metadata protobuf bytes with an explicit result status.
pub fn multicodec_prefix_for_name_proto_result(codec_name: &str) -> Result<JsValue, JsValue> {
    proto_result_object(multicodec_prefix_for_name_proto_output(codec_name)?)
}

fn multicodec_prefix_for_name_proto_output(codec_name: &str) -> Result<ProtoOutput, JsValue> {
    let Some((name, spec)) = find_codec_spec(codec_name) else {
        return Ok(encode_error_proto(CodecBoundaryError::Multiformat(
            CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC,
        )));
    };
    let message = codec_spec_proto(name, spec)?;
    Ok(encode_proto(&message))
}

#[wasm_bindgen(js_name = multicodecLookupPrefixProto)]
/// Resolve a prefix as a `CodecMulticodecLookupResult` protobuf message.
pub fn multicodec_lookup_prefix_proto(bytes: &Uint8Array) -> Result<Uint8Array, JsValue> {
    proto_bytes(multicodec_lookup_prefix_proto_output(bytes)?)
}

#[wasm_bindgen(js_name = multicodecLookupPrefixProtoResult)]
/// Resolve a prefix protobuf message with an explicit result status.
pub fn multicodec_lookup_prefix_proto_result(bytes: &Uint8Array) -> Result<JsValue, JsValue> {
    proto_result_object(multicodec_lookup_prefix_proto_output(bytes)?)
}

fn multicodec_lookup_prefix_proto_output(bytes: &Uint8Array) -> Result<ProtoOutput, JsValue> {
    let bytes = Zeroizing::new(bytes.to_vec());
    let Some(found) = lookup_codec_prefix(bytes.as_slice()) else {
        return Ok(encode_error_proto(CodecBoundaryError::Multiformat(
            CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTICODEC_PREFIX,
        )));
    };
    let spec = CodecSpec {
        tag: found.tag,
        key_material: found.key_material,
        alg: found.alg,
        codec: found.codec,
        key_length: found.key_length,
    };
    let prefix_length = usize_to_u32(found.codec.len())?;
    let metadata = codec_spec_proto(found.name, &spec)?;
    let result = CodecMulticodecLookupResult {
        name: found.name.to_owned(),
        prefix_length,
        metadata: Some(metadata).into(),
        __buffa_unknown_fields: Default::default(),
    };
    Ok(encode_proto(&result))
}

#[wasm_bindgen(js_name = multicodecTableProto)]
/// Return the supported multicodec table as protobuf bytes.
pub fn multicodec_table_proto() -> Result<Uint8Array, JsValue> {
    proto_bytes(multicodec_table_proto_output()?)
}

#[wasm_bindgen(js_name = multicodecTableProtoResult)]
/// Return the supported multicodec table protobuf bytes with an explicit result status.
pub fn multicodec_table_proto_result() -> Result<JsValue, JsValue> {
    proto_result_object(multicodec_table_proto_output()?)
}

fn multicodec_table_proto_output() -> Result<ProtoOutput, JsValue> {
    let mut entries = Vec::with_capacity(MULTICODEC_TABLE.len());
    for (name, spec) in MULTICODEC_TABLE {
        let entry = codec_spec_proto(name, spec)?;
        entries.push(entry);
    }
    let result = CodecMulticodecTableResult {
        entries,
        __buffa_unknown_fields: Default::default(),
    };
    Ok(encode_proto(&result))
}

#[wasm_bindgen(js_name = multikeyParseProto)]
/// Parse and validate a multikey as a `CodecMultikeyParseResult` protobuf message.
pub fn multikey_parse_proto(multikey: &str) -> Result<Uint8Array, JsValue> {
    proto_bytes(multikey_parse_proto_output(multikey)?)
}

#[wasm_bindgen(js_name = multikeyParseProtoResult)]
/// Parse and validate a multikey protobuf message with an explicit result status.
pub fn multikey_parse_proto_result(multikey: &str) -> Result<JsValue, JsValue> {
    proto_result_object(multikey_parse_proto_output(multikey)?)
}

fn multikey_parse_proto_output(multikey: &str) -> Result<ProtoOutput, JsValue> {
    let parsed = match parse_multikey(multikey) {
        Ok(parsed) => parsed,
        Err(error) => return Ok(encode_error_proto(multikey_boundary_error(error))),
    };
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
    Ok(encode_proto(&result))
}

#[wasm_bindgen(js_name = dagCborVerifyCidProto)]
/// Recompute and compare a CID as a `CodecDagCborVerifyCidResult` protobuf message.
pub fn dag_cbor_verify_cid_proto(cid: &str, bytes: &Uint8Array) -> Result<Uint8Array, JsValue> {
    proto_bytes(dag_cbor_verify_cid_proto_output(cid, bytes))
}

#[wasm_bindgen(js_name = dagCborVerifyCidProtoResult)]
/// Recompute and compare a CID protobuf message with an explicit result status.
pub fn dag_cbor_verify_cid_proto_result(cid: &str, bytes: &Uint8Array) -> Result<JsValue, JsValue> {
    proto_result_object(dag_cbor_verify_cid_proto_output(cid, bytes))
}

fn dag_cbor_verify_cid_proto_output(cid: &str, bytes: &Uint8Array) -> ProtoOutput {
    let bytes = Zeroizing::new(bytes.to_vec());
    if bytes.len() > MAX_DAG_CBOR_INPUT_LEN {
        return encode_error_proto(CodecBoundaryError::Canonicalization(
            CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_INVALID_CBOR,
        ));
    }
    let (valid, expected_cid, actual_cid) = verify_dag_cbor_cid(cid, bytes.as_slice());
    let result = CodecDagCborVerifyCidResult {
        valid,
        expected_cid,
        actual_cid,
        __buffa_unknown_fields: Default::default(),
    };
    encode_proto(&result)
}

#[wasm_bindgen(js_name = pemDecodeProto)]
/// Decode PEM text armor as a `CodecPemDecodeResult` protobuf message.
pub fn pem_decode_proto(input: &str, options_json: &str) -> Result<Uint8Array, JsValue> {
    proto_bytes(pem_decode_proto_output(input, options_json))
}

#[wasm_bindgen(js_name = pemDecodeProtoResult)]
/// Decode PEM text armor protobuf bytes with an explicit result status.
pub fn pem_decode_proto_result(input: &str, options_json: &str) -> Result<JsValue, JsValue> {
    proto_result_object(pem_decode_proto_output(input, options_json))
}

fn pem_decode_proto_output(input: &str, options_json: &str) -> ProtoOutput {
    let Ok(options) = parse_decode_options(options_json) else {
        return encode_error_proto(CodecBoundaryError::Canonicalization(
            CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_INVALID_JSON,
        ));
    };
    let labels = match options.allowed_labels {
        Some(labels) => {
            let mut parsed = Vec::with_capacity(labels.len());
            for label in labels {
                let Ok(label) = parse_label(&label) else {
                    return encode_error_proto(CodecBoundaryError::Pem(
                        CodecErrorReason::CODEC_ERROR_REASON_PEM_UNSUPPORTED_LABEL,
                    ));
                };
                parsed.push(label);
            }
            parsed
        }
        None => vec![
            PemLabel::PrivateKey,
            PemLabel::EcPrivateKey,
            PemLabel::PublicKey,
        ],
    };
    let policy = PemDecodePolicy {
        allowed_labels: &labels,
        max_input_len: options.max_input_len,
        max_der_len: options.max_der_len,
    };
    let decoded = match decode_pem(input, policy) {
        Ok(decoded) => decoded,
        Err(error) => return encode_error_proto(pem_boundary_error(error)),
    };
    let result = CodecPemDecodeResult {
        label: label_text(decoded.label).to_owned(),
        der: decoded.der.to_vec(),
        __buffa_unknown_fields: Default::default(),
    };
    encode_proto(&result)
}
