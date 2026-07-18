// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! # reallyme-codec
//!
//! Codec-only utilities used by ReallyMe and DID tooling: base encodings,
//! canonical JSON/CBOR serialization, multicodec lookup, multikey handling, and
//! PEM text armor.
//! This crate deliberately has no cryptographic primitive dependencies.

#![forbid(unsafe_code)]

/// Standard (RFC 4648) base64 encode/decode.
#[cfg(feature = "base64")]
pub mod base64 {
    pub use codec_base64::{base64_to_bytes, bytes_to_base64, Base64Error};
}

/// URL-safe (RFC 4648 §5) base64 encode/decode without padding.
#[cfg(feature = "base64url")]
pub mod base64url {
    pub use codec_base64url::{
        base64url_bytes_to_bytes, base64url_to_bytes, bytes_to_base64url, Base64UrlError,
    };

    #[cfg(feature = "serde")]
    pub use codec_base64url::{serde_bytes, serde_option_bytes};
}

/// DAG-CBOR encode/decode and content-identifier helpers.
#[cfg(feature = "cbor")]
pub mod cbor {
    pub use codec_cbor::{
        compute_cid_dag_cbor, dag_cbor_multihash, decode_dag_cbor, decode_deterministic_cbor,
        encode_dag_cbor, encode_deterministic_cbor, is_valid_cid_string, sha2_256_content_hash,
        try_parse_cid, verify_dag_cbor_cid, CborError, CborValue, ContentHash, DagCborMultihash,
        DeterministicCborError, DeterministicCborInteger, DeterministicCborMapEntry,
        DeterministicCborMapKey, DeterministicCborNegativeInteger, DeterministicCborProfileError,
        DeterministicCborValue, DAG_CBOR_CODEC, DETERMINISTIC_CBOR_NEGATIVE_MAX,
        DETERMINISTIC_CBOR_NEGATIVE_MIN, MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
        MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES, MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES,
        MAX_DETERMINISTIC_CBOR_INPUT_LEN, MAX_DETERMINISTIC_CBOR_NESTING_DEPTH,
        MAX_DETERMINISTIC_CBOR_NODES, MAX_DETERMINISTIC_CBOR_OUTPUT_LEN,
    };
}

/// Canonical lowercase hexadecimal encode/decode helpers.
#[cfg(feature = "hex")]
pub mod hex {
    pub use codec_hex::{bytes_to_lower_hex, lower_hex_to_bytes, write_lower_hex, HexError};
}

/// JSON Canonicalization Scheme helpers.
#[cfg(feature = "jcs")]
pub mod jcs {
    pub use codec_jcs::{canonicalize_json_text, canonicalize_trusted_json_value, JcsError};
}

/// Multibase self-describing base encodings.
#[cfg(feature = "multibase")]
pub mod multibase {
    pub use codec_multibase::{
        base58btc_decode, base58btc_encode, bytes_to_multibase58btc, bytes_to_multibase_base64url,
        multibase_to_bytes, Base58Error, MultibaseError,
    };
}

/// Semantic multicodec registry lookup, prefix handling, and metadata.
#[cfg(feature = "multicodec")]
pub mod multicodec;

/// Multikey encoding/parsing that binds an algorithm to opaque key bytes.
#[cfg(feature = "multikey")]
pub mod multikey {
    pub use codec_multikey::{
        binding_type_matches_codec, encode_multikey, parse_multikey, validate_key_binding,
        KeyBindingInput, MultikeyError, ParsedMultikey,
    };
}

/// PEM text armor parsing and encoding.
#[cfg(feature = "pem")]
pub mod pem {
    pub use codec_pem::{
        decode_pem, encode_pem, PemDecodePolicy, PemDocument, PemEncodeOptions, PemError, PemLabel,
        PemLineEnding,
    };
}

/// Generated protobuf operation contract lane.
#[cfg(feature = "operation-contract")]
pub mod operation_contract;

/// Adapter-facing semantic layer for scalar and raw-byte operations.
#[cfg(feature = "operation-contract")]
#[doc(hidden)]
pub mod scalar_ops;
