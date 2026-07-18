// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! WASM facade over the Rust ReallyMe codec crates.

mod base_encoding;
mod boundary;
mod cbor;
mod jcs;
mod map_error;
mod multiformat;
mod proto_output;

pub use base_encoding::{
    base64_decode, base64_encode, base64url_decode, base64url_encode, bytes_to_lower_hex_wasm,
    lower_hex_to_bytes_wasm,
};
pub use cbor::{
    dag_cbor_codec_code, dag_cbor_compute_cid, dag_cbor_multihash_wasm,
    dag_cbor_sha256_content_hash, is_valid_cid_string_wasm, try_parse_cid_wasm,
};
pub use jcs::canonicalize_json_wasm;
pub use multiformat::{
    base58btc_decode_wasm, base58btc_encode_wasm, binding_type_matches_codec_wasm,
    multibase_base58btc_encode, multibase_base64url_encode, multibase_decode,
    multicodec_strip_prefix, multikey_encode, require_supported_multicodec,
    validate_key_binding_wasm,
};
pub use proto_output::{process_operation, process_operation_json};
