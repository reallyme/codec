// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Denial-of-service regression tests: the decoder must reject hostile
//! length prefixes and nesting *before* it allocates or recurses, so that
//! none of these inputs can abort the process.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codec_cbor::{decode_dag_cbor, CborError, MAX_DAG_CBOR_INPUT_LEN, MAX_NESTING_DEPTH};

#[test]
fn absolute_input_size_cap_rejects_before_parsing() {
    let bytes = vec![0_u8; MAX_DAG_CBOR_INPUT_LEN + 1];
    assert_eq!(decode_dag_cbor(&bytes), Err(CborError::InputTooLarge));
}

#[test]
fn oversized_array_length_rejected_without_allocation() {
    // Array header with count 0x7FFF_FFFF_FFFF_FFFF and no elements.
    // Naively this would `Vec::with_capacity(i64::MAX)` and abort; it must
    // instead fail closed because the input cannot hold that many items.
    let bytes = [0x9b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
    assert_eq!(
        decode_dag_cbor(&bytes),
        Err(CborError::ContainerLengthExceedsInput)
    );
}

#[test]
fn array_length_of_2_pow_32_rejected() {
    // Count 2^32: enough to attempt a ~137 GB reservation on a 64-bit host.
    let bytes = [0x9b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00];
    assert_eq!(
        decode_dag_cbor(&bytes),
        Err(CborError::ContainerLengthExceedsInput)
    );
}

#[test]
fn oversized_map_length_rejected_without_allocation() {
    // Map header (major type 5) with an enormous entry count.
    let bytes = [0xbb, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
    assert_eq!(
        decode_dag_cbor(&bytes),
        Err(CborError::ContainerLengthExceedsInput)
    );
}

#[test]
fn deeply_nested_arrays_rejected_before_stack_overflow() {
    // `0x81` is "array of length 1"; a long run of them is a linked list
    // of containers that would recurse the decoder once per byte.
    let bytes = vec![0x81u8; MAX_NESTING_DEPTH + 8];
    assert_eq!(decode_dag_cbor(&bytes), Err(CborError::DepthExceeded));
}

#[test]
fn nesting_exactly_at_limit_is_accepted() {
    // MAX_NESTING_DEPTH nested single-element arrays wrapping one integer.
    // This is the deepest structure the decoder will accept, so it must
    // decode rather than trip the depth guard.
    let mut bytes = vec![0x81u8; MAX_NESTING_DEPTH];
    bytes.push(0x00); // innermost value: integer 0
    let decoded = decode_dag_cbor(&bytes).expect("depth at the limit must decode");

    // Walk back down to confirm the structure round-tripped intact.
    let mut cursor = &decoded;
    for _ in 0..MAX_NESTING_DEPTH {
        match cursor {
            codec_cbor::CborValue::Array(items) => {
                assert_eq!(items.len(), 1);
                cursor = &items[0];
            }
            other => panic!("expected array, found {other:?}"),
        }
    }
    assert_eq!(cursor, &codec_cbor::CborValue::Int(0));
}

#[test]
fn deeply_nested_maps_rejected() {
    // `0xa1` is "map of length 1"; its key must be a string. Nest maps
    // under a single string key until the depth guard trips.
    // Encoding of one level: A1 60 (map(1), empty-string key) then value.
    let mut bytes = Vec::new();
    for _ in 0..(MAX_NESTING_DEPTH + 4) {
        bytes.push(0xa1); // map, 1 entry
        bytes.push(0x60); // key: empty text string
    }
    bytes.push(0x00); // innermost value
    assert_eq!(decode_dag_cbor(&bytes), Err(CborError::DepthExceeded));
}

#[test]
fn truncated_array_still_rejected() {
    // Count 3 but no elements present: within the length bound (3 <= a few
    // remaining bytes would be false here), so it must fail as truncated
    // input rather than panic.
    let bytes = [0x83u8]; // array(3), nothing follows
    assert!(decode_dag_cbor(&bytes).is_err());
}

#[test]
fn nested_fill_remaining_arrays_do_not_preallocate_declared_counts() {
    // Regression for allocation amplification: every outer array declares that
    // it will contain nearly all remaining bytes, then spends its first element
    // on another oversized array. The decoder must reject the eventual
    // truncation without reserving each ancestor's full declared count.
    let mut bytes = Vec::new();
    for remaining in (1_u16..=64_u16).rev() {
        bytes.push(0x98); // array with one-byte count
        bytes.push(u8::try_from(remaining).expect("test count fits in u8"));
    }
    bytes.push(0x00);

    assert!(decode_dag_cbor(&bytes).is_err());
}
