// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

internal const val MAX_DETERMINISTIC_CBOR_NESTING_DEPTH: Int = 64
internal const val MAX_DETERMINISTIC_CBOR_NODES: Int = 65_536
internal const val MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES: Int = 16_384
internal const val MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES: Int = 1_048_576
internal const val MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES: Int = 1_048_576
internal const val MAX_CODEC_PROTO_MESSAGE_BYTES: Int = 16_781_312

// One semantic map level expands to Value -> Map -> MapEntry. Five outer/key
// wrappers cover the deepest generated request/result path. Java Protobuf
// Lite's default of 100 cannot carry the documented semantic depth of 64.
internal const val MAX_DETERMINISTIC_CBOR_PROTO_MESSAGE_DEPTH: Int = 197
