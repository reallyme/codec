// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared execution for schema-defined scalar and raw-byte operations.
//!
//! The generated dispatcher and legacy native/WASM facades share these
//! implementations. Borrowed request views preserve caller ownership without
//! constructing another sensitive payload owner or serializing native calls.

use codec_proto::generated::proto::reallyme::codec::v1::{
    CodecBindingTypeMatchesCodecRequestView, CodecBytesRequestView, CodecMultikeyEncodeRequestView,
    CodecTextRequestView, CodecValidateKeyBindingRequestView,
};

use codec_base64::{base64_to_bytes, bytes_to_base64, Base64Error};
use codec_base64url::{base64url_to_bytes, bytes_to_base64url, Base64UrlError};
use codec_cbor::{
    compute_cid_dag_cbor, dag_cbor_multihash, is_valid_cid_string, sha2_256_content_hash,
    try_parse_cid, ContentHash, DagCborMultihash, MAX_DAG_CBOR_INPUT_LEN,
};
use codec_hex::{bytes_to_lower_hex, lower_hex_to_bytes};
use codec_jcs::canonicalize_json_text;
pub use codec_jcs::JcsError;
use codec_multibase::{
    base58btc_decode, base58btc_encode, bytes_to_multibase58btc, bytes_to_multibase_base64url,
    multibase_to_bytes, Base58Error, MultibaseError, MAX_BASE58BTC_INPUT_LEN,
};
#[allow(deprecated)]
use codec_multikey::{
    binding_type_matches_codec, encode_multikey as encode_multikey_primitive, parse_multikey,
    validate_key_binding, validate_multikey_binding, KeyBindingInput, ParsedMultikey,
};

pub use codec_cbor::{
    DAG_CBOR_CODEC as DAG_CBOR_CODEC_CODE, MAX_DAG_CBOR_INPUT_LEN as MAX_DAG_CBOR_INPUT_BYTES,
};
pub use codec_hex::HexError;
pub use codec_multibase::MAX_BASE58BTC_DECODED_LEN as MAX_BASE58BTC_INPUT_BYTES;
pub use codec_multikey::MultikeyError;

/// Error returned by scalar/raw-byte operation helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ScalarOperationError {
    /// The caller supplied more bytes than the operation admits.
    #[error("scalar operation input too large")]
    InputTooLarge,
}

/// Encode bytes as padded standard Base64.
pub fn encode_base64(input: &[u8]) -> String {
    let request = CodecBytesRequestView { value: input };
    let input = request.value;
    bytes_to_base64(input)
}

/// Decode canonical padded standard Base64.
pub fn decode_base64(input: &str) -> Result<Vec<u8>, Base64Error> {
    let request = CodecTextRequestView { value: input };
    let input = request.value;
    base64_to_bytes(input)
}

/// Encode bytes as unpadded URL-safe Base64.
pub fn encode_base64url(input: &[u8]) -> String {
    let request = CodecBytesRequestView { value: input };
    let input = request.value;
    bytes_to_base64url(input)
}

/// Decode canonical unpadded URL-safe Base64.
pub fn decode_base64url(input: &str) -> Result<Vec<u8>, Base64UrlError> {
    let request = CodecTextRequestView { value: input };
    let input = request.value;
    base64url_to_bytes(input)
}

/// Decode a borrowed ASCII Base64url request through the same primitive policy.
pub(super) fn decode_base64url_bytes(input: &[u8]) -> Result<Vec<u8>, Base64UrlError> {
    let request = CodecBytesRequestView { value: input };
    codec_base64url::base64url_bytes_to_bytes(request.value)
}

/// Encode bytes as lowercase hexadecimal.
pub fn encode_lower_hex(input: &[u8]) -> String {
    let request = CodecBytesRequestView { value: input };
    let input = request.value;
    bytes_to_lower_hex(input)
}

/// Decode canonical lowercase hexadecimal.
pub fn decode_lower_hex(input: &str) -> Result<Vec<u8>, HexError> {
    let request = CodecTextRequestView { value: input };
    let input = request.value;
    lower_hex_to_bytes(input)
}

/// Encode bytes as base58btc within the primitive budget.
pub fn encode_base58btc(input: &[u8]) -> Result<String, Base58Error> {
    let request = CodecBytesRequestView { value: input };
    let input = request.value;
    base58btc_encode(input)
}

/// Decode base58btc after enforcing the text budget.
pub fn decode_base58btc(input: &str) -> Result<Vec<u8>, Base58Error> {
    let request = CodecTextRequestView { value: input };
    let input = request.value;
    if input.len() > MAX_BASE58BTC_INPUT_LEN {
        return Err(Base58Error::InputTooLarge);
    }
    base58btc_decode(input)
}

/// Encode bytes with the base58btc multibase prefix.
pub fn encode_multibase_base58btc(input: &[u8]) -> Result<String, Base58Error> {
    let request = CodecBytesRequestView { value: input };
    let input = request.value;
    bytes_to_multibase58btc(input)
}

/// Encode bytes with the Base64url multibase prefix.
pub fn encode_multibase_base64url(input: &[u8]) -> Result<String, MultibaseError> {
    let request = CodecBytesRequestView { value: input };
    let input = request.value;
    bytes_to_multibase_base64url(input)
}

/// Decode a supported multibase string within the text budget.
pub fn decode_multibase(input: &str) -> Result<Vec<u8>, MultibaseError> {
    let request = CodecTextRequestView { value: input };
    let input = request.value;
    let max_base58btc_multibase_len = MAX_BASE58BTC_INPUT_LEN
        .checked_add(1)
        .ok_or(MultibaseError::LengthOverflow)?;
    if input.starts_with('z') && input.len() > max_base58btc_multibase_len {
        return Err(MultibaseError::Base58(Base58Error::InputTooLarge));
    }
    multibase_to_bytes(input)
}

/// Encode public key bytes as a named multikey.
pub fn encode_multikey(codec_name: &str, public_key: &[u8]) -> Result<String, MultikeyError> {
    let request = CodecMultikeyEncodeRequestView {
        codec_name,
        public_key,
    };
    encode_multikey_primitive(request.codec_name, request.public_key)
}

/// Parse and validate a multikey string.
pub fn parse_multikey_value(input: &str) -> Result<ParsedMultikey, MultikeyError> {
    let request = CodecTextRequestView { value: input };
    let input = request.value;
    parse_multikey(input)
}

/// Check whether a binding type names the selected codec.
pub fn binding_matches_codec(binding_type: &str, codec_name: &str) -> bool {
    let request = CodecBindingTypeMatchesCodecRequestView {
        binding_type,
        codec_name,
    };
    binding_type_matches_codec(request.binding_type, request.codec_name)
}

#[allow(deprecated)]
/// Validate the parsed multikey against its declared binding.
pub fn validate_binding(
    binding_type: &str,
    algorithm: Option<&str>,
    multikey: &ParsedMultikey,
) -> Result<(), MultikeyError> {
    validate_key_binding(
        KeyBindingInput {
            binding_type,
            algorithm,
        },
        multikey,
    )
}

/// Validate an encoded multikey against its declared binding.
pub fn validate_encoded_binding(
    binding_type: &str,
    algorithm: Option<&str>,
    encoded_multikey: &str,
) -> Result<(), MultikeyError> {
    let request = CodecValidateKeyBindingRequestView {
        binding_type,
        algorithm,
        multikey: encoded_multikey,
    };
    validate_multikey_binding(
        KeyBindingInput {
            binding_type: request.binding_type,
            algorithm: request.algorithm,
        },
        request.multikey,
    )
}

/// Compute a CID over bounded, caller-supplied DAG-CBOR bytes.
pub fn compute_dag_cbor_cid(input: &[u8]) -> Result<String, ScalarOperationError> {
    let request = CodecBytesRequestView { value: input };
    let input = request.value;
    if input.len() > MAX_DAG_CBOR_INPUT_LEN {
        return Err(ScalarOperationError::InputTooLarge);
    }
    Ok(compute_cid_dag_cbor(input))
}

/// Compute the SHA-256 digest over bounded DAG-CBOR bytes.
pub fn dag_cbor_content_hash(input: &[u8]) -> Result<ContentHash, ScalarOperationError> {
    let request = CodecBytesRequestView { value: input };
    let input = request.value;
    if input.len() > MAX_DAG_CBOR_INPUT_LEN {
        return Err(ScalarOperationError::InputTooLarge);
    }
    Ok(sha2_256_content_hash(input))
}

/// Compute a SHA-256 multihash over bounded DAG-CBOR bytes.
pub fn dag_cbor_multihash_value(input: &[u8]) -> Result<DagCborMultihash, ScalarOperationError> {
    let request = CodecBytesRequestView { value: input };
    let input = request.value;
    if input.len() > MAX_DAG_CBOR_INPUT_LEN {
        return Err(ScalarOperationError::InputTooLarge);
    }
    Ok(dag_cbor_multihash(input))
}

/// Parse a CID and return its canonical text when valid.
pub fn parse_cid(input: &str) -> Option<String> {
    let request = CodecTextRequestView { value: input };
    let input = request.value;
    try_parse_cid(input).map(|cid| cid.to_string())
}

/// Report whether the supplied text is a valid CID.
pub fn valid_cid(input: &str) -> bool {
    let request = CodecTextRequestView { value: input };
    let input = request.value;
    is_valid_cid_string(input)
}

/// Return the DAG-CBOR multicodec code.
pub fn dag_cbor_codec_code() -> u32 {
    0x71
}

/// Canonicalize JSON text using the JCS rules.
pub fn canonicalize_json(input: &str) -> Result<String, JcsError> {
    let request = CodecTextRequestView { value: input };
    let input = request.value;
    canonicalize_json_text(input)
}
