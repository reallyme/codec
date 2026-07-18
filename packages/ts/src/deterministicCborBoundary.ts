// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

export const MAX_DETERMINISTIC_CBOR_INPUT_LEN = 1_048_576;
export const MAX_DETERMINISTIC_CBOR_OUTPUT_LEN = 1_048_576;
export const MAX_DETERMINISTIC_CBOR_NESTING_DEPTH = 64;
export const MAX_DETERMINISTIC_CBOR_NODES = 65_536;
export const MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES = 16_384;
export const MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES = 1_048_576;
export const MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES = 1_048_576;
// A nested map expands to Value -> Map -> MapEntry on the protobuf wire. The
// five additional layers cover the result/request and deepest scalar wrapper.
// Buf's default recursion limit of 100 cannot represent semantic depth 64.
export const MAX_DETERMINISTIC_CBOR_PROTO_RECURSION_DEPTH =
  MAX_DETERMINISTIC_CBOR_NESTING_DEPTH * 3 + 5;
export const DETERMINISTIC_CBOR_U64_MAX = (1n << 64n) - 1n;
export const DETERMINISTIC_CBOR_I64_MIN = -(1n << 63n);
export const DETERMINISTIC_CBOR_NEGATIVE_MAX = -1n;
