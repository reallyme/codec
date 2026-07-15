// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_multibase::{
    base58btc_decode, base58btc_encode, bytes_to_multibase58btc, bytes_to_multibase_base64url,
    multibase_to_bytes,
};
use codec_multicodec::{
    lookup_codec_prefix, strip_codec_prefix, CodecSpec, MULTICODEC_TABLE, VARIABLE_KEY_LENGTH,
};
use codec_multikey::{
    binding_type_matches_codec, encode_multikey, parse_multikey, validate_key_binding,
    KeyBindingInput, MultikeyError,
};
use js_sys::{Array, Object, Uint8Array};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use zeroize::Zeroizing;

use crate::map_error::{invalid_input, unsupported_codec};
use crate::write_js_object::{
    codec_lookup_to_js, codec_spec_to_js, set_bytes, set_string, set_u32,
};

fn find_codec_spec(codec_name: &str) -> Option<(&'static str, &'static CodecSpec)> {
    MULTICODEC_TABLE
        .iter()
        .find(|(name, _)| *name == codec_name)
        .map(|(name, spec)| (*name, spec))
}

fn map_multikey_boundary_error(error: MultikeyError) -> JsValue {
    match error {
        MultikeyError::UnknownCodecName { .. } | MultikeyError::UnknownCodecPrefix => {
            unsupported_codec()
        }
        MultikeyError::InvalidMultibase
        | MultikeyError::DecodedTooShort(_)
        | MultikeyError::KeyLengthMismatch { .. }
        | MultikeyError::KeyTooLarge { .. }
        | MultikeyError::EncodedPayloadTooLarge
        | MultikeyError::BindingTypeCodecMismatch { .. }
        | MultikeyError::BindingAlgorithmMismatch { .. }
        | MultikeyError::BindingAlgorithmMissing { .. } => invalid_input(),
    }
}

#[wasm_bindgen(js_name = base58btcEncode)]
/// Encode bytes using the base58btc alphabet without a multibase prefix.
pub fn base58btc_encode_wasm(bytes: &Uint8Array) -> Result<String, JsValue> {
    let input = Zeroizing::new(bytes.to_vec());
    base58btc_encode(input.as_slice()).map_err(|_| invalid_input())
}

#[wasm_bindgen(js_name = base58btcDecode)]
/// Decode bytes using the base58btc alphabet without a multibase prefix.
pub fn base58btc_decode_wasm(encoded: &str) -> Result<Uint8Array, JsValue> {
    let decoded = base58btc_decode(encoded).map_err(|_| invalid_input())?;
    Ok(Uint8Array::from(decoded.as_slice()))
}

#[wasm_bindgen(js_name = multibaseBase64urlEncode)]
/// Encode bytes with the multibase base64url prefix.
pub fn multibase_base64url_encode(bytes: &Uint8Array) -> String {
    let input = Zeroizing::new(bytes.to_vec());
    bytes_to_multibase_base64url(input.as_slice())
}

#[wasm_bindgen(js_name = multibaseBase58btcEncode)]
/// Encode bytes with the multibase base58btc prefix.
pub fn multibase_base58btc_encode(bytes: &Uint8Array) -> Result<String, JsValue> {
    let input = Zeroizing::new(bytes.to_vec());
    bytes_to_multibase58btc(input.as_slice()).map_err(|_| invalid_input())
}

#[wasm_bindgen(js_name = multibaseDecode)]
/// Decode a supported multibase string.
pub fn multibase_decode(encoded: &str) -> Result<Uint8Array, JsValue> {
    let decoded = multibase_to_bytes(encoded).map_err(|_| invalid_input())?;
    Ok(Uint8Array::from(decoded.as_slice()))
}

#[wasm_bindgen(js_name = multicodecPrefixForName)]
/// Return multicodec metadata for a canonical codec name.
pub fn multicodec_prefix_for_name(codec_name: &str) -> Result<JsValue, JsValue> {
    let (name, spec) = find_codec_spec(codec_name).ok_or_else(unsupported_codec)?;
    codec_spec_to_js(name, spec)
}

#[wasm_bindgen(js_name = multicodecLookupPrefix)]
/// Resolve a byte slice that starts with a known multicodec prefix.
pub fn multicodec_lookup_prefix(bytes: &Uint8Array) -> Result<JsValue, JsValue> {
    let bytes = Zeroizing::new(bytes.to_vec());
    let found = lookup_codec_prefix(bytes.as_slice()).ok_or_else(unsupported_codec)?;
    codec_lookup_to_js(found)
}

#[wasm_bindgen(js_name = multicodecStripPrefix)]
/// Strip a known multicodec prefix, or return the original bytes when none is found.
pub fn multicodec_strip_prefix(bytes: &Uint8Array) -> Uint8Array {
    let bytes = Zeroizing::new(bytes.to_vec());
    Uint8Array::from(strip_codec_prefix(bytes.as_slice()))
}

#[wasm_bindgen(js_name = multicodecTable)]
/// Return the supported multicodec table.
pub fn multicodec_table() -> Result<JsValue, JsValue> {
    let array = Array::new();
    for (name, spec) in MULTICODEC_TABLE {
        array.push(&codec_spec_to_js(name, spec)?);
    }
    Ok(array.into())
}

#[wasm_bindgen(js_name = multikeyEncode)]
/// Encode a public key as a multibase base58btc multikey.
pub fn multikey_encode(codec_name: &str, public_key: &Uint8Array) -> Result<String, JsValue> {
    let public_key = Zeroizing::new(public_key.to_vec());
    encode_multikey(codec_name, public_key.as_slice()).map_err(map_multikey_boundary_error)
}

#[wasm_bindgen(js_name = multikeyParse)]
/// Parse and validate a multikey string.
pub fn multikey_parse(multikey: &str) -> Result<JsValue, JsValue> {
    let parsed = parse_multikey(multikey).map_err(map_multikey_boundary_error)?;
    let object = Object::new();
    set_string(&object, "codecName", parsed.codec_name)?;
    set_string(&object, "algorithmName", parsed.alg)?;
    set_bytes(&object, "publicKey", &parsed.public_key)?;
    if parsed.key_length != VARIABLE_KEY_LENGTH {
        set_u32(&object, "expectedPublicKeyLength", parsed.key_length)?;
    }
    Ok(object.into())
}

#[wasm_bindgen(js_name = bindingTypeMatchesCodec)]
/// Return whether a multikey binding type is compatible with a codec name.
pub fn binding_type_matches_codec_wasm(binding_type: &str, codec_name: &str) -> bool {
    binding_type_matches_codec(binding_type, codec_name)
}

#[wasm_bindgen(js_name = validateKeyBinding)]
/// Validate a binding type and optional algorithm against a parsed multikey.
pub fn validate_key_binding_wasm(
    binding_type: &str,
    algorithm: Option<String>,
    multikey: &str,
) -> Result<(), JsValue> {
    let parsed = parse_multikey(multikey).map_err(|_| invalid_input())?;
    let binding = KeyBindingInput {
        binding_type,
        algorithm: algorithm.as_deref(),
    };
    validate_key_binding(binding, &parsed).map_err(|_| invalid_input())?;
    Ok(())
}

#[wasm_bindgen(js_name = requireSupportedMulticodec)]
/// Fail closed when a codec name is not in the supported table.
pub fn require_supported_multicodec(codec_name: &str) -> Result<(), JsValue> {
    find_codec_spec(codec_name)
        .map(|_| ())
        .ok_or_else(unsupported_codec)
}
