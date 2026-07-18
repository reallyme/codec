// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

/// Maximum encoded deterministic-CBOR byte length accepted at public
/// decode/hash boundaries.
///
/// The generic deterministic profile may carry identity-adjacent text and byte
/// strings. Keep this bound explicit and shared so every adapter can reject
/// hostile payload sizes before large allocations or cross-runtime copies.
pub const MAX_DETERMINISTIC_CBOR_INPUT_LEN: usize = 1024 * 1024;

/// Maximum deterministic-CBOR byte length the encoder may produce.
///
/// This separate name prevents future callers from accidentally coupling an
/// output allocation limit to a transport-specific input limit.
pub const MAX_DETERMINISTIC_CBOR_OUTPUT_LEN: usize = 1024 * 1024;

/// Maximum recursive array/map nesting depth for deterministic generic CBOR.
///
/// The generic recursive value model crosses native, JVM, TypeScript, WASM,
/// protobuf, and C ABI lanes. The value is deliberately stricter than the
/// existing DAG-CBOR bound and is backed by matching recursion tests in every
/// exposed lane. Changing it is a profile-version decision, not runtime tuning.
/// A scalar root has depth zero. A root array or map has depth one, and each
/// child array or map increases the depth by one.
pub const MAX_DETERMINISTIC_CBOR_NESTING_DEPTH: usize = 64;

/// Maximum semantic nodes in a deterministic-CBOR value tree.
///
/// Every [`crate::DeterministicCborValue`] counts as one node and every map key
/// counts as one additional node. Map-entry wrappers and array slots do not add
/// a node beyond their contained key/value owners. The top-level value is
/// included. This definition must be identical in every transport adapter so
/// the limit caps traversal work independently of encoded representation.
pub const MAX_DETERMINISTIC_CBOR_NODES: usize = 65_536;

/// Maximum entries in a single deterministic-CBOR array or map.
///
/// This prevents one declared container from driving disproportionate sort,
/// validation, or allocation work even when aggregate node and byte limits are
/// still below their caps. An array entry is one element; a map entry is one
/// key/value pair.
pub const MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES: usize = 16_384;

/// Maximum aggregate UTF-8 bytes across all text values and text map keys.
///
/// The aggregate is calculated with checked addition over exact UTF-8 byte
/// lengths, not Unicode scalar counts or UTF-16 code units.
pub const MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES: usize = 1024 * 1024;

/// Maximum aggregate bytes across all byte-string values.
pub const MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES: usize = 1024 * 1024;

/// Smallest negative integer supported by the deterministic generic profile.
///
/// CBOR can represent negative integers outside this range. ReallyMe Codec
/// intentionally does not expose that larger domain in `0.2.0` because every
/// supported SDK lane must preserve values exactly.
pub const DETERMINISTIC_CBOR_NEGATIVE_MIN: i64 = i64::MIN;

/// Largest negative integer supported by the deterministic generic profile.
pub const DETERMINISTIC_CBOR_NEGATIVE_MAX: i64 = -1;
