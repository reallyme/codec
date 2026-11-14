// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Maximum array/map nesting depth accepted by DAG-CBOR encode and decode.
///
/// Limits recursive traversal of caller-controlled containers. Runtime
/// adapters may enforce stricter transport limits.
pub const MAX_NESTING_DEPTH: usize = 64;

/// Maximum semantic nodes accepted by the DAG-CBOR profile.
pub const MAX_DAG_CBOR_NODES: usize = 65_536;

/// Maximum entries in one DAG-CBOR array or map.
pub const MAX_DAG_CBOR_CONTAINER_ENTRIES: usize = 16_384;

/// Maximum encoded DAG-CBOR byte length accepted by encode and decode.
///
/// This is a defense-in-depth bound for authoritative signed documents. It is
/// intentionally much larger than expected production payloads while keeping
/// parser and allocation work predictable under hostile input. Borrowed-byte
/// hash and CID computation helpers do not enforce this cap or validate CBOR
/// syntax; CID verification does validate the block.
pub const MAX_DAG_CBOR_INPUT_LEN: usize = 1_048_576;
