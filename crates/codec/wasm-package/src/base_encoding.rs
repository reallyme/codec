// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_base64::{base64_to_bytes, bytes_to_base64};
use codec_base64url::{base64url_to_bytes, bytes_to_base64url};
use codec_hex::{bytes_to_lower_hex, lower_hex_to_bytes, HexError};
use js_sys::Uint8Array;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use zeroize::Zeroizing;

use crate::map_error::{invalid_input, non_canonical};

#[wasm_bindgen(js_name = base64Encode)]
/// Encode bytes using canonical padded RFC 4648 base64.
pub fn base64_encode(bytes: &Uint8Array) -> String {
    let input = Zeroizing::new(bytes.to_vec());
    bytes_to_base64(input.as_slice())
}

#[wasm_bindgen(js_name = base64Decode)]
/// Decode canonical padded RFC 4648 base64.
pub fn base64_decode(encoded: &str) -> Result<Uint8Array, JsValue> {
    let decoded = base64_to_bytes(encoded).map_err(|_| invalid_input())?;
    Ok(Uint8Array::from(decoded.as_slice()))
}

#[wasm_bindgen(js_name = base64urlEncode)]
/// Encode bytes using unpadded RFC 4648 URL-safe base64.
pub fn base64url_encode(bytes: &Uint8Array) -> String {
    let input = Zeroizing::new(bytes.to_vec());
    bytes_to_base64url(input.as_slice())
}

#[wasm_bindgen(js_name = base64urlDecode)]
/// Decode strict unpadded RFC 4648 URL-safe base64.
pub fn base64url_decode(encoded: &str) -> Result<Uint8Array, JsValue> {
    let decoded = base64url_to_bytes(encoded).map_err(|_| invalid_input())?;
    Ok(Uint8Array::from(decoded.as_slice()))
}

#[wasm_bindgen(js_name = bytesToLowerHex)]
/// Encode bytes as canonical lowercase hexadecimal.
pub fn bytes_to_lower_hex_wasm(bytes: &Uint8Array) -> String {
    let input = Zeroizing::new(bytes.to_vec());
    bytes_to_lower_hex(input.as_slice())
}

#[wasm_bindgen(js_name = lowerHexToBytes)]
/// Decode canonical lowercase hexadecimal.
pub fn lower_hex_to_bytes_wasm(encoded: &str) -> Result<Uint8Array, JsValue> {
    let decoded = lower_hex_to_bytes(encoded).map_err(|error| match error {
        HexError::Uppercase => non_canonical(),
        HexError::OddLength | HexError::InvalidCharacter => invalid_input(),
    })?;
    Ok(Uint8Array::from(decoded.as_slice()))
}
