// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Deterministic DAG-CBOR codec for authoritative, cryptographically
//! signed data.
//!
//! The decoder is strict by construction: it rejects non-canonical
//! integers, indefinite-length items, floats, tags, out-of-order map keys,
//! and trailing bytes, so a given value has exactly one accepted encoding.
//! Decoding untrusted input is bounded in input size, memory, and stack
//! depth — container length prefixes are checked against the remaining input
//! before any allocation, and nesting is capped at [`MAX_NESTING_DEPTH`] — so
//! neither a crafted length nor pathological nesting can drive an
//! out-of-memory or stack-overflow abort.

mod cid;
mod decode_dag_cbor;
mod decode_deterministic_cbor;
mod deterministic;
mod encode_dag_cbor;
mod encode_deterministic_cbor;
mod error;
mod value;

/// Maximum array/map nesting depth accepted by [`decode_dag_cbor`].
///
/// Authoritative documents in this system are shallow; this bound is far
/// above any legitimate structure while still stopping a hostile input
/// from recursing the decoder into a stack overflow.
pub const MAX_NESTING_DEPTH: usize = 128;

/// Maximum encoded DAG-CBOR byte length accepted at public decode/hash
/// boundaries.
///
/// This is a defense-in-depth bound for authoritative signed documents. It is
/// intentionally much larger than expected production payloads while keeping
/// parser, hash, and allocation work predictable under hostile input.
pub const MAX_DAG_CBOR_INPUT_LEN: usize = 1024 * 1024;

pub use cid::{
    compute_cid_dag_cbor, dag_cbor_multihash, is_valid_cid_string, sha2_256_content_hash,
    try_parse_cid, verify_dag_cbor_cid, ContentHash, DagCborMultihash, DAG_CBOR_CODEC,
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
