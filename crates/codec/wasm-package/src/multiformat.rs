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
use js_sys::{Array, JsString, Object, Uint8Array};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use zeroize::Zeroizing;

use crate::boundary::{
    validate_input_lengths, validate_js_inputs, zeroizing_bytes, zeroizing_string,
};
use crate::map_error::{invalid_input, provider_failure, unsupported_codec};
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
        _ => provider_failure(),
    }
}

#[wasm_bindgen(js_name = base58btcEncode)]
/// Encode bytes using the base58btc alphabet without a multibase prefix.
pub fn base58btc_encode_wasm(bytes: &Uint8Array) -> Result<String, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    base58btc_encode(input.as_slice()).map_err(|_| invalid_input())
}

#[wasm_bindgen(js_name = base58btcDecode)]
/// Decode bytes using the base58btc alphabet without a multibase prefix.
pub fn base58btc_decode_wasm(encoded: &JsString) -> Result<Uint8Array, JsValue> {
    validate_js_inputs(&[encoded], &[])?;
    let encoded = zeroizing_string(encoded)?;
    validate_input_lengths(&[encoded.len()])?;
    let decoded = Zeroizing::new(base58btc_decode(&encoded).map_err(|_| invalid_input())?);
    Ok(Uint8Array::from(decoded.as_slice()))
}

#[wasm_bindgen(js_name = multibaseBase64urlEncode)]
/// Encode bytes with the multibase base64url prefix.
pub fn multibase_base64url_encode(bytes: &Uint8Array) -> Result<String, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    Ok(bytes_to_multibase_base64url(input.as_slice()))
}

#[wasm_bindgen(js_name = multibaseBase58btcEncode)]
/// Encode bytes with the multibase base58btc prefix.
pub fn multibase_base58btc_encode(bytes: &Uint8Array) -> Result<String, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    bytes_to_multibase58btc(input.as_slice()).map_err(|_| invalid_input())
}

#[wasm_bindgen(js_name = multibaseDecode)]
/// Decode a supported multibase string.
pub fn multibase_decode(encoded: &JsString) -> Result<Uint8Array, JsValue> {
    validate_js_inputs(&[encoded], &[])?;
    let encoded = zeroizing_string(encoded)?;
    validate_input_lengths(&[encoded.len()])?;
    let decoded = Zeroizing::new(multibase_to_bytes(&encoded).map_err(|_| invalid_input())?);
    Ok(Uint8Array::from(decoded.as_slice()))
}

#[wasm_bindgen(js_name = multicodecPrefixForName)]
/// Return multicodec metadata for a canonical codec name.
pub fn multicodec_prefix_for_name(codec_name: &JsString) -> Result<JsValue, JsValue> {
    validate_js_inputs(&[codec_name], &[])?;
    let codec_name = zeroizing_string(codec_name)?;
    validate_input_lengths(&[codec_name.len()])?;
    let (name, spec) = find_codec_spec(&codec_name).ok_or_else(unsupported_codec)?;
    codec_spec_to_js(name, spec)
}

#[wasm_bindgen(js_name = multicodecLookupPrefix)]
/// Resolve a byte slice that starts with a known multicodec prefix.
pub fn multicodec_lookup_prefix(bytes: &Uint8Array) -> Result<JsValue, JsValue> {
    let bytes = zeroizing_bytes(bytes)?;
    let found = lookup_codec_prefix(bytes.as_slice()).ok_or_else(unsupported_codec)?;
    codec_lookup_to_js(found)
}

#[wasm_bindgen(js_name = multicodecStripPrefix)]
/// Strip a known multicodec prefix, or return the original bytes when none is found.
pub fn multicodec_strip_prefix(bytes: &Uint8Array) -> Result<Uint8Array, JsValue> {
    let bytes = zeroizing_bytes(bytes)?;
    Ok(Uint8Array::from(strip_codec_prefix(bytes.as_slice())))
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
pub fn multikey_encode(codec_name: &JsString, public_key: &Uint8Array) -> Result<String, JsValue> {
    validate_js_inputs(&[codec_name], &[public_key])?;
    let codec_name = zeroizing_string(codec_name)?;
    let public_key = zeroizing_bytes(public_key)?;
    validate_input_lengths(&[codec_name.len(), public_key.len()])?;
    encode_multikey(&codec_name, public_key.as_slice()).map_err(map_multikey_boundary_error)
}

#[wasm_bindgen(js_name = multikeyParse)]
/// Parse and validate a multikey string.
pub fn multikey_parse(multikey: &JsString) -> Result<JsValue, JsValue> {
    validate_js_inputs(&[multikey], &[])?;
    let multikey = zeroizing_string(multikey)?;
    validate_input_lengths(&[multikey.len()])?;
    let parsed = parse_multikey(&multikey).map_err(map_multikey_boundary_error)?;
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
pub fn binding_type_matches_codec_wasm(
    binding_type: &JsString,
    codec_name: &JsString,
) -> Result<bool, JsValue> {
    validate_js_inputs(&[binding_type, codec_name], &[])?;
    let binding_type = zeroizing_string(binding_type)?;
    let codec_name = zeroizing_string(codec_name)?;
    validate_input_lengths(&[binding_type.len(), codec_name.len()])?;
    Ok(binding_type_matches_codec(&binding_type, &codec_name))
}

#[wasm_bindgen(js_name = validateKeyBinding)]
/// Validate a binding type and optional algorithm against a parsed multikey.
pub fn validate_key_binding_wasm(
    binding_type: &JsString,
    algorithm: Option<JsString>,
    multikey: &JsString,
) -> Result<(), JsValue> {
    match algorithm.as_ref() {
        Some(algorithm) => validate_js_inputs(&[binding_type, algorithm, multikey], &[])?,
        None => validate_js_inputs(&[binding_type, multikey], &[])?,
    }
    let binding_type = zeroizing_string(binding_type)?;
    let algorithm = algorithm.as_ref().map(zeroizing_string).transpose()?;
    let multikey = zeroizing_string(multikey)?;
    let algorithm_len = algorithm.as_ref().map_or(0, |value| value.len());
    validate_input_lengths(&[binding_type.len(), algorithm_len, multikey.len()])?;
    let parsed = parse_multikey(&multikey).map_err(|_| invalid_input())?;
    let binding = KeyBindingInput {
        binding_type: &binding_type,
        algorithm: algorithm.as_ref().map(|value| value.as_str()),
    };
    validate_key_binding(binding, &parsed).map_err(|_| invalid_input())?;
    Ok(())
}

#[wasm_bindgen(js_name = requireSupportedMulticodec)]
/// Fail closed when a codec name is not in the supported table.
pub fn require_supported_multicodec(codec_name: &JsString) -> Result<(), JsValue> {
    validate_js_inputs(&[codec_name], &[])?;
    let codec_name = zeroizing_string(codec_name)?;
    validate_input_lengths(&[codec_name.len()])?;
    find_codec_spec(&codec_name)
        .map(|_| ())
        .ok_or_else(unsupported_codec)
}
