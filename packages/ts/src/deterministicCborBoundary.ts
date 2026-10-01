// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

export const MAX_DETERMINISTIC_CBOR_INPUT_LEN = 1_048_576;
export const MAX_DETERMINISTIC_CBOR_OUTPUT_LEN = 1_048_576;
export const MAX_DETERMINISTIC_CBOR_NESTING_DEPTH = 64;
export const MAX_DETERMINISTIC_CBOR_NODES = 65_536;
export const MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES = 16_384;
export const MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES = 1_048_576;
export const MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES = 1_048_576;
// A nested map expands to Value -> Map -> MapEntry on the protobuf wire. The
// protobuf-es counts the root message in addition to the wire's five outer
// layers. Depth 64 maps therefore require one more level than Buffa.
export const MAX_DETERMINISTIC_CBOR_PROTO_RECURSION_DEPTH = 198;
export const DETERMINISTIC_CBOR_U64_MAX = (1n << 64n) - 1n;
export const DETERMINISTIC_CBOR_I64_MIN = -(1n << 63n);
export const DETERMINISTIC_CBOR_NEGATIVE_MAX = -1n;
