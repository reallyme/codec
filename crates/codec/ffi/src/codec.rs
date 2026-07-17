// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_base64::{base64_to_bytes, bytes_to_base64};
use codec_base64url::{base64url_to_bytes, bytes_to_base64url};
use codec_cbor::{
    compute_cid_dag_cbor, dag_cbor_multihash, decode_dag_cbor, encode_dag_cbor,
    is_valid_cid_string, sha2_256_content_hash, try_parse_cid, verify_dag_cbor_cid, CborValue,
    DAG_CBOR_CODEC, MAX_DAG_CBOR_INPUT_LEN,
};
use codec_hex::{bytes_to_lower_hex, lower_hex_to_bytes};
use codec_jcs::canonicalize_json_text;
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
    KeyBindingInput,
};
use codec_pem::{
    decode_pem, encode_pem, PemDecodePolicy, PemEncodeOptions, PemLabel, PemLineEnding,
};
use codec_runtime::proto_process::{process_proto, process_proto_json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{self, Write};
use zeroize::{Zeroize, Zeroizing};

use crate::guard::ffi_guard;
use crate::pointer::{read_slice, validate_output_len_pair, write_i32, write_len, write_slice};
use crate::status::{
    CodecStatus, CODEC_BUFFER_TOO_SMALL, CODEC_INTERNAL_ERROR, CODEC_INVALID_ARGUMENT, CODEC_OK,
};

const DEFAULT_MAX_PEM_INPUT_LEN: usize = 1024 * 1024;
const DEFAULT_MAX_DER_LEN: usize = 1024 * 1024;
const DEFAULT_PEM_LINE_WIDTH: usize = 64;

/// Maximum aggregate caller-controlled input accepted by the generic C ABI.
///
/// This check runs before decoding or copying so every operation, including
/// future dispatch additions, inherits a finite allocation budget.
pub(crate) const MAX_CODEC_FFI_INPUT_BYTES: usize = 1024 * 1024;

/// Maximum generic result accepted by SDK adapters after an FFI size probe.
///
/// DAG-CBOR can expand substantially when represented as tagged JSON, so this
/// is deliberately larger than the input budget. Keeping it finite prevents a
/// corrupted or mismatched provider from turning the two-pass ABI into an
/// unbounded allocation request in Swift or JNI callers.
pub(crate) const MAX_CODEC_FFI_OUTPUT_BYTES: usize = 64 * 1024 * 1024;

/// Version of the exported C function signatures and calling conventions.
///
/// SDKs must reject a library that does not expose this exact value before
/// casting or calling any other dynamically resolved symbol.
pub const CODEC_ABI_VERSION: u32 = 2;

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

/// Returns the exact C ABI contract version implemented by this library.
///
/// This leaf function only returns a compile-time constant and therefore has
/// no operation that can panic or unwind across the C boundary.
#[no_mangle]
pub extern "C" fn rm_codec_abi_version() -> u32 {
    CODEC_ABI_VERSION
}

/// Returns the authoritative maximum encoded protobuf result-envelope size.
///
/// Native SDKs query this value only after validating [`CODEC_ABI_VERSION`],
/// removing hardcoded cross-language copies of the protocol allocation bound.
#[no_mangle]
pub extern "C" fn rm_codec_max_proto_result_envelope_bytes() -> usize {
    codec_proto::MAX_CODEC_PROTO_RESULT_ENVELOPE_BYTES
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

fn validate_boundary_input_lengths(lengths: &[usize]) -> Result<(), CodecStatus> {
    let mut aggregate = 0_usize;
    for length in lengths {
        aggregate = aggregate
            .checked_add(*length)
            .ok_or(CODEC_INVALID_ARGUMENT)?;
        if aggregate > MAX_CODEC_FFI_INPUT_BYTES {
            return Err(CODEC_INVALID_ARGUMENT);
        }
    }
    Ok(())
}

fn validate_proto_boundary_input_length(request_len: usize) -> Result<(), CodecStatus> {
    // Native SDKs use one bounded byte beyond the ProtoJSON limit as a
    // sentinel so the core can return its stable resource-limit envelope.
    let maximum = codec_proto::MAX_CODEC_PROTO_JSON_BYTES
        .checked_add(1)
        .ok_or(CODEC_INVALID_ARGUMENT)?;
    if request_len > maximum {
        return Err(CODEC_INVALID_ARGUMENT);
    }
    Ok(())
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
    if bytes.len() > MAX_CODEC_FFI_OUTPUT_BYTES {
        bytes.zeroize();
        return CODEC_INTERNAL_ERROR;
    }
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

fn initialize_output_length(
    output_ptr: *mut u8,
    output_len: usize,
    len_out: *mut usize,
) -> CodecStatus {
    let status = validate_output_len_pair(output_ptr, output_len, len_out);
    if status != CODEC_OK {
        return status;
    }
    // SAFETY: The shared pair validator rejected a null, misaligned, or
    // overlapping produced-length pointer before this initialization.
    unsafe { write_len(len_out, 0) }
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

#[derive(Default)]
struct JsonLengthCounter {
    length: usize,
    overflowed: bool,
}

impl Write for JsonLengthCounter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self.length.checked_add(bytes.len()) {
            Some(length) => self.length = length,
            None => self.overflowed = true,
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct FixedJsonWriter<'a> {
    bytes: &'a mut [u8],
    written: usize,
}

impl Write for FixedJsonWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let end = self
            .written
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::from(io::ErrorKind::WriteZero))?;
        let destination = self
            .bytes
            .get_mut(self.written..end)
            .ok_or_else(|| io::Error::from(io::ErrorKind::WriteZero))?;
        destination.copy_from_slice(bytes);
        self.written = end;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn json_bytes<T: Serialize>(value: T) -> Result<CodecOutput, CodecStatus> {
    // Count first without retaining any serialized bytes, then serialize into
    // one fixed-size allocation. A growable Vec would free secret-bearing
    // intermediate allocations without giving `Zeroizing` a chance to wipe
    // them during PEM decode and document-to-JSON operations.
    let mut counter = JsonLengthCounter::default();
    serde_json::to_writer(&mut counter, &value).map_err(|_| CODEC_INTERNAL_ERROR)?;
    if counter.overflowed || counter.length > MAX_CODEC_FFI_OUTPUT_BYTES {
        return Err(CODEC_INTERNAL_ERROR);
    }

    let mut output = output_bytes(vec![0_u8; counter.length]);
    let written = {
        let mut writer = FixedJsonWriter {
            bytes: output.as_mut_slice(),
            written: 0,
        };
        serde_json::to_writer(&mut writer, &value).map_err(|_| CODEC_INTERNAL_ERROR)?;
        writer.written
    };
    if written != counter.length {
        return Err(CODEC_INTERNAL_ERROR);
    }
    Ok(output)
}

fn pem_decode_json_bytes(label: PemLabel, der: &str) -> Result<CodecOutput, CodecStatus> {
    json_bytes(PemDecodeJson {
        label: label_text(label)?,
        der,
    })
}

fn parse_label(label: &str) -> Result<PemLabel, CodecStatus> {
    match label {
        "PRIVATE KEY" => Ok(PemLabel::PrivateKey),
        "EC PRIVATE KEY" => Ok(PemLabel::EcPrivateKey),
        "PUBLIC KEY" => Ok(PemLabel::PublicKey),
        _ => Err(CODEC_INVALID_ARGUMENT),
    }
}

fn label_text(label: PemLabel) -> Result<&'static str, CodecStatus> {
    match label {
        PemLabel::PrivateKey => Ok("PRIVATE KEY"),
        PemLabel::EcPrivateKey => Ok("EC PRIVATE KEY"),
        PemLabel::PublicKey => Ok("PUBLIC KEY"),
        _ => Err(CODEC_INTERNAL_ERROR),
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

fn key_material_kind_text(kind: KeyMaterialKind) -> Result<&'static str, CodecStatus> {
    match kind {
        KeyMaterialKind::PublicKey => Ok("public-key"),
        KeyMaterialKind::PrivateKey => Ok("private-key"),
        KeyMaterialKind::SymmetricKey => Ok("symmetric-key"),
        KeyMaterialKind::NotKey => Ok("not-key"),
        _ => Err(CODEC_INTERNAL_ERROR),
    }
}

fn codec_tag_text(tag: MulticodecTag) -> Result<&'static str, CodecStatus> {
    match tag {
        MulticodecTag::Encryption => Ok("encryption"),
        MulticodecTag::Key => Ok("key"),
        MulticodecTag::Hash => Ok("hash"),
        MulticodecTag::Multihash => Ok("multihash"),
        MulticodecTag::Multikey => Ok("multikey"),
        _ => Err(CODEC_INTERNAL_ERROR),
    }
}

fn codec_spec_json(name: &str, spec: &CodecSpec) -> Result<serde_json::Value, CodecStatus> {
    let fixed_length = if spec.key_length == VARIABLE_KEY_LENGTH {
        serde_json::Value::Null
    } else {
        json!(spec.key_length)
    };
    Ok(json!({
        "name": name,
        "code": bytes_to_lower_hex(spec.codec),
        "prefix": bytes_to_base64url(spec.codec),
        "tag": codec_tag_text(spec.tag)?,
        "keyMaterialKind": key_material_kind_text(spec.key_material)?,
        "fixedLength": fixed_length,
    }))
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

fn cbor_to_tagged(value: CborValue) -> Result<TaggedCborValue, CodecStatus> {
    match value {
        CborValue::Null => Ok(TaggedCborValue::Null),
        CborValue::Bool(value) => Ok(TaggedCborValue::Bool(value)),
        CborValue::Int(value) => Ok(TaggedCborValue::Int(value)),
        CborValue::String(value) => Ok(TaggedCborValue::String(value)),
        CborValue::Bytes(value) => Ok(TaggedCborValue::Bytes(bytes_to_base64url(&value))),
        CborValue::Array(values) => {
            let values = values
                .into_iter()
                .map(cbor_to_tagged)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(TaggedCborValue::Array(values))
        }
        CborValue::Map(entries) => {
            let mut values = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                values.push(TaggedCborMapEntry {
                    key,
                    value: cbor_to_tagged(value)?,
                });
            }
            Ok(TaggedCborValue::Map(values))
        }
        _ => Err(CODEC_INTERNAL_ERROR),
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
    validate_boundary_input_lengths(&[first_len, second_len, third_len])?;
    // SAFETY: The C ABI contract supplies each input as a caller-owned byte
    // range valid for the duration of this call. `read_slice` validates null
    // pointers and impossible lengths before constructing borrowed slices.
    let first = unsafe { read_slice(first_ptr, first_len) }?;
    // SAFETY: Same ABI input contract and validation as `first`.
    let second = unsafe { read_slice(second_ptr, second_len) }?;
    // SAFETY: Same ABI input contract and validation as `first`.
    let third = unsafe { read_slice(third_ptr, third_len) }?;
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
            let max_multibase_len = MAX_BASE58BTC_INPUT_LEN
                .checked_add(1)
                .ok_or(CODEC_INVALID_ARGUMENT)?;
            if first.first().copied() == Some(b'z') && first.len() > max_multibase_len {
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
            json_bytes(codec_spec_json(name, spec)?)
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
            let metadata = codec_spec_json(found.name, &spec)?;
            json_bytes(json!({
                "name": found.name,
                "prefixLength": found.codec.len(),
                "metadata": metadata,
            }))
        }
        CODEC_MULTICODEC_STRIP_PREFIX => Ok(output_bytes(strip_codec_prefix(first).to_vec())),
        CODEC_MULTICODEC_TABLE => {
            let entries = MULTICODEC_TABLE
                .iter()
                .map(|(name, spec)| codec_spec_json(name, spec))
                .collect::<Result<Vec<_>, _>>()?;
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
            json_bytes(cbor_to_tagged(value)?)
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
            canonicalize_json_text(text)
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

fn process_bool(
    operation: u32,
    first_ptr: *const u8,
    first_len: usize,
    second_ptr: *const u8,
    second_len: usize,
) -> Result<i32, CodecStatus> {
    validate_boundary_input_lengths(&[first_len, second_len])?;
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
/// aligned `usize` storage. Once those output pointers validate, `len_out` is
/// initialized to zero before inputs are processed. A buffer-too-small result
/// replaces it with the required length; every other failure leaves it zero.
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
        let output_status = initialize_output_length(output_ptr, output_len, len_out);
        if output_status != CODEC_OK {
            return output_status;
        }
        let output = match process(
            operation, first_ptr, first_len, second_ptr, second_len, third_ptr, third_len,
        ) {
            Ok(value) => value,
            Err(status) => return status,
        };
        write_output(output_ptr, output_len, len_out, output)
    })
}

/// Executes one self-describing protobuf codec request.
///
/// The output is always a binary `CodecProtoResultEnvelope`. The C status
/// reports only ABI success or failure; operation results and structured codec
/// errors are carried exclusively inside the envelope.
///
/// # Safety
///
/// Non-empty input ranges must point to initialized caller-owned bytes that
/// remain valid for the duration of the call. The implementation copies all
/// operation results into Rust-owned memory before mutating output, so callers
/// may use the same byte storage for input and output when their platform ABI
/// permits it. Non-empty output ranges must point to writable caller-owned
/// bytes and must not alias `len_out`. `len_out` must point to writable,
/// aligned `usize` storage. Once validated, `len_out` follows the initialized
/// failure semantics documented by [`rm_codec_process`].
#[no_mangle]
pub unsafe extern "C" fn rm_codec_process_proto(
    request_ptr: *const u8,
    request_len: usize,
    output_ptr: *mut u8,
    output_len: usize,
    len_out: *mut usize,
) -> CodecStatus {
    ffi_guard(|| {
        let output_status = initialize_output_length(output_ptr, output_len, len_out);
        if output_status != CODEC_OK {
            return output_status;
        }
        if let Err(status) = validate_proto_boundary_input_length(request_len) {
            return status;
        }
        // SAFETY: The request follows the same caller-owned pointer/length
        // contract as every other byte input in this module.
        let request = match unsafe { read_slice(request_ptr, request_len) } {
            Ok(value) => value,
            Err(status) => return status,
        };
        write_output(output_ptr, output_len, len_out, process_proto(request))
    })
}

/// Executes one generated ProtoJSON codec request.
///
/// The output is always the same binary `CodecProtoResultEnvelope` returned by
/// [`rm_codec_process_proto`]. JSON is therefore an input convenience only and
/// cannot create a second result model.
///
/// # Safety
///
/// The pointer, length, output, aliasing, and ownership requirements are
/// identical to [`rm_codec_process_proto`].
#[no_mangle]
pub unsafe extern "C" fn rm_codec_process_proto_json(
    request_ptr: *const u8,
    request_len: usize,
    output_ptr: *mut u8,
    output_len: usize,
    len_out: *mut usize,
) -> CodecStatus {
    ffi_guard(|| {
        let output_status = initialize_output_length(output_ptr, output_len, len_out);
        if output_status != CODEC_OK {
            return output_status;
        }
        if let Err(status) = validate_proto_boundary_input_length(request_len) {
            return status;
        }
        // SAFETY: The request follows the caller-owned pointer/length contract
        // documented by this export and validated by the shared helper.
        let request = match unsafe { read_slice(request_ptr, request_len) } {
            Ok(value) => value,
            Err(status) => return status,
        };
        write_output(output_ptr, output_len, len_out, process_proto_json(request))
    })
}

/// Run a ReallyMe codec predicate through the shared Rust implementation.
///
/// # Safety
///
/// Non-empty input ranges must point to initialized caller-owned bytes that
/// remain valid for the duration of the call. `result_out` must point to
/// writable, aligned `i32` storage. Once validated, `result_out` is initialized
/// to false (`0`) before inputs are processed and remains false on failure.
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
        // SAFETY: `write_i32` validates null and alignment before writing the
        // deterministic failure value.
        let result_status = unsafe { write_i32(result_out, 0) };
        if result_status != CODEC_OK {
            return result_status;
        }
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
    use codec_proto::generated::proto::reallyme::codec::v1::CodecErrorReason;
    use codec_proto::generated::proto::reallyme::codec::v1::{
        __buffa::oneof::codec_operation_request::Operation as CodecOperation,
        CodecMultikeyParseRequest, CodecOperationRequest,
    };
    use codec_proto::CodecProtoStatus;
    use codec_proto::{
        decode_codec_error_payload, decode_proto_result_envelope, encode_protobuf,
        CodecWireErrorBranch,
    };

    use super::{
        json_bytes, rm_codec_abi_version, rm_codec_max_proto_result_envelope_bytes,
        rm_codec_process, rm_codec_process_bool, rm_codec_process_proto,
        rm_codec_process_proto_json, validate_boundary_input_lengths, CODEC_ABI_VERSION,
        CODEC_BASE58BTC_DECODE, CODEC_BASE58BTC_ENCODE, CODEC_BASE64_ENCODE,
        CODEC_CANONICALIZE_JSON, CODEC_DAG_CBOR_ENCODE, MAX_BASE58BTC_INPUT_LEN,
        MAX_CODEC_FFI_INPUT_BYTES,
    };
    use crate::status::{CODEC_BUFFER_TOO_SMALL, CODEC_INVALID_ARGUMENT, CODEC_OK};

    #[test]
    fn abi_version_export_matches_the_sdk_contract() {
        assert_eq!(rm_codec_abi_version(), CODEC_ABI_VERSION);
        assert_eq!(
            rm_codec_max_proto_result_envelope_bytes(),
            codec_proto::MAX_CODEC_PROTO_RESULT_ENVELOPE_BYTES
        );
    }

    #[test]
    fn json_output_is_written_into_its_precomputed_fixed_buffer() {
        let output = json_bytes(serde_json::json!({
            "label": "PRIVATE KEY",
            "der": "c2Vuc2l0aXZlLWRlcg"
        }));
        assert!(output.is_ok());
        let Some(output) = output.ok() else {
            return;
        };
        assert_eq!(
            output.as_slice(),
            br#"{"der":"c2Vuc2l0aXZlLWRlcg","label":"PRIVATE KEY"}"#
        );
    }

    #[test]
    fn aggregate_ffi_limit_rejects_oversize_and_integer_overflow() {
        assert!(validate_boundary_input_lengths(&[MAX_CODEC_FFI_INPUT_BYTES]).is_ok());
        assert_eq!(
            validate_boundary_input_lengths(&[MAX_CODEC_FFI_INPUT_BYTES, 1]),
            Err(CODEC_INVALID_ARGUMENT)
        );
        assert_eq!(
            validate_boundary_input_lengths(&[usize::MAX, 1]),
            Err(CODEC_INVALID_ARGUMENT)
        );
    }

    #[test]
    fn protobuf_ffi_exports_return_resource_limit_envelopes_for_oversized_input() {
        type ProcessProto =
            unsafe extern "C" fn(*const u8, usize, *mut u8, usize, *mut usize) -> i32;
        let cases = [
            (
                rm_codec_process_proto as ProcessProto,
                codec_proto::MAX_CODEC_PROTO_MESSAGE_BYTES,
            ),
            (
                rm_codec_process_proto_json as ProcessProto,
                codec_proto::MAX_CODEC_PROTO_JSON_BYTES,
            ),
        ];

        for (process, limit) in cases {
            let oversized = vec![0_u8; limit + 1];
            let mut produced_len = 0_usize;
            // SAFETY: The input pointer covers the entire caller-owned buffer,
            // and the null output is valid for a zero-length first-pass query.
            let status = unsafe {
                process(
                    oversized.as_ptr(),
                    oversized.len(),
                    core::ptr::null_mut(),
                    0,
                    &mut produced_len,
                )
            };

            assert_eq!(status, CODEC_BUFFER_TOO_SMALL);
            assert!(produced_len > 0);

            let mut output = vec![0_u8; produced_len];
            // SAFETY: The input, output, and produced-length storage are
            // distinct caller-owned allocations valid for this call.
            let status = unsafe {
                process(
                    oversized.as_ptr(),
                    oversized.len(),
                    output.as_mut_ptr(),
                    output.len(),
                    &mut produced_len,
                )
            };
            assert_eq!(status, CODEC_OK);
            output.truncate(produced_len);

            let envelope = decode_proto_result_envelope(&output);
            assert!(envelope.is_ok());
            let Some(envelope) = envelope.ok() else {
                return;
            };
            assert_eq!(envelope.status(), CodecProtoStatus::CodecError);
            let error = decode_codec_error_payload(envelope.bytes());
            assert!(error.is_ok());
            let Some(error) = error.ok() else {
                return;
            };
            assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
            assert_eq!(
                error.reason(),
                CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED
            );
        }
    }

    #[test]
    fn protobuf_ffi_exports_reject_inputs_above_bounded_sentinel() {
        let oversized = vec![0_u8; codec_proto::MAX_CODEC_PROTO_JSON_BYTES + 2];

        for process in [rm_codec_process_proto, rm_codec_process_proto_json] {
            let mut produced_len = 0_usize;
            // SAFETY: The input pointer covers the caller-owned buffer and the
            // null output is valid for a zero-length first-pass query.
            let status = unsafe {
                process(
                    oversized.as_ptr(),
                    oversized.len(),
                    core::ptr::null_mut(),
                    0,
                    &mut produced_len,
                )
            };
            assert_eq!(status, CODEC_INVALID_ARGUMENT);
            assert_eq!(produced_len, 0);
        }
    }

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
    fn generic_ffi_rejects_oversized_ignored_arguments_before_dispatch() {
        let oversized = vec![0_u8; MAX_CODEC_FFI_INPUT_BYTES + 1];
        let input = b"abc";
        let mut produced_len = 0_usize;

        // SAFETY: Every pointer describes valid caller-owned storage. The
        // third argument is deliberately oversized and must be rejected even
        // though the selected operation would otherwise ignore it.
        let status = unsafe {
            rm_codec_process(
                CODEC_BASE64_ENCODE,
                input.as_ptr(),
                input.len(),
                core::ptr::null(),
                0,
                oversized.as_ptr(),
                oversized.len(),
                core::ptr::null_mut(),
                0,
                &mut produced_len,
            )
        };

        assert_eq!(status, CODEC_INVALID_ARGUMENT);
        assert_eq!(produced_len, 0);
    }

    #[test]
    fn canonicalization_boundaries_reject_ambiguous_object_keys() {
        let duplicate_json = br#"{"a":1,"a":2}"#;
        let mut produced_len = 0_usize;

        // SAFETY: The input and produced-length storage remain valid for the
        // duration of the call. No output byte buffer is supplied because the
        // operation must reject this ambiguous JSON text before encoding.
        let jcs_status = unsafe {
            rm_codec_process(
                CODEC_CANONICALIZE_JSON,
                duplicate_json.as_ptr(),
                duplicate_json.len(),
                core::ptr::null(),
                0,
                core::ptr::null(),
                0,
                core::ptr::null_mut(),
                0,
                &mut produced_len,
            )
        };
        assert_eq!(jcs_status, CODEC_INVALID_ARGUMENT);
        assert_eq!(produced_len, 0);

        let duplicate_dag_cbor = br#"{"type":"map","value":[{"key":"a","value":{"type":"int","value":1}},{"key":"a","value":{"type":"int","value":2}}]}"#;
        // SAFETY: The input and produced-length storage remain valid for the
        // duration of the call. The duplicate map key must fail before output.
        let dag_cbor_status = unsafe {
            rm_codec_process(
                CODEC_DAG_CBOR_ENCODE,
                duplicate_dag_cbor.as_ptr(),
                duplicate_dag_cbor.len(),
                core::ptr::null(),
                0,
                core::ptr::null(),
                0,
                core::ptr::null_mut(),
                0,
                &mut produced_len,
            )
        };
        assert_eq!(dag_cbor_status, CODEC_INVALID_ARGUMENT);
        assert_eq!(produced_len, 0);
    }

    #[test]
    fn proto_error_is_carried_only_in_the_binary_result_envelope() {
        let request = CodecOperationRequest {
            operation: Some(CodecOperation::MultikeyParse(Box::new(
                CodecMultikeyParseRequest {
                    multikey: "not-a-key".to_owned(),
                    __buffa_unknown_fields: Default::default(),
                },
            ))),
            __buffa_unknown_fields: Default::default(),
        };
        let input = encode_protobuf(&request);
        let mut produced_len = 0_usize;

        // SAFETY: The input vector and produced-length output are valid for
        // the duration of this call. No output byte buffer is supplied.
        let probe_status = unsafe {
            rm_codec_process_proto(
                input.as_ptr(),
                input.len(),
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
                input.as_ptr(),
                input.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut produced_len,
            )
        };

        assert_eq!(status, CODEC_OK);
        assert!(produced_len > 0);
        let decoded = decode_proto_result_envelope(&output);
        assert!(decoded.is_ok());
        let Some(result) = decoded.ok() else {
            return;
        };
        assert_eq!(result.status(), CodecProtoStatus::CodecError);
    }

    #[test]
    fn proto_json_returns_the_same_binary_result_envelope() {
        let input = br#"{"multikeyParse":{"multikey":"not-a-key"}}"#;
        let mut produced_len = 0_usize;

        // SAFETY: The input and produced-length storage remain valid for the
        // duration of the sizing call.
        let probe_status = unsafe {
            rm_codec_process_proto_json(
                input.as_ptr(),
                input.len(),
                core::ptr::null_mut(),
                0,
                &mut produced_len,
            )
        };
        assert_eq!(probe_status, CODEC_BUFFER_TOO_SMALL);
        assert!(produced_len > 0);

        let mut output = vec![0_u8; produced_len];
        // SAFETY: All pointers describe valid, non-overlapping caller-owned
        // storage for the duration of the call.
        let status = unsafe {
            rm_codec_process_proto_json(
                input.as_ptr(),
                input.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut produced_len,
            )
        };

        assert_eq!(status, CODEC_OK);
        let decoded = decode_proto_result_envelope(&output);
        assert!(decoded.is_ok());
        let Some(result) = decoded.ok() else {
            return;
        };
        assert_eq!(result.status(), CodecProtoStatus::CodecError);
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

    #[test]
    fn failure_paths_initialize_scalar_out_parameters() {
        let mut produced_len = usize::MAX;
        // SAFETY: `produced_len` is valid writable storage. The deliberately
        // invalid operation has no input or byte output ranges to validate.
        let status = unsafe {
            rm_codec_process(
                u32::MAX,
                core::ptr::null(),
                0,
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

        let mut result = i32::MAX;
        // SAFETY: `result` is valid writable storage and both empty input
        // ranges satisfy the predicate boundary contract.
        let status = unsafe {
            rm_codec_process_bool(
                u32::MAX,
                core::ptr::null(),
                0,
                core::ptr::null(),
                0,
                &mut result,
            )
        };
        assert_eq!(status, CODEC_INVALID_ARGUMENT);
        assert_eq!(result, 0);
    }
}
