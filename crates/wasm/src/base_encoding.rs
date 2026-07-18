// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_core::scalar_ops::{
    decode_base64, decode_base64url, decode_lower_hex, encode_base64, encode_base64url,
    encode_lower_hex, HexError,
};
use js_sys::{JsString, Uint8Array};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use zeroize::Zeroizing;

use crate::boundary::{zeroizing_bytes, zeroizing_string};
use crate::map_error::{invalid_input, non_canonical, provider_failure};

#[wasm_bindgen(js_name = base64Encode)]
/// Encode bytes using canonical padded RFC 4648 base64.
pub fn base64_encode(bytes: &Uint8Array) -> Result<String, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    Ok(encode_base64(input.as_slice()))
}

#[wasm_bindgen(js_name = base64Decode)]
/// Decode canonical padded RFC 4648 base64.
pub fn base64_decode(encoded: &JsString) -> Result<Uint8Array, JsValue> {
    let encoded = zeroizing_string(encoded)?;
    let decoded = Zeroizing::new(decode_base64(&encoded).map_err(|_| invalid_input())?);
    Ok(Uint8Array::from(decoded.as_slice()))
}

#[wasm_bindgen(js_name = base64urlEncode)]
/// Encode bytes using unpadded RFC 4648 URL-safe base64.
pub fn base64url_encode(bytes: &Uint8Array) -> Result<String, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    Ok(encode_base64url(input.as_slice()))
}

#[wasm_bindgen(js_name = base64urlDecode)]
/// Decode strict unpadded RFC 4648 URL-safe base64.
pub fn base64url_decode(encoded: &JsString) -> Result<Uint8Array, JsValue> {
    let encoded = zeroizing_string(encoded)?;
    let decoded = Zeroizing::new(decode_base64url(&encoded).map_err(|_| invalid_input())?);
    Ok(Uint8Array::from(decoded.as_slice()))
}

#[wasm_bindgen(js_name = bytesToLowerHex)]
/// Encode bytes as canonical lowercase hexadecimal.
pub fn bytes_to_lower_hex_wasm(bytes: &Uint8Array) -> Result<String, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    Ok(encode_lower_hex(input.as_slice()))
}

#[wasm_bindgen(js_name = lowerHexToBytes)]
/// Decode canonical lowercase hexadecimal.
pub fn lower_hex_to_bytes_wasm(encoded: &JsString) -> Result<Uint8Array, JsValue> {
    let encoded = zeroizing_string(encoded)?;
    let decoded = Zeroizing::new(decode_lower_hex(&encoded).map_err(|error| match error {
        HexError::Uppercase => non_canonical(),
        HexError::OddLength | HexError::InvalidCharacter => invalid_input(),
        _ => provider_failure(),
    })?);
    Ok(Uint8Array::from(decoded.as_slice()))
}
