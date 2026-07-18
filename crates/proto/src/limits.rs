// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

// The recursive deterministic-CBOR protobuf tree adds wrapper and length-prefix
// bytes around a semantic document. These transport constants deliberately
// duplicate the frozen semantic maxima instead of depending on the codec crate:
// protobuf is a lower-level transport package and must not acquire a semantic
// implementation dependency. Compile-time assertions below and repository
// readiness checks keep the duplicated values synchronized.
const CODEC_PROTO_DETERMINISTIC_CBOR_TEXT_BYTES: usize = 1024 * 1024;
const CODEC_PROTO_DETERMINISTIC_CBOR_BYTE_STRING_BYTES: usize = 1024 * 1024;
const CODEC_PROTO_DETERMINISTIC_CBOR_NODES: usize = 65_536;

// Each semantic node is allowed 128 bytes of protobuf/ProtoJSON structure.
// The largest generated leaf and map-entry paths are substantially smaller;
// this bound includes every nested message tag and length prefix, integer text,
// field name, punctuation, and the map-entry wrapper not counted as a semantic
// node. Keep the margin explicit so generator naming changes remain bounded.
const CODEC_PROTO_MAX_STRUCTURAL_BYTES_PER_CBOR_NODE: usize = 128;
const CODEC_PROTO_MAX_FIXED_OPERATION_BYTES: usize = 4096;
const CODEC_PROTO_JSON_MAX_TEXT_ESCAPE_EXPANSION: usize = 6;

/// Maximum accepted binary protobuf message size at codec wire boundaries.
///
/// This derived cap preserves every value inside the deterministic-CBOR
/// semantic byte/node limits after recursive generated-protobuf wrapping. It is
/// a transport limit, not permission for a larger semantic CBOR document.
pub const MAX_CODEC_PROTO_MESSAGE_BYTES: usize = max_codec_proto_message_bytes_const();

/// Maximum accepted `CodecError` envelope size at codec wire boundaries.
pub const MAX_CODEC_PROTO_ERROR_ENVELOPE_BYTES: usize = 4096;

/// Maximum accepted proto3 JSON message size at codec wire boundaries.
///
/// The derivation includes worst-case JSON escaping for text, base64 expansion
/// for byte strings, recursive generated field structure, and fixed operation
/// framing. Semantic validation still applies the smaller CBOR profile limits.
pub const MAX_CODEC_PROTO_JSON_BYTES: usize = max_codec_proto_json_bytes_const();

const fn max_codec_proto_message_bytes_const() -> usize {
    let Some(payload_bytes) = CODEC_PROTO_DETERMINISTIC_CBOR_TEXT_BYTES
        .checked_add(CODEC_PROTO_DETERMINISTIC_CBOR_BYTE_STRING_BYTES)
    else {
        return 0;
    };
    let Some(structural_bytes) = CODEC_PROTO_DETERMINISTIC_CBOR_NODES
        .checked_mul(CODEC_PROTO_MAX_STRUCTURAL_BYTES_PER_CBOR_NODE)
    else {
        return 0;
    };
    let Some(with_structure) = payload_bytes.checked_add(structural_bytes) else {
        return 0;
    };
    match with_structure.checked_add(CODEC_PROTO_MAX_FIXED_OPERATION_BYTES) {
        Some(limit) => limit,
        None => 0,
    }
}

const fn max_codec_proto_json_bytes_const() -> usize {
    let Some(text_bytes) = CODEC_PROTO_DETERMINISTIC_CBOR_TEXT_BYTES
        .checked_mul(CODEC_PROTO_JSON_MAX_TEXT_ESCAPE_EXPANSION)
    else {
        return 0;
    };
    let Some(byte_string_groups) = CODEC_PROTO_DETERMINISTIC_CBOR_BYTE_STRING_BYTES.checked_add(2)
    else {
        return 0;
    };
    let Some(byte_string_bytes) = (byte_string_groups / 3).checked_mul(4) else {
        return 0;
    };
    let Some(structural_bytes) = CODEC_PROTO_DETERMINISTIC_CBOR_NODES
        .checked_mul(CODEC_PROTO_MAX_STRUCTURAL_BYTES_PER_CBOR_NODE)
    else {
        return 0;
    };
    let Some(payload_bytes) = text_bytes.checked_add(byte_string_bytes) else {
        return 0;
    };
    let Some(with_structure) = payload_bytes.checked_add(structural_bytes) else {
        return 0;
    };
    match with_structure.checked_add(CODEC_PROTO_MAX_FIXED_OPERATION_BYTES) {
        Some(limit) => limit,
        None => 0,
    }
}

// One semantic map level expands to Value -> Map -> MapEntry on the wire.
// The fully discriminated response and the deepest integer-key wrapper add at
// most five further message layers. Derive the transport recursion allowance
// from that schema expansion so a documented-valid depth-64 CBOR tree remains
// reachable through both request and response contracts.
const CODEC_PROTO_DETERMINISTIC_CBOR_NESTING_DEPTH: u32 = 64;
const CODEC_PROTO_MESSAGE_LAYERS_PER_CBOR_MAP_DEPTH: u32 = 3;
const CODEC_PROTO_OUTER_AND_KEY_WRAPPER_LAYERS: u32 = 5;

// Generated ProtoJSON represents one nested map value as a value object, map
// object, entries array, and entry object. Bound JSON structure explicitly
// before disabling serde_json's lower generic recursion cap; otherwise a valid
// depth-64 semantic tree is rejected before the semantic validator sees it.
const CODEC_PROTO_JSON_CONTAINERS_PER_CBOR_MAP_DEPTH: usize = 4;
const CODEC_PROTO_JSON_OUTER_AND_KEY_CONTAINERS: usize = 8;

pub(crate) const MAX_CODEC_PROTO_JSON_NESTING_DEPTH: usize = codec_proto_json_nesting_depth();
pub(crate) const CODEC_PROTO_RECURSION_LIMIT: u32 = codec_proto_recursion_limit();
pub(crate) const CODEC_PROTO_UNKNOWN_FIELD_LIMIT: usize = 0;

const fn codec_proto_recursion_limit() -> u32 {
    let Some(recursive_layers) = CODEC_PROTO_DETERMINISTIC_CBOR_NESTING_DEPTH
        .checked_mul(CODEC_PROTO_MESSAGE_LAYERS_PER_CBOR_MAP_DEPTH)
    else {
        return 0;
    };
    match recursive_layers.checked_add(CODEC_PROTO_OUTER_AND_KEY_WRAPPER_LAYERS) {
        Some(limit) => limit,
        None => 0,
    }
}

const fn codec_proto_json_nesting_depth() -> usize {
    let semantic_depth = CODEC_PROTO_DETERMINISTIC_CBOR_NESTING_DEPTH as usize;
    let Some(recursive_containers) =
        semantic_depth.checked_mul(CODEC_PROTO_JSON_CONTAINERS_PER_CBOR_MAP_DEPTH)
    else {
        return 0;
    };
    match recursive_containers.checked_add(CODEC_PROTO_JSON_OUTER_AND_KEY_CONTAINERS) {
        Some(limit) => limit,
        None => 0,
    }
}

const _: () = assert!(CODEC_PROTO_RECURSION_LIMIT != 0);
const _: () = assert!(MAX_CODEC_PROTO_JSON_NESTING_DEPTH != 0);
const _: () = assert!(MAX_CODEC_PROTO_MESSAGE_BYTES == 10_489_856);
const _: () = assert!(MAX_CODEC_PROTO_JSON_BYTES == 16_082_264);
