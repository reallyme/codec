// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_base64url::{base64url_to_bytes, bytes_to_base64url};
use codec_cbor::{
    compute_cid_dag_cbor, dag_cbor_multihash, decode_dag_cbor, encode_dag_cbor,
    is_valid_cid_string, sha2_256_content_hash, verify_dag_cbor_cid, CborValue,
    MAX_DAG_CBOR_INPUT_LEN,
};
use js_sys::{JsString, Object, Uint8Array};
use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use crate::boundary::{
    validate_input_lengths, validate_js_inputs, zeroizing_bytes, zeroizing_string,
};
use crate::map_error::{invalid_input, non_canonical, provider_failure};
use crate::write_js_object::{set_bool, set_string};

const JS_SAFE_INTEGER_MAX: i64 = 9_007_199_254_740_991;
const JS_SAFE_INTEGER_MIN: i64 = -JS_SAFE_INTEGER_MAX;

#[derive(Clone, Copy)]
struct TaggedInt(i64);

impl Serialize for TaggedInt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if (JS_SAFE_INTEGER_MIN..=JS_SAFE_INTEGER_MAX).contains(&self.0) {
            serializer.serialize_i64(self.0)
        } else {
            serializer.serialize_str(&self.0.to_string())
        }
    }
}

impl<'de> Deserialize<'de> for TaggedInt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(TaggedIntVisitor)
    }
}

struct TaggedIntVisitor;

impl<'de> Visitor<'de> for TaggedIntVisitor {
    type Value = TaggedInt;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a signed 64-bit DAG-CBOR integer")
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        Ok(TaggedInt(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        i64::try_from(value)
            .map(TaggedInt)
            .map_err(|_| E::custom("integer outside signed 64-bit range"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        parse_decimal_i64(value)
            .map(TaggedInt)
            .ok_or_else(|| E::custom("invalid signed 64-bit integer"))
    }
}

fn parse_decimal_i64(value: &str) -> Option<i64> {
    let digits = value.strip_prefix('-').unwrap_or(value);
    if digits.is_empty() {
        return None;
    }
    if digits.len() > 1 && digits.starts_with('0') {
        return None;
    }
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse::<i64>().ok()
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
enum TaggedCborValue {
    Null,
    Bool(bool),
    Int(TaggedInt),
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

fn tagged_to_cbor(value: TaggedCborValue) -> Result<CborValue, JsValue> {
    match value {
        TaggedCborValue::Null => Ok(CborValue::Null),
        TaggedCborValue::Bool(value) => Ok(CborValue::Bool(value)),
        TaggedCborValue::Int(value) => Ok(CborValue::Int(value.0)),
        TaggedCborValue::String(value) => Ok(CborValue::String(value)),
        TaggedCborValue::Bytes(value) => {
            let bytes = base64url_to_bytes(&value).map_err(|_| invalid_input())?;
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

fn cbor_to_tagged(value: CborValue) -> Result<TaggedCborValue, JsValue> {
    match value {
        CborValue::Null => Ok(TaggedCborValue::Null),
        CborValue::Bool(value) => Ok(TaggedCborValue::Bool(value)),
        CborValue::Int(value) => Ok(TaggedCborValue::Int(TaggedInt(value))),
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
        _ => Err(provider_failure()),
    }
}

#[wasm_bindgen(js_name = dagCborEncode)]
/// Encode a tagged JSON representation as canonical DAG-CBOR bytes.
pub fn dag_cbor_encode(value_json: &JsString) -> Result<Uint8Array, JsValue> {
    validate_js_inputs(&[value_json], &[])?;
    let value_json = zeroizing_string(value_json)?;
    validate_input_lengths(&[value_json.len()])?;
    let tagged: TaggedCborValue = serde_json::from_str(&value_json).map_err(|_| invalid_input())?;
    let encoded = encode_dag_cbor(&tagged_to_cbor(tagged)?).map_err(|_| invalid_input())?;
    Ok(Uint8Array::from(encoded.as_slice()))
}

#[wasm_bindgen(js_name = dagCborDecode)]
/// Decode canonical DAG-CBOR bytes into the tagged JSON representation.
pub fn dag_cbor_decode(bytes: &Uint8Array) -> Result<String, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    let value = decode_dag_cbor(input.as_slice()).map_err(|_| non_canonical())?;
    let tagged = cbor_to_tagged(value)?;
    serde_json::to_string(&tagged).map_err(|_| provider_failure())
}

#[wasm_bindgen(js_name = dagCborComputeCid)]
/// Compute a CIDv1 dag-cbor/sha2-256 string for already-canonical DAG-CBOR bytes.
pub fn dag_cbor_compute_cid(bytes: &Uint8Array) -> Result<String, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    if input.len() > MAX_DAG_CBOR_INPUT_LEN {
        return Err(invalid_input());
    }
    Ok(compute_cid_dag_cbor(input.as_slice()))
}

#[wasm_bindgen(js_name = dagCborVerifyCid)]
/// Recompute and compare a CIDv1 dag-cbor/sha2-256 string.
pub fn dag_cbor_verify_cid(cid: &JsString, bytes: &Uint8Array) -> Result<JsValue, JsValue> {
    validate_js_inputs(&[cid], &[bytes])?;
    let cid = zeroizing_string(cid)?;
    let input = zeroizing_bytes(bytes)?;
    validate_input_lengths(&[cid.len(), input.len()])?;
    if input.len() > MAX_DAG_CBOR_INPUT_LEN {
        return Err(invalid_input());
    }
    let (valid, expected_cid, actual_cid) = verify_dag_cbor_cid(&cid, input.as_slice());
    let object = Object::new();
    set_bool(&object, "valid", valid)?;
    set_string(&object, "expectedCid", &expected_cid)?;
    set_string(&object, "actualCid", &actual_cid)?;
    Ok(object.into())
}

#[wasm_bindgen(js_name = dagCborSha256ContentHash)]
/// Return the raw sha2-256 content hash for canonical DAG-CBOR bytes.
pub fn dag_cbor_sha256_content_hash(bytes: &Uint8Array) -> Result<Uint8Array, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    if input.len() > MAX_DAG_CBOR_INPUT_LEN {
        return Err(invalid_input());
    }
    let digest = sha2_256_content_hash(input.as_slice());
    Ok(Uint8Array::from(digest.as_slice()))
}

#[wasm_bindgen(js_name = dagCborMultihash)]
/// Return the sha2-256 multihash envelope for canonical DAG-CBOR bytes.
pub fn dag_cbor_multihash_wasm(bytes: &Uint8Array) -> Result<Uint8Array, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    if input.len() > MAX_DAG_CBOR_INPUT_LEN {
        return Err(invalid_input());
    }
    let multihash = dag_cbor_multihash(input.as_slice());
    Ok(Uint8Array::from(multihash.to_bytes().as_slice()))
}

#[wasm_bindgen(js_name = isValidCidString)]
/// Return whether a string parses as a CID.
pub fn is_valid_cid_string_wasm(cid: &JsString) -> Result<bool, JsValue> {
    validate_js_inputs(&[cid], &[])?;
    let cid = zeroizing_string(cid)?;
    validate_input_lengths(&[cid.len()])?;
    Ok(is_valid_cid_string(&cid))
}

#[wasm_bindgen(js_name = tryParseCid)]
/// Return the canonical CID string, or `undefined` when parsing fails.
pub fn try_parse_cid_wasm(cid: &JsString) -> Result<JsValue, JsValue> {
    validate_js_inputs(&[cid], &[])?;
    let cid = zeroizing_string(cid)?;
    validate_input_lengths(&[cid.len()])?;
    Ok(match codec_cbor::try_parse_cid(&cid) {
        Some(parsed) => JsValue::from_str(&parsed.to_string()),
        None => JsValue::UNDEFINED,
    })
}

#[wasm_bindgen(js_name = dagCborCodecCode)]
/// Return the DAG-CBOR multicodec code.
pub fn dag_cbor_codec_code() -> u32 {
    0x71
}
