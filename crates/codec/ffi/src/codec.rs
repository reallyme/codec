// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use buffa::{EnumValue, Message};
use codec_base64::{base64_to_bytes, bytes_to_base64};
use codec_base64url::{base64url_to_bytes, bytes_to_base64url};
use codec_cbor::{
    compute_cid_dag_cbor, dag_cbor_multihash, decode_dag_cbor, encode_dag_cbor,
    is_valid_cid_string, sha2_256_content_hash, try_parse_cid, verify_dag_cbor_cid, CborValue,
    DAG_CBOR_CODEC, MAX_DAG_CBOR_INPUT_LEN,
};
use codec_hex::{bytes_to_lower_hex, lower_hex_to_bytes};
use codec_jcs::canonicalize_json;
use codec_multibase::{
    base58btc_decode, base58btc_encode, bytes_to_multibase58btc, bytes_to_multibase_base64url,
    multibase_to_bytes, MAX_BASE58BTC_INPUT_LEN,
};
use codec_multicodec::{
    lookup_codec_prefix, strip_codec_prefix, CodecSpec, CodecTag as MulticodecTag, KeyMaterialKind,
    MULTICODEC_TABLE, VARIABLE_KEY_LENGTH,
};
use codec_multikey::{
    binding_type_matches_codec, encode_multikey, parse_multikey, validate_key_binding,
    KeyBindingInput, MultikeyError,
};
use codec_pem::{
    decode_pem, encode_pem, PemDecodePolicy, PemEncodeOptions, PemError, PemLabel, PemLineEnding,
};
use codec_proto::generated::proto::reallyme::codec::v1::{
    CodecBaseEncodingError, CodecCanonicalizationError, CodecDagCborVerifyCidResult, CodecError,
    CodecErrorReason, CodecKeyMaterialKind, CodecMulticodecLookupResult, CodecMulticodecSpec,
    CodecMulticodecTableResult, CodecMultiformatError, CodecMultikeyParseResult,
    CodecPemDecodeResult, CodecPemError, CodecTag as ProtoCodecTag,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use zeroize::{Zeroize, Zeroizing};

use crate::guard::ffi_guard;
use crate::pointer::{read_slice, validate_output_len_pair, write_i32, write_len, write_slice};
use crate::status::{
    CodecStatus, CODEC_BUFFER_TOO_SMALL, CODEC_INTERNAL_ERROR, CODEC_INVALID_ARGUMENT, CODEC_OK,
    CODEC_PROTO_ERROR,
};

const DEFAULT_MAX_PEM_INPUT_LEN: usize = 1024 * 1024;
const DEFAULT_MAX_DER_LEN: usize = 1024 * 1024;
const DEFAULT_PEM_LINE_WIDTH: usize = 64;

const CODEC_BASE64_ENCODE: u32 = 1;
const CODEC_BASE64_DECODE: u32 = 2;
const CODEC_BASE64URL_ENCODE: u32 = 3;
const CODEC_BASE64URL_DECODE: u32 = 4;
const CODEC_LOWER_HEX_ENCODE: u32 = 5;
const CODEC_LOWER_HEX_DECODE: u32 = 6;
const CODEC_BASE58BTC_ENCODE: u32 = 7;
const CODEC_BASE58BTC_DECODE: u32 = 8;
const CODEC_MULTIBASE_BASE58BTC_ENCODE: u32 = 9;
const CODEC_MULTIBASE_BASE64URL_ENCODE: u32 = 10;
const CODEC_MULTIBASE_DECODE: u32 = 11;
const CODEC_MULTICODEC_PREFIX_FOR_NAME: u32 = 12;
const CODEC_MULTICODEC_LOOKUP_PREFIX: u32 = 13;
const CODEC_MULTICODEC_STRIP_PREFIX: u32 = 14;
const CODEC_MULTICODEC_TABLE: u32 = 15;
const CODEC_MULTIKEY_ENCODE: u32 = 16;
const CODEC_MULTIKEY_PARSE: u32 = 17;
const CODEC_REQUIRE_SUPPORTED_MULTICODEC: u32 = 18;
const CODEC_DAG_CBOR_ENCODE: u32 = 19;
const CODEC_DAG_CBOR_DECODE: u32 = 20;
const CODEC_DAG_CBOR_COMPUTE_CID: u32 = 21;
const CODEC_DAG_CBOR_VERIFY_CID: u32 = 22;
const CODEC_DAG_CBOR_SHA256_CONTENT_HASH: u32 = 23;
const CODEC_DAG_CBOR_MULTIHASH: u32 = 24;
const CODEC_TRY_PARSE_CID: u32 = 25;
const CODEC_DAG_CBOR_CODEC_CODE: u32 = 26;
const CODEC_CANONICALIZE_JSON: u32 = 27;
const CODEC_PEM_DECODE: u32 = 28;
const CODEC_PEM_ENCODE: u32 = 29;
const CODEC_VALIDATE_KEY_BINDING: u32 = 30;

const CODEC_BOOL_BINDING_TYPE_MATCHES_CODEC: u32 = 1;
const CODEC_BOOL_IS_VALID_CID_STRING: u32 = 2;

type CodecOutput = Zeroizing<Vec<u8>>;

struct ProtoOutput {
    bytes: CodecOutput,
    success_status: CodecStatus,
}

#[derive(Clone, Copy)]
enum CodecBoundaryError {
    BaseEncoding(CodecErrorReason),
    Pem(CodecErrorReason),
    Multiformat(CodecErrorReason),
    Canonicalization(CodecErrorReason),
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
enum TaggedCborValue {
    Null,
    Bool(bool),
    Int(i64),
    String(String),
    Bytes(String),
    Array(Vec<TaggedCborValue>),
    Map(Vec<TaggedCborMapEntry>),
}

#[derive(Deserialize, Serialize)]
struct TaggedCborMapEntry {
    key: String,
    value: TaggedCborValue,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecodeOptions {
    #[serde(default)]
    allowed_labels: Option<Vec<String>>,
    #[serde(default = "default_max_pem_input_len")]
    max_input_len: usize,
    #[serde(default = "default_max_der_len")]
    max_der_len: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncodeOptions {
    #[serde(default = "default_max_der_len")]
    max_der_len: usize,
    #[serde(default = "default_pem_line_width")]
    line_width: usize,
    #[serde(default)]
    line_ending: Option<String>,
}

#[derive(Serialize)]
struct PemDecodeJson<'a> {
    label: &'static str,
    der: &'a str,
}

fn default_max_pem_input_len() -> usize {
    DEFAULT_MAX_PEM_INPUT_LEN
}

fn default_max_der_len() -> usize {
    DEFAULT_MAX_DER_LEN
}

fn default_pem_line_width() -> usize {
    DEFAULT_PEM_LINE_WIDTH
}

fn read_text<'a>(ptr: *const u8, len: usize) -> Result<&'a str, CodecStatus> {
    // SAFETY: All exported ABI entry points define input pointers as borrowed
    // `(ptr, len)` byte ranges owned by the caller for the duration of the
    // call. `read_slice` validates null and oversized ranges before borrowing.
    let bytes = unsafe { read_slice(ptr, len) }?;
    core::str::from_utf8(bytes).map_err(|_| CODEC_INVALID_ARGUMENT)
}

fn empty_or_text<'a>(ptr: *const u8, len: usize) -> Result<&'a str, CodecStatus> {
    read_text(ptr, len)
}

fn write_output(
    output_ptr: *mut u8,
    output_len: usize,
    len_out: *mut usize,
    mut bytes: CodecOutput,
) -> CodecStatus {
    let status = validate_output_len_pair(output_ptr, output_len, len_out);
    if status != CODEC_OK {
        return status;
    }
    let produced_len = bytes.len();
    // SAFETY: `validate_output_len_pair` rejected null, misaligned, and
    // overlapping produced-length storage before this write.
    let len_status = unsafe { write_len(len_out, produced_len) };
    if len_status != CODEC_OK {
        return len_status;
    }
    if output_len < produced_len {
        bytes.zeroize();
        return CODEC_BUFFER_TOO_SMALL;
    }
    // SAFETY: `validate_output_len_pair` checked the output byte range and its
    // disjointness from `len_out`; `bytes` is an immutable Rust slice and the
    // subsequent copy writes only `bytes.len()` initialized bytes.
    let Ok(output) = (unsafe { write_slice(output_ptr, output_len) }) else {
        return CODEC_INVALID_ARGUMENT;
    };
    output[..produced_len].copy_from_slice(&bytes);
    bytes.zeroize();
    CODEC_OK
}

fn output_bytes(value: Vec<u8>) -> CodecOutput {
    Zeroizing::new(value)
}

fn text_bytes(value: String) -> CodecOutput {
    output_bytes(value.into_bytes())
}

fn text_str_bytes(value: &str) -> CodecOutput {
    let mut output = output_bytes(Vec::with_capacity(value.len()));
    output.extend_from_slice(value.as_bytes());
    output
}

fn json_bytes(value: serde_json::Value) -> Result<CodecOutput, CodecStatus> {
    let mut output = output_bytes(Vec::new());
    serde_json::to_writer(&mut *output, &value).map_err(|_| CODEC_INTERNAL_ERROR)?;
    Ok(output)
}

fn pem_decode_json_bytes(label: PemLabel, der: &str) -> Result<CodecOutput, CodecStatus> {
    let mut output = output_bytes(Vec::new());
    serde_json::to_writer(
        &mut *output,
        &PemDecodeJson {
            label: label_text(label),
            der,
        },
    )
    .map_err(|_| CODEC_INTERNAL_ERROR)?;
    Ok(output)
}

fn parse_label(label: &str) -> Result<PemLabel, CodecStatus> {
    match label {
        "PRIVATE KEY" => Ok(PemLabel::PrivateKey),
        "EC PRIVATE KEY" => Ok(PemLabel::EcPrivateKey),
        "PUBLIC KEY" => Ok(PemLabel::PublicKey),
        _ => Err(CODEC_INVALID_ARGUMENT),
    }
}

fn label_text(label: PemLabel) -> &'static str {
    match label {
        PemLabel::PrivateKey => "PRIVATE KEY",
        PemLabel::EcPrivateKey => "EC PRIVATE KEY",
        PemLabel::PublicKey => "PUBLIC KEY",
    }
}

fn parse_line_ending(line_ending: Option<&str>) -> Result<PemLineEnding, CodecStatus> {
    match line_ending {
        None | Some("lf") | Some("LF") => Ok(PemLineEnding::Lf),
        Some("crlf") | Some("CRLF") => Ok(PemLineEnding::Crlf),
        Some(_) => Err(CODEC_INVALID_ARGUMENT),
    }
}

fn parse_decode_options(options_json: &str) -> Result<DecodeOptions, CodecStatus> {
    if options_json.is_empty() {
        return Ok(DecodeOptions {
            allowed_labels: None,
            max_input_len: DEFAULT_MAX_PEM_INPUT_LEN,
            max_der_len: DEFAULT_MAX_DER_LEN,
        });
    }
    serde_json::from_str(options_json).map_err(|_| CODEC_INVALID_ARGUMENT)
}

fn parse_encode_options(options_json: &str) -> Result<EncodeOptions, CodecStatus> {
    if options_json.is_empty() {
        return Ok(EncodeOptions {
            max_der_len: DEFAULT_MAX_DER_LEN,
            line_width: DEFAULT_PEM_LINE_WIDTH,
            line_ending: None,
        });
    }
    serde_json::from_str(options_json).map_err(|_| CODEC_INVALID_ARGUMENT)
}

fn key_material_kind_text(kind: KeyMaterialKind) -> &'static str {
    match kind {
        KeyMaterialKind::PublicKey => "public-key",
        KeyMaterialKind::PrivateKey => "private-key",
        KeyMaterialKind::SymmetricKey => "symmetric-key",
        KeyMaterialKind::NotKey => "not-key",
    }
}

fn codec_tag_text(tag: MulticodecTag) -> &'static str {
    match tag {
        MulticodecTag::Encryption => "encryption",
        MulticodecTag::Key => "key",
        MulticodecTag::Hash => "hash",
        MulticodecTag::Multihash => "multihash",
        MulticodecTag::Multikey => "multikey",
    }
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

fn usize_to_u32(value: usize) -> Result<u32, CodecStatus> {
    u32::try_from(value).map_err(|_| CODEC_INTERNAL_ERROR)
}

fn codec_spec_proto(name: &str, spec: &CodecSpec) -> Result<CodecMulticodecSpec, CodecStatus> {
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

fn proto_bytes(message: &impl Message) -> ProtoOutput {
    let mut output = output_bytes(Vec::new());
    message.encode(&mut *output);
    ProtoOutput {
        bytes: output,
        success_status: CODEC_OK,
    }
}

fn codec_error_proto_bytes(error: CodecBoundaryError) -> ProtoOutput {
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
    let mut output = output_bytes(Vec::new());
    message.encode(&mut *output);
    ProtoOutput {
        bytes: output,
        success_status: CODEC_PROTO_ERROR,
    }
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

fn pem_decode_proto_bytes(label: PemLabel, der: &[u8]) -> ProtoOutput {
    proto_bytes(&CodecPemDecodeResult {
        label: label_text(label).to_owned(),
        der: der.to_vec(),
        __buffa_unknown_fields: Default::default(),
    })
}

fn codec_spec_json(name: &str, spec: &CodecSpec) -> serde_json::Value {
    let fixed_length = if spec.key_length == VARIABLE_KEY_LENGTH {
        serde_json::Value::Null
    } else {
        json!(spec.key_length)
    };
    json!({
        "name": name,
        "code": bytes_to_lower_hex(spec.codec),
        "prefix": bytes_to_base64url(spec.codec),
        "tag": codec_tag_text(spec.tag),
        "keyMaterialKind": key_material_kind_text(spec.key_material),
        "fixedLength": fixed_length,
    })
}

fn find_codec_spec(codec_name: &str) -> Option<(&'static str, &'static CodecSpec)> {
    MULTICODEC_TABLE
        .iter()
        .find(|(name, _)| *name == codec_name)
        .map(|(name, spec)| (*name, spec))
}

fn tagged_to_cbor(value: TaggedCborValue) -> Result<CborValue, CodecStatus> {
    match value {
        TaggedCborValue::Null => Ok(CborValue::Null),
        TaggedCborValue::Bool(value) => Ok(CborValue::Bool(value)),
        TaggedCborValue::Int(value) => Ok(CborValue::Int(value)),
        TaggedCborValue::String(value) => Ok(CborValue::String(value)),
        TaggedCborValue::Bytes(value) => {
            let bytes = base64url_to_bytes(&value).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            Ok(CborValue::Bytes(bytes))
        }
        TaggedCborValue::Array(values) => {
            let mut out = Vec::with_capacity(values.len());
            for value in values {
                out.push(tagged_to_cbor(value)?);
            }
            Ok(CborValue::Array(out))
        }
        TaggedCborValue::Map(entries) => {
            let mut out = Vec::with_capacity(entries.len());
            for entry in entries {
                out.push((entry.key, tagged_to_cbor(entry.value)?));
            }
            Ok(CborValue::Map(out))
        }
    }
}

fn cbor_to_tagged(value: CborValue) -> TaggedCborValue {
    match value {
        CborValue::Null => TaggedCborValue::Null,
        CborValue::Bool(value) => TaggedCborValue::Bool(value),
        CborValue::Int(value) => TaggedCborValue::Int(value),
        CborValue::String(value) => TaggedCborValue::String(value),
        CborValue::Bytes(value) => TaggedCborValue::Bytes(bytes_to_base64url(&value)),
        CborValue::Array(values) => {
            TaggedCborValue::Array(values.into_iter().map(cbor_to_tagged).collect())
        }
        CborValue::Map(entries) => TaggedCborValue::Map(
            entries
                .into_iter()
                .map(|(key, value)| TaggedCborMapEntry {
                    key,
                    value: cbor_to_tagged(value),
                })
                .collect(),
        ),
    }
}

fn process(
    operation: u32,
    first_ptr: *const u8,
    first_len: usize,
    second_ptr: *const u8,
    second_len: usize,
    third_ptr: *const u8,
    third_len: usize,
) -> Result<CodecOutput, CodecStatus> {
    // SAFETY: The C ABI contract supplies each input as a caller-owned byte
    // range valid for the duration of this call. `read_slice` validates null
    // pointers and impossible lengths before constructing borrowed slices.
    let first = unsafe { read_slice(first_ptr, first_len) }?;
    // SAFETY: Same ABI input contract and validation as `first`.
    let second = unsafe { read_slice(second_ptr, second_len) }?;
    // SAFETY: Same ABI input contract and validation as `first`.
    let _third = unsafe { read_slice(third_ptr, third_len) }?;
    match operation {
        CODEC_BASE64_ENCODE => Ok(text_bytes(bytes_to_base64(first))),
        CODEC_BASE64_DECODE => {
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            base64_to_bytes(text)
                .map(output_bytes)
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_BASE64URL_ENCODE => Ok(text_bytes(bytes_to_base64url(first))),
        CODEC_BASE64URL_DECODE => {
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            base64url_to_bytes(text)
                .map(output_bytes)
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_LOWER_HEX_ENCODE => Ok(text_bytes(bytes_to_lower_hex(first))),
        CODEC_LOWER_HEX_DECODE => {
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            lower_hex_to_bytes(text)
                .map(output_bytes)
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_BASE58BTC_ENCODE => base58btc_encode(first)
            .map(text_bytes)
            .map_err(|_| CODEC_INVALID_ARGUMENT),
        CODEC_BASE58BTC_DECODE => {
            if first.len() > MAX_BASE58BTC_INPUT_LEN {
                return Err(CODEC_INVALID_ARGUMENT);
            }
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            base58btc_decode(text)
                .map(output_bytes)
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_MULTIBASE_BASE58BTC_ENCODE => bytes_to_multibase58btc(first)
            .map(text_bytes)
            .map_err(|_| CODEC_INVALID_ARGUMENT),
        CODEC_MULTIBASE_BASE64URL_ENCODE => Ok(text_bytes(bytes_to_multibase_base64url(first))),
        CODEC_MULTIBASE_DECODE => {
            if first.first().copied() == Some(b'z') && first.len() > MAX_BASE58BTC_INPUT_LEN + 1 {
                return Err(CODEC_INVALID_ARGUMENT);
            }
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            multibase_to_bytes(text)
                .map(output_bytes)
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_MULTICODEC_PREFIX_FOR_NAME => {
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            let (name, spec) = find_codec_spec(text).ok_or(CODEC_INVALID_ARGUMENT)?;
            json_bytes(codec_spec_json(name, spec))
        }
        CODEC_MULTICODEC_LOOKUP_PREFIX => {
            let found = lookup_codec_prefix(first).ok_or(CODEC_INVALID_ARGUMENT)?;
            let spec = CodecSpec {
                tag: found.tag,
                key_material: found.key_material,
                alg: found.alg,
                codec: found.codec,
                key_length: found.key_length,
            };
            json_bytes(json!({
                "name": found.name,
                "prefixLength": found.codec.len(),
                "metadata": codec_spec_json(found.name, &spec),
            }))
        }
        CODEC_MULTICODEC_STRIP_PREFIX => Ok(output_bytes(strip_codec_prefix(first).to_vec())),
        CODEC_MULTICODEC_TABLE => {
            let entries: Vec<serde_json::Value> = MULTICODEC_TABLE
                .iter()
                .map(|(name, spec)| codec_spec_json(name, spec))
                .collect();
            json_bytes(json!(entries))
        }
        CODEC_MULTIKEY_ENCODE => {
            let codec_name = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            encode_multikey(codec_name, second)
                .map(text_bytes)
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_MULTIKEY_PARSE => {
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            let parsed = parse_multikey(text).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            let expected = if parsed.key_length == VARIABLE_KEY_LENGTH {
                serde_json::Value::Null
            } else {
                json!(parsed.key_length)
            };
            json_bytes(json!({
                "codecName": parsed.codec_name,
                "algorithmName": parsed.alg,
                "publicKey": bytes_to_base64url(&parsed.public_key),
                "expectedPublicKeyLength": expected,
            }))
        }
        CODEC_REQUIRE_SUPPORTED_MULTICODEC => {
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            find_codec_spec(text)
                .map(|_| output_bytes(Vec::new()))
                .ok_or(CODEC_INVALID_ARGUMENT)
        }
        CODEC_DAG_CBOR_ENCODE => {
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            let tagged: TaggedCborValue =
                serde_json::from_str(text).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            encode_dag_cbor(&tagged_to_cbor(tagged)?)
                .map(output_bytes)
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_DAG_CBOR_DECODE => {
            let value = decode_dag_cbor(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            serde_json::to_vec(&cbor_to_tagged(value))
                .map(output_bytes)
                .map_err(|_| CODEC_INTERNAL_ERROR)
        }
        CODEC_DAG_CBOR_COMPUTE_CID => {
            if first.len() > MAX_DAG_CBOR_INPUT_LEN {
                return Err(CODEC_INVALID_ARGUMENT);
            }
            Ok(text_bytes(compute_cid_dag_cbor(first)))
        }
        CODEC_DAG_CBOR_VERIFY_CID => {
            let cid = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            if second.len() > MAX_DAG_CBOR_INPUT_LEN {
                return Err(CODEC_INVALID_ARGUMENT);
            }
            let (valid, expected_cid, actual_cid) = verify_dag_cbor_cid(cid, second);
            json_bytes(json!({
                "valid": valid,
                "expectedCid": expected_cid,
                "actualCid": actual_cid,
            }))
        }
        CODEC_DAG_CBOR_SHA256_CONTENT_HASH => {
            if first.len() > MAX_DAG_CBOR_INPUT_LEN {
                return Err(CODEC_INVALID_ARGUMENT);
            }
            Ok(output_bytes(sha2_256_content_hash(first).to_vec()))
        }
        CODEC_DAG_CBOR_MULTIHASH => {
            if first.len() > MAX_DAG_CBOR_INPUT_LEN {
                return Err(CODEC_INVALID_ARGUMENT);
            }
            Ok(output_bytes(dag_cbor_multihash(first).to_bytes()))
        }
        CODEC_TRY_PARSE_CID => {
            let cid = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            try_parse_cid(cid)
                .map(|parsed| text_bytes(parsed.to_string()))
                .ok_or(CODEC_INVALID_ARGUMENT)
        }
        CODEC_DAG_CBOR_CODEC_CODE => Ok(text_bytes(DAG_CBOR_CODEC.to_string())),
        CODEC_CANONICALIZE_JSON => {
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            let value: serde_json::Value =
                serde_json::from_str(text).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            canonicalize_json(&value)
                .map(text_bytes)
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_PEM_DECODE => {
            let input = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            let options_json = core::str::from_utf8(second).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            let options = parse_decode_options(options_json)?;
            let labels = match options.allowed_labels {
                Some(labels) => {
                    let mut parsed = Vec::with_capacity(labels.len());
                    for label in labels {
                        parsed.push(parse_label(&label)?);
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
            let decoded = decode_pem(input, policy).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            let encoded_der = Zeroizing::new(bytes_to_base64url(decoded.der.as_slice()));
            pem_decode_json_bytes(decoded.label, encoded_der.as_str())
        }
        CODEC_PEM_ENCODE => {
            let label = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            // SAFETY: `third_ptr`/`third_len` is the options JSON byte range
            // supplied under the same C ABI input contract as the other
            // arguments; validation happens inside `read_slice`.
            let third = unsafe { read_slice(third_ptr, third_len) }?;
            let options_json = core::str::from_utf8(third).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            let options = parse_encode_options(options_json)?;
            let line_ending = parse_line_ending(options.line_ending.as_deref())?;
            let options = PemEncodeOptions {
                max_der_len: options.max_der_len,
                line_width: options.line_width,
                line_ending,
            };
            encode_pem(parse_label(label)?, second, options)
                .map(|value| text_str_bytes(value.as_str()))
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_VALIDATE_KEY_BINDING => {
            let binding_type = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            let algorithm = empty_or_text(second_ptr, second_len)?;
            let multikey = empty_or_text(third_ptr, third_len)?;
            let parsed = parse_multikey(multikey).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            let binding = KeyBindingInput {
                binding_type,
                algorithm: if algorithm.is_empty() {
                    None
                } else {
                    Some(algorithm)
                },
            };
            validate_key_binding(binding, &parsed)
                .map(|()| output_bytes(Vec::new()))
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        _ => Err(CODEC_INVALID_ARGUMENT),
    }
}

fn process_proto(
    operation: u32,
    first_ptr: *const u8,
    first_len: usize,
    second_ptr: *const u8,
    second_len: usize,
    third_ptr: *const u8,
    third_len: usize,
) -> Result<ProtoOutput, CodecStatus> {
    // SAFETY: The C ABI contract supplies each input as a caller-owned byte
    // range valid for the duration of this call. `read_slice` validates null
    // pointers and impossible lengths before constructing borrowed slices.
    let first = unsafe { read_slice(first_ptr, first_len) }?;
    // SAFETY: Same ABI input contract and validation as `first`.
    let second = unsafe { read_slice(second_ptr, second_len) }?;
    // SAFETY: Same ABI input contract and validation as `first`.
    let _third = unsafe { read_slice(third_ptr, third_len) }?;
    match operation {
        CODEC_MULTICODEC_PREFIX_FOR_NAME => {
            let Ok(text) = core::str::from_utf8(first) else {
                return Ok(codec_error_proto_bytes(CodecBoundaryError::Multiformat(
                    CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC,
                )));
            };
            let Some((name, spec)) = find_codec_spec(text) else {
                return Ok(codec_error_proto_bytes(CodecBoundaryError::Multiformat(
                    CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC,
                )));
            };
            Ok(proto_bytes(&codec_spec_proto(name, spec)?))
        }
        CODEC_MULTICODEC_LOOKUP_PREFIX => {
            let Some(found) = lookup_codec_prefix(first) else {
                return Ok(codec_error_proto_bytes(CodecBoundaryError::Multiformat(
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
            let result = CodecMulticodecLookupResult {
                name: found.name.to_owned(),
                prefix_length,
                metadata: Some(codec_spec_proto(found.name, &spec)?).into(),
                __buffa_unknown_fields: Default::default(),
            };
            Ok(proto_bytes(&result))
        }
        CODEC_MULTICODEC_TABLE => {
            let mut entries = Vec::with_capacity(MULTICODEC_TABLE.len());
            for (name, spec) in MULTICODEC_TABLE {
                entries.push(codec_spec_proto(name, spec)?);
            }
            let result = CodecMulticodecTableResult {
                entries,
                __buffa_unknown_fields: Default::default(),
            };
            Ok(proto_bytes(&result))
        }
        CODEC_MULTIKEY_PARSE => {
            let Ok(text) = core::str::from_utf8(first) else {
                return Ok(codec_error_proto_bytes(CodecBoundaryError::Multiformat(
                    CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTIKEY,
                )));
            };
            let parsed = match parse_multikey(text) {
                Ok(parsed) => parsed,
                Err(error) => return Ok(codec_error_proto_bytes(multikey_boundary_error(error))),
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
            Ok(proto_bytes(&result))
        }
        CODEC_DAG_CBOR_VERIFY_CID => {
            let Ok(cid) = core::str::from_utf8(first) else {
                return Ok(codec_error_proto_bytes(CodecBoundaryError::Multiformat(
                    CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTIBASE_PREFIX,
                )));
            };
            if second.len() > MAX_DAG_CBOR_INPUT_LEN {
                return Ok(codec_error_proto_bytes(
                    CodecBoundaryError::Canonicalization(
                        CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_INVALID_CBOR,
                    ),
                ));
            }
            let (valid, expected_cid, actual_cid) = verify_dag_cbor_cid(cid, second);
            let result = CodecDagCborVerifyCidResult {
                valid,
                expected_cid,
                actual_cid,
                __buffa_unknown_fields: Default::default(),
            };
            Ok(proto_bytes(&result))
        }
        CODEC_PEM_DECODE => {
            let Ok(input) = core::str::from_utf8(first) else {
                return Ok(codec_error_proto_bytes(CodecBoundaryError::Pem(
                    CodecErrorReason::CODEC_ERROR_REASON_PEM_INVALID_BODY,
                )));
            };
            let Ok(options_json) = core::str::from_utf8(second) else {
                return Ok(codec_error_proto_bytes(
                    CodecBoundaryError::Canonicalization(
                        CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_INVALID_JSON,
                    ),
                ));
            };
            let Ok(options) = parse_decode_options(options_json) else {
                return Ok(codec_error_proto_bytes(
                    CodecBoundaryError::Canonicalization(
                        CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_INVALID_JSON,
                    ),
                ));
            };
            let labels = match options.allowed_labels {
                Some(labels) => {
                    let mut parsed = Vec::with_capacity(labels.len());
                    for label in labels {
                        let Ok(label) = parse_label(&label) else {
                            return Ok(codec_error_proto_bytes(CodecBoundaryError::Pem(
                                CodecErrorReason::CODEC_ERROR_REASON_PEM_UNSUPPORTED_LABEL,
                            )));
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
                Err(error) => {
                    return Ok(codec_error_proto_bytes(pem_boundary_error(error)));
                }
            };
            Ok(pem_decode_proto_bytes(
                decoded.label,
                decoded.der.as_slice(),
            ))
        }
        _ => Err(CODEC_INVALID_ARGUMENT),
    }
}

fn process_bool(
    operation: u32,
    first_ptr: *const u8,
    first_len: usize,
    second_ptr: *const u8,
    second_len: usize,
) -> Result<i32, CodecStatus> {
    let first = read_text(first_ptr, first_len)?;
    let second = read_text(second_ptr, second_len)?;
    match operation {
        CODEC_BOOL_BINDING_TYPE_MATCHES_CODEC => {
            Ok(i32::from(binding_type_matches_codec(first, second)))
        }
        CODEC_BOOL_IS_VALID_CID_STRING => Ok(i32::from(is_valid_cid_string(first))),
        _ => Err(CODEC_INVALID_ARGUMENT),
    }
}

/// Run a ReallyMe codec operation through the shared Rust implementation.
///
/// Text arguments and text outputs are UTF-8 bytes. Structured outputs use
/// compact JSON at this ABI boundary so Swift/Kotlin packages can keep typed
/// public methods without reimplementing DAG-CBOR, multicodec, or PEM logic.
///
/// # Safety
///
/// Non-empty input ranges must point to initialized caller-owned bytes that
/// remain valid for the duration of the call. The implementation copies all
/// operation results into Rust-owned memory before mutating output, so callers
/// may use the same byte storage for input and output when their platform ABI
/// permits it. Non-empty output ranges must point to writable caller-owned
/// bytes and must not alias `len_out`. `len_out` must point to writable,
/// aligned `usize` storage.
#[no_mangle]
pub unsafe extern "C" fn rm_codec_process(
    operation: u32,
    first_ptr: *const u8,
    first_len: usize,
    second_ptr: *const u8,
    second_len: usize,
    third_ptr: *const u8,
    third_len: usize,
    output_ptr: *mut u8,
    output_len: usize,
    len_out: *mut usize,
) -> CodecStatus {
    ffi_guard(|| {
        let output = match process(
            operation, first_ptr, first_len, second_ptr, second_len, third_ptr, third_len,
        ) {
            Ok(value) => value,
            Err(status) => return status,
        };
        write_output(output_ptr, output_len, len_out, output)
    })
}

/// Run a ReallyMe codec operation and encode supported structured outputs as
/// protobuf bytes. Operations that naturally return raw bytes or text continue
/// to use [`rm_codec_process`]; this entry point is for fixed-shape boundary
/// results such as multikey parsing and PEM metadata.
///
/// # Safety
///
/// Non-empty input ranges must point to initialized caller-owned bytes that
/// remain valid for the duration of the call. The implementation copies all
/// operation results into Rust-owned memory before mutating output, so callers
/// may use the same byte storage for input and output when their platform ABI
/// permits it. Non-empty output ranges must point to writable caller-owned
/// bytes and must not alias `len_out`. `len_out` must point to writable,
/// aligned `usize` storage.
#[no_mangle]
pub unsafe extern "C" fn rm_codec_process_proto(
    operation: u32,
    first_ptr: *const u8,
    first_len: usize,
    second_ptr: *const u8,
    second_len: usize,
    third_ptr: *const u8,
    third_len: usize,
    output_ptr: *mut u8,
    output_len: usize,
    len_out: *mut usize,
) -> CodecStatus {
    ffi_guard(|| {
        let output = match process_proto(
            operation, first_ptr, first_len, second_ptr, second_len, third_ptr, third_len,
        ) {
            Ok(value) => value,
            Err(status) => return status,
        };
        let status = write_output(output_ptr, output_len, len_out, output.bytes);
        if status == CODEC_OK {
            output.success_status
        } else {
            status
        }
    })
}

/// Run a ReallyMe codec predicate through the shared Rust implementation.
///
/// # Safety
///
/// Non-empty input ranges must point to initialized caller-owned bytes that
/// remain valid for the duration of the call. `result_out` must point to
/// writable, aligned `i32` storage.
#[no_mangle]
pub unsafe extern "C" fn rm_codec_process_bool(
    operation: u32,
    first_ptr: *const u8,
    first_len: usize,
    second_ptr: *const u8,
    second_len: usize,
    result_out: *mut i32,
) -> CodecStatus {
    ffi_guard(|| {
        let value = match process_bool(operation, first_ptr, first_len, second_ptr, second_len) {
            Ok(value) => value,
            Err(status) => return status,
        };
        // SAFETY: `result_out` is governed by this export's safety contract,
        // and `write_i32` validates null and alignment before writing.
        unsafe { write_i32(result_out, value) }
    })
}

#[cfg(test)]
mod tests {
    use super::{
        rm_codec_process, rm_codec_process_proto, CODEC_BASE58BTC_DECODE, CODEC_BASE58BTC_ENCODE,
        CODEC_BASE64_ENCODE, CODEC_MULTIKEY_PARSE, MAX_BASE58BTC_INPUT_LEN,
    };
    use crate::status::{
        CODEC_BUFFER_TOO_SMALL, CODEC_INVALID_ARGUMENT, CODEC_OK, CODEC_PROTO_ERROR,
    };

    #[test]
    fn base58_decode_rejects_inputs_above_ffi_cap() {
        let input = vec![b'1'; MAX_BASE58BTC_INPUT_LEN + 1];
        let mut produced_len = 0_usize;

        // SAFETY: The input vector and produced-length output are valid for
        // the duration of this call. No output byte buffer is supplied.
        let status = unsafe {
            rm_codec_process(
                CODEC_BASE58BTC_DECODE,
                input.as_ptr(),
                input.len(),
                core::ptr::null(),
                0,
                core::ptr::null(),
                0,
                core::ptr::null_mut(),
                0,
                &mut produced_len,
            )
        };

        assert_eq!(status, CODEC_INVALID_ARGUMENT);
        assert_eq!(produced_len, 0);
    }

    #[test]
    fn base58_encode_rejects_inputs_above_ffi_cap() {
        let input = vec![0_u8; MAX_BASE58BTC_INPUT_LEN + 1];
        let mut produced_len = 0_usize;

        // SAFETY: The input vector and produced-length output are valid for
        // the duration of this call. No output byte buffer is supplied.
        let status = unsafe {
            rm_codec_process(
                CODEC_BASE58BTC_ENCODE,
                input.as_ptr(),
                input.len(),
                core::ptr::null(),
                0,
                core::ptr::null(),
                0,
                core::ptr::null_mut(),
                0,
                &mut produced_len,
            )
        };

        assert_eq!(status, CODEC_INVALID_ARGUMENT);
        assert_eq!(produced_len, 0);
    }

    #[test]
    fn proto_error_envelope_uses_distinct_success_status() {
        let input = b"not-a-key";
        let mut produced_len = 0_usize;

        // SAFETY: The input vector and produced-length output are valid for
        // the duration of this call. No output byte buffer is supplied.
        let probe_status = unsafe {
            rm_codec_process_proto(
                CODEC_MULTIKEY_PARSE,
                input.as_ptr(),
                input.len(),
                core::ptr::null(),
                0,
                core::ptr::null(),
                0,
                core::ptr::null_mut(),
                0,
                &mut produced_len,
            )
        };
        assert_eq!(probe_status, CODEC_BUFFER_TOO_SMALL);
        assert!(produced_len > 0);

        let mut output = vec![0_u8; produced_len];
        // SAFETY: All pointers describe valid caller-owned storage for the
        // duration of this call.
        let status = unsafe {
            rm_codec_process_proto(
                CODEC_MULTIKEY_PARSE,
                input.as_ptr(),
                input.len(),
                core::ptr::null(),
                0,
                core::ptr::null(),
                0,
                output.as_mut_ptr(),
                output.len(),
                &mut produced_len,
            )
        };

        assert_eq!(status, CODEC_PROTO_ERROR);
        assert!(produced_len > 0);
    }

    #[test]
    fn process_allows_input_output_alias_after_result_is_copied() {
        let mut buffer = *b"abc\0";
        let mut produced_len = 0_usize;

        // SAFETY: The input and output ranges are valid for the call. This
        // deliberately aliases input and output to cover the documented
        // copy-then-write invariant.
        let status = unsafe {
            rm_codec_process(
                CODEC_BASE64_ENCODE,
                buffer.as_ptr(),
                3,
                core::ptr::null(),
                0,
                core::ptr::null(),
                0,
                buffer.as_mut_ptr(),
                buffer.len(),
                &mut produced_len,
            )
        };

        assert_eq!(status, CODEC_OK);
        assert_eq!(produced_len, 4);
        assert_eq!(&buffer, b"YWJj");
    }

    #[test]
    fn process_reports_length_before_rejecting_short_output_buffer() {
        let input = b"abc";
        let mut output = [0_u8; 3];
        let mut produced_len = 0_usize;

        // SAFETY: All pointers describe valid caller-owned storage for the
        // duration of this call.
        let status = unsafe {
            rm_codec_process(
                CODEC_BASE64_ENCODE,
                input.as_ptr(),
                input.len(),
                core::ptr::null(),
                0,
                core::ptr::null(),
                0,
                output.as_mut_ptr(),
                output.len(),
                &mut produced_len,
            )
        };

        assert_eq!(status, CODEC_BUFFER_TOO_SMALL);
        assert_eq!(produced_len, 4);
        assert_eq!(output, [0_u8; 3]);
    }
}
