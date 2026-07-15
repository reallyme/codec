// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! WASM facade over the Rust ReallyMe codec crates.

mod base_encoding;
mod cbor;
mod jcs;
mod map_error;
mod multiformat;
mod pem;
mod proto_output;
mod write_js_object;

pub use base_encoding::{
    base64_decode, base64_encode, base64url_decode, base64url_encode, bytes_to_lower_hex_wasm,
    lower_hex_to_bytes_wasm,
};
pub use cbor::{
    dag_cbor_codec_code, dag_cbor_compute_cid, dag_cbor_decode, dag_cbor_encode,
    dag_cbor_multihash_wasm, dag_cbor_sha256_content_hash, dag_cbor_verify_cid,
    is_valid_cid_string_wasm, try_parse_cid_wasm,
};
pub use jcs::canonicalize_json_wasm;
pub use multiformat::{
    base58btc_decode_wasm, base58btc_encode_wasm, binding_type_matches_codec_wasm,
    multibase_base58btc_encode, multibase_base64url_encode, multibase_decode,
    multicodec_lookup_prefix, multicodec_prefix_for_name, multicodec_strip_prefix,
    multicodec_table, multikey_encode, multikey_parse, require_supported_multicodec,
    validate_key_binding_wasm,
};
pub use pem::{pem_decode, pem_encode};
pub use proto_output::{
    dag_cbor_verify_cid_proto, dag_cbor_verify_cid_proto_result, multicodec_lookup_prefix_proto,
    multicodec_lookup_prefix_proto_result, multicodec_prefix_for_name_proto,
    multicodec_prefix_for_name_proto_result, multicodec_table_proto, multicodec_table_proto_result,
    multikey_parse_proto, multikey_parse_proto_result, pem_decode_proto, pem_decode_proto_result,
};
