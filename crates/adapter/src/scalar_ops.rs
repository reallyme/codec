// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Compatibility names for the shared generated-contract scalar implementation.
//!
//! Keep this internal crate as a thin facade so existing native and WASM imports
//! retain their signatures without acquiring an independent policy path.

pub use codec_runtime::operation_contract::scalar_ops::{
    binding_matches_codec, canonicalize_json, compute_dag_cbor_cid, dag_cbor_codec_code,
    dag_cbor_content_hash, dag_cbor_multihash_value, decode_base58btc, decode_base64,
    decode_base64url, decode_lower_hex, decode_multibase, encode_base58btc, encode_base64,
    encode_base64url, encode_lower_hex, encode_multibase_base58btc, encode_multibase_base64url,
    encode_multikey, parse_cid, parse_multikey_value, valid_cid, validate_binding,
    validate_encoded_binding, HexError, JcsError, MultikeyError, ScalarOperationError,
    DAG_CBOR_CODEC_CODE, MAX_BASE58BTC_INPUT_BYTES, MAX_DAG_CBOR_INPUT_BYTES,
};
