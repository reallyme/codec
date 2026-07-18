// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_core::scalar_ops::{
    compute_dag_cbor_cid, dag_cbor_codec_code as scalar_dag_cbor_codec_code, dag_cbor_content_hash,
    dag_cbor_multihash_value, parse_cid, valid_cid,
};
use js_sys::{JsString, Uint8Array};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use crate::boundary::{zeroizing_bytes, zeroizing_string};
use crate::map_error::invalid_input;

#[wasm_bindgen(js_name = dagCborComputeCid)]
/// Compute a CIDv1 dag-cbor/sha2-256 string for already-canonical DAG-CBOR bytes.
pub fn dag_cbor_compute_cid(bytes: &Uint8Array) -> Result<String, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    compute_dag_cbor_cid(input.as_slice()).map_err(|_| invalid_input())
}

#[wasm_bindgen(js_name = dagCborSha256ContentHash)]
/// Return the raw sha2-256 content hash for canonical DAG-CBOR bytes.
pub fn dag_cbor_sha256_content_hash(bytes: &Uint8Array) -> Result<Uint8Array, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    let digest = dag_cbor_content_hash(input.as_slice()).map_err(|_| invalid_input())?;
    Ok(Uint8Array::from(digest.as_slice()))
}

#[wasm_bindgen(js_name = dagCborMultihash)]
/// Return the sha2-256 multihash envelope for canonical DAG-CBOR bytes.
pub fn dag_cbor_multihash_wasm(bytes: &Uint8Array) -> Result<Uint8Array, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    let multihash = dag_cbor_multihash_value(input.as_slice()).map_err(|_| invalid_input())?;
    Ok(Uint8Array::from(multihash.to_bytes().as_slice()))
}

#[wasm_bindgen(js_name = isValidCidString)]
/// Return whether a string parses as a CID.
pub fn is_valid_cid_string_wasm(cid: &JsString) -> Result<bool, JsValue> {
    let cid = zeroizing_string(cid)?;
    Ok(valid_cid(&cid))
}

#[wasm_bindgen(js_name = tryParseCid)]
/// Return the canonical CID string, or `undefined` when parsing fails.
pub fn try_parse_cid_wasm(cid: &JsString) -> Result<JsValue, JsValue> {
    let cid = zeroizing_string(cid)?;
    Ok(match parse_cid(&cid) {
        Some(parsed) => JsValue::from_str(&parsed),
        None => JsValue::UNDEFINED,
    })
}

#[wasm_bindgen(js_name = dagCborCodecCode)]
/// Return the DAG-CBOR multicodec code.
pub fn dag_cbor_codec_code() -> u32 {
    scalar_dag_cbor_codec_code()
}
