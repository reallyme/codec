// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    decode_json, decode_protobuf, decode_protobuf_with_limit, validate_json_nesting,
    MAX_CODEC_PROTO_ELEMENT_MEMORY_BYTES, MAX_CODEC_PROTO_JSON_BYTES,
    MAX_CODEC_PROTO_JSON_NESTING_DEPTH, MAX_CODEC_PROTO_JSON_TOKENS,
    MAX_CODEC_PROTO_SEMANTIC_NODES,
};
use crate::generated::proto::reallyme::codec::v1::{
    CodecDeterministicCborArray, CodecDeterministicCborValue, CodecErrorReason,
    CodecPemDecodeRequest,
};

#[test]
fn binary_byte_cap_accepts_exact_size_and_rejects_next_byte() {
    let encoded = [0x0a, 0x01, b'a'];
    assert!(decode_protobuf_with_limit::<CodecPemDecodeRequest>(&encoded, encoded.len()).is_ok());
    assert_eq!(
        decode_protobuf_with_limit::<CodecPemDecodeRequest>(&encoded, encoded.len() - 1)
            .err()
            .map(|error| error.reason()),
        Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED)
    );
}

#[test]
fn proto_json_byte_cap_accepts_exact_size_and_rejects_next_byte() {
    let mut exact = vec![b' '; MAX_CODEC_PROTO_JSON_BYTES - 2];
    exact.extend_from_slice(b"{}");
    assert!(decode_json::<CodecPemDecodeRequest>(&exact).is_ok());
    exact.push(b' ');
    assert_eq!(
        decode_json::<CodecPemDecodeRequest>(&exact)
            .err()
            .map(|error| error.reason()),
        Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED)
    );
}

#[test]
fn binary_element_memory_limit_rejects_repeated_empty_messages() {
    let element_size = core::mem::size_of::<CodecDeterministicCborValue>();
    assert!(element_size > 0);
    let count = (MAX_CODEC_PROTO_ELEMENT_MEMORY_BYTES / element_size) + 1;
    let mut encoded = Vec::with_capacity(count * 2);
    for _ in 0..count {
        encoded.extend_from_slice(&[0x0a, 0]);
    }
    assert_eq!(
        decode_protobuf::<CodecDeterministicCborArray>(&encoded)
            .err()
            .map(|error| error.reason()),
        Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED)
    );
}

#[test]
fn proto_json_nesting_accepts_exact_limit_and_rejects_one_more() {
    // Test the structural scanner directly so an unrelated generated
    // message shape or deserializer recursion limit cannot mask this cap.
    let mut exact = vec![b'['; MAX_CODEC_PROTO_JSON_NESTING_DEPTH];
    exact.push(b'0');
    exact.extend(core::iter::repeat_n(
        b']',
        MAX_CODEC_PROTO_JSON_NESTING_DEPTH,
    ));
    assert!(validate_json_nesting(&exact).is_ok());

    let mut over_limit = Vec::with_capacity(exact.len() + 2);
    over_limit.push(b'[');
    over_limit.extend_from_slice(&exact);
    over_limit.push(b']');
    assert_eq!(
        validate_json_nesting(&over_limit)
            .err()
            .map(|error| error.reason()),
        Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED)
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn proto_json_token_budget_accepts_exact_limit_and_counts_literals() {
    // Repeated object fields exercise the token budget independently of
    // the array-element budget; the full parser rejects duplicates later.
    let scalar_count = MAX_CODEC_PROTO_JSON_TOKENS.checked_sub(4).unwrap() / 2;
    let capacity = scalar_count
        .checked_mul(6)
        .unwrap()
        .checked_add(16)
        .unwrap();
    let mut exact = Vec::with_capacity(capacity);
    exact.extend_from_slice(b"{\"x\":[0]");
    for _ in 0..scalar_count {
        exact.extend_from_slice(b",\"x\":0");
    }
    exact.push(b'}');
    assert!(validate_json_nesting(&exact).is_ok());

    // The next scalar is a literal so its accounting cannot be hidden by
    // the number-token branch used to fill the accepted boundary.
    exact.pop();
    exact.extend_from_slice(b",\"x\":true}");
    assert_eq!(
        validate_json_nesting(&exact)
            .err()
            .map(|error| error.reason()),
        Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED)
    );
}

#[test]
fn proto_json_pre_scan_charges_map_entries_before_deserialization() {
    let entries = MAX_CODEC_PROTO_SEMANTIC_NODES / 2;
    for property in ["entries", "en\\u0074ries"] {
        let mut exact = format!("{{\"{property}\":[").into_bytes();
        for index in 0..entries {
            if index != 0 {
                exact.push(b',');
            }
            exact.extend_from_slice(b"{\"key\":0,\"value\":0}");
        }
        exact.extend_from_slice(b"]}");
        assert!(validate_json_nesting(&exact).is_ok());

        exact.truncate(exact.len() - 2);
        exact.extend_from_slice(b",{\"key\":0,\"value\":0}]}");
        assert_eq!(
            validate_json_nesting(&exact)
                .err()
                .map(|error| error.reason()),
            Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED)
        );
    }
}

#[test]
fn proto_json_pre_scan_rejects_empty_map_entries_before_allocation() {
    for property in ["entries", "en\\u0074ries"] {
        let input = format!("{{\"mapValue\":{{\"{property}\":[{{  }}]}}}}");
        assert_eq!(
            validate_json_nesting(input.as_bytes())
                .err()
                .map(|error| error.reason()),
            Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_JSON)
        );
    }
}

#[test]
fn proto_json_scanner_rejects_unclosed_structure_independently() {
    for incomplete in [b"[".as_slice(), b"\"".as_slice(), b"\"\\".as_slice()] {
        assert_eq!(
            validate_json_nesting(incomplete)
                .err()
                .map(|error| error.reason()),
            Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_JSON)
        );
    }
}

#[test]
fn proto_json_scanner_leaves_non_number_punctuation_to_the_parser() {
    // This pre-scan only rejects fractional syntax after a number token.
    // The full JSON parser rejects a standalone period later.
    assert!(validate_json_nesting(b".").is_ok());
}
