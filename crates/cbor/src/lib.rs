// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic generic CBOR and a DAG-CBOR subset with content-ID helpers.
//!
//! Both profiles reject non-canonical
//! integers, indefinite-length items, floats, tags, out-of-order map keys,
//! and trailing bytes, so a given value has exactly one accepted encoding.
//! DAG-CBOR supports text map keys and signed 64-bit integers. Generic CBOR
//! also supports integer map keys and the full unsigned 64-bit range.
//! Input-size and nesting limits bound parser work; declared container lengths
//! are checked against remaining input before allocating their contents.

mod cid;
mod decode_dag_cbor;
mod decode_deterministic_cbor;
mod deterministic;
mod encode_dag_cbor;
mod encode_deterministic_cbor;
mod error;
mod value;

/// Maximum array/map nesting depth accepted by DAG-CBOR encode and decode.
///
/// Limits recursive traversal of caller-controlled containers. Runtime
/// adapters may enforce stricter transport limits.
pub const MAX_NESTING_DEPTH: usize = 128;

/// Maximum encoded DAG-CBOR byte length accepted by encode and decode.
///
/// This is a defense-in-depth bound for authoritative signed documents. It is
/// intentionally much larger than expected production payloads while keeping
/// parser and allocation work predictable under hostile input. Borrowed-byte
/// hash and CID helpers do not enforce this cap or validate CBOR syntax.
pub const MAX_DAG_CBOR_INPUT_LEN: usize = 1024 * 1024;

pub use cid::{
    compute_cid_dag_cbor, dag_cbor_multihash, is_valid_cid_string, sha2_256_content_hash,
    try_parse_cid, verify_dag_cbor_cid, ContentHash, DagCborMultihash, DAG_CBOR_CODEC,
    MAX_CID_STRING_LEN,
};
pub use decode_dag_cbor::decode_dag_cbor;
pub use decode_deterministic_cbor::decode_deterministic_cbor;
pub use deterministic::{
    DeterministicCborError, DeterministicCborInteger, DeterministicCborMapEntry,
    DeterministicCborMapKey, DeterministicCborNegativeInteger, DeterministicCborProfileError,
    DeterministicCborValue, DETERMINISTIC_CBOR_NEGATIVE_MAX, DETERMINISTIC_CBOR_NEGATIVE_MIN,
    MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES, MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES,
    MAX_DETERMINISTIC_CBOR_INPUT_LEN, MAX_DETERMINISTIC_CBOR_NESTING_DEPTH,
    MAX_DETERMINISTIC_CBOR_NODES, MAX_DETERMINISTIC_CBOR_OUTPUT_LEN,
};
pub use encode_dag_cbor::encode_dag_cbor;
pub use encode_deterministic_cbor::encode_deterministic_cbor;
pub use error::CborError;
pub use value::CborValue;
