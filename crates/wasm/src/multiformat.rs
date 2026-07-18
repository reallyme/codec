// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_core::multicodec::{
    prefix_for_name as semantic_multicodec_prefix_for_name,
    strip_prefix as semantic_multicodec_strip_prefix, MulticodecOperationError,
};
use codec_core::scalar_ops::{
    binding_matches_codec, decode_base58btc, decode_multibase, encode_base58btc,
    encode_multibase_base58btc, encode_multibase_base64url, encode_multikey, parse_multikey_value,
    validate_binding, MultikeyError,
};
use js_sys::{JsString, Uint8Array};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use zeroize::Zeroizing;

use crate::boundary::{
    validate_input_lengths, validate_js_inputs, zeroizing_bytes, zeroizing_string,
};
use crate::map_error::{invalid_input, provider_failure, unsupported_codec};

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

fn map_multicodec_boundary_error(error: MulticodecOperationError) -> JsValue {
    match error {
        MulticodecOperationError::UnknownName => unsupported_codec(),
        MulticodecOperationError::InvalidPrefix => unsupported_codec(),
        MulticodecOperationError::RegistryInvariant
        | MulticodecOperationError::AllocationFailure => provider_failure(),
        _ => provider_failure(),
    }
}

#[wasm_bindgen(js_name = base58btcEncode)]
/// Encode bytes using the base58btc alphabet without a multibase prefix.
pub fn base58btc_encode_wasm(bytes: &Uint8Array) -> Result<String, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    encode_base58btc(input.as_slice()).map_err(|_| invalid_input())
}

#[wasm_bindgen(js_name = base58btcDecode)]
/// Decode bytes using the base58btc alphabet without a multibase prefix.
pub fn base58btc_decode_wasm(encoded: &JsString) -> Result<Uint8Array, JsValue> {
    let encoded = zeroizing_string(encoded)?;
    let decoded = Zeroizing::new(decode_base58btc(&encoded).map_err(|_| invalid_input())?);
    Ok(Uint8Array::from(decoded.as_slice()))
}

#[wasm_bindgen(js_name = multibaseBase64urlEncode)]
/// Encode bytes with the multibase base64url prefix.
pub fn multibase_base64url_encode(bytes: &Uint8Array) -> Result<String, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    encode_multibase_base64url(input.as_slice()).map_err(|_| invalid_input())
}

#[wasm_bindgen(js_name = multibaseBase58btcEncode)]
/// Encode bytes with the multibase base58btc prefix.
pub fn multibase_base58btc_encode(bytes: &Uint8Array) -> Result<String, JsValue> {
    let input = zeroizing_bytes(bytes)?;
    encode_multibase_base58btc(input.as_slice()).map_err(|_| invalid_input())
}

#[wasm_bindgen(js_name = multibaseDecode)]
/// Decode a supported multibase string.
pub fn multibase_decode(encoded: &JsString) -> Result<Uint8Array, JsValue> {
    let encoded = zeroizing_string(encoded)?;
    let decoded = Zeroizing::new(decode_multibase(&encoded).map_err(|_| invalid_input())?);
    Ok(Uint8Array::from(decoded.as_slice()))
}

#[wasm_bindgen(js_name = multicodecStripPrefix)]
/// Strip a known multicodec prefix, or return the original bytes when none is found.
pub fn multicodec_strip_prefix(bytes: &Uint8Array) -> Result<Uint8Array, JsValue> {
    let bytes = zeroizing_bytes(bytes)?;
    let stripped = semantic_multicodec_strip_prefix(bytes.as_slice())
        .map_err(map_multicodec_boundary_error)?;
    Ok(Uint8Array::from(stripped))
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
    Ok(binding_matches_codec(&binding_type, &codec_name))
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
    let parsed = parse_multikey_value(&multikey).map_err(|_| invalid_input())?;
    validate_binding(
        &binding_type,
        algorithm.as_ref().map(|value| value.as_str()),
        &parsed,
    )
    .map_err(|_| invalid_input())?;
    Ok(())
}

#[wasm_bindgen(js_name = requireSupportedMulticodec)]
/// Fail closed when a codec name is not in the supported table.
pub fn require_supported_multicodec(codec_name: &JsString) -> Result<(), JsValue> {
    let codec_name = zeroizing_string(codec_name)?;
    semantic_multicodec_prefix_for_name(&codec_name)
        .map(|_| ())
        .map_err(map_multicodec_boundary_error)
}
