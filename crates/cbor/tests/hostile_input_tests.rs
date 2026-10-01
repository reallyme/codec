// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Denial-of-service regression tests: the decoder must reject hostile
//! length prefixes and nesting *before* it allocates or recurses, so that
//! none of these inputs can abort the process.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codec_cbor::{
    decode_dag_cbor, encode_dag_cbor, CborError, CborValue, MAX_DAG_CBOR_CONTAINER_ENTRIES,
    MAX_DAG_CBOR_INPUT_LEN, MAX_DAG_CBOR_NODES, MAX_NESTING_DEPTH,
};

#[test]
fn dag_container_entry_limit_accepts_boundary_and_rejects_next() {
    let mut accepted = vec![0x99, 0x40, 0x00];
    accepted.extend(core::iter::repeat_n(0x00, MAX_DAG_CBOR_CONTAINER_ENTRIES));
    let value = decode_dag_cbor(&accepted).expect("entry limit is accepted");
    assert_eq!(encode_dag_cbor(&value), Ok(accepted));

    let mut rejected = vec![0x99, 0x40, 0x01];
    rejected.extend(core::iter::repeat_n(
        0x00,
        MAX_DAG_CBOR_CONTAINER_ENTRIES + 1,
    ));
    assert_eq!(
        decode_dag_cbor(&rejected),
        Err(CborError::ContainerEntriesExceeded),
    );
    let semantic = CborValue::Array(
        core::iter::repeat_with(|| CborValue::Null)
            .take(MAX_DAG_CBOR_CONTAINER_ENTRIES + 1)
            .collect(),
    );
    assert_eq!(
        encode_dag_cbor(&semantic),
        Err(CborError::ContainerEntriesExceeded),
    );
}

#[test]
fn absolute_input_size_cap_rejects_before_parsing() {
    let bytes = vec![0_u8; MAX_DAG_CBOR_INPUT_LEN + 1];
    assert_eq!(decode_dag_cbor(&bytes), Err(CborError::InputTooLarge));
}

#[test]
fn debug_output_never_contains_document_bytes() {
    let value = CborValue::Map(vec![(
        "private-name".to_owned(),
        CborValue::Bytes(b"private-bytes".to_vec()),
    )]);
    let debug = format!("{value:?}");
    assert!(!debug.contains("private-name"));
    assert!(!debug.contains("private-bytes"));
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
fn encode_nesting_exactly_at_limit_is_accepted_and_next_is_rejected() {
    let mut at_limit = CborValue::Int(0);
    for _ in 0..MAX_NESTING_DEPTH {
        at_limit = CborValue::Array(vec![at_limit]);
    }
    let encoded = encode_dag_cbor(&at_limit).expect("depth at the limit must encode");
    let mut expected = vec![0x81; MAX_NESTING_DEPTH];
    expected.push(0x00);
    assert_eq!(encoded.as_slice(), expected);

    let over_limit = CborValue::Array(vec![at_limit]);
    assert_eq!(encode_dag_cbor(&over_limit), Err(CborError::DepthExceeded));
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
fn nested_declared_arrays_share_the_global_node_budget() {
    // Every length is minimally encoded and individually plausible from the
    // remaining input. Their combined declarations exceed the node budget,
    // so the decoder must reject before reserving the fourth container.
    let mut bytes = Vec::new();
    for _ in 0..5 {
        bytes.extend_from_slice(&[0x99, 0x40, 0x00]);
    }
    bytes.extend(core::iter::repeat_n(0x00, MAX_DAG_CBOR_CONTAINER_ENTRIES));

    assert_eq!(decode_dag_cbor(&bytes), Err(CborError::NodeLimitExceeded));
}

#[test]
fn dag_global_node_budget_accepts_exact_limit_and_rejects_next_in_both_directions() {
    // Four child arrays stay below the per-container cap, so only the global
    // node guard can distinguish these two otherwise canonical documents.
    let partitions = [16_383, 16_383, 16_383, 16_382];
    assert_eq!(
        1 + partitions.len() + partitions.iter().sum::<usize>(),
        MAX_DAG_CBOR_NODES
    );
    let mut value = CborValue::Array(
        partitions
            .into_iter()
            .map(|count| CborValue::Array((0..count).map(|_| CborValue::Int(0)).collect()))
            .collect(),
    );
    let exact = encode_dag_cbor(&value).expect("node limit must be reachable");
    let decoded = decode_dag_cbor(&exact);
    assert_eq!(decoded.as_ref(), Ok(&value));

    let CborValue::Array(outer) = &mut value else {
        panic!("constructed array changed variant");
    };
    let CborValue::Array(last) = &mut outer[3] else {
        panic!("constructed child array changed variant");
    };
    last.push(CborValue::Int(0));
    assert_eq!(encode_dag_cbor(&value), Err(CborError::NodeLimitExceeded));

    let mut over = exact;
    // The fourth child header is array(16,382): change it to array(16,383)
    // and append one scalar. All earlier lengths remain canonical.
    let fourth_header = 1 + 3 * (3 + 16_383);
    assert_eq!(&over[fourth_header..fourth_header + 3], &[0x99, 0x3f, 0xfe]);
    over[fourth_header + 2] = 0xff;
    over.push(0x00);
    assert_eq!(decode_dag_cbor(&over), Err(CborError::NodeLimitExceeded));
}

#[test]
fn dag_map_entry_cap_rejects_before_constructing_entries() {
    let over_count = MAX_DAG_CBOR_CONTAINER_ENTRIES + 1;
    let value = CborValue::Map(
        (0..over_count)
            .map(|index| (index.to_string(), CborValue::Int(0)))
            .collect(),
    );
    assert_eq!(
        encode_dag_cbor(&value),
        Err(CborError::ContainerEntriesExceeded)
    );

    let mut bytes = vec![0xb9, 0x40, 0x01];
    for _ in 0..over_count {
        bytes.extend_from_slice(&[0x60, 0x00]);
    }
    assert_eq!(
        decode_dag_cbor(&bytes),
        Err(CborError::ContainerEntriesExceeded)
    );
}

#[test]
fn float_rejection_metadata_never_contains_payload_bits() {
    for initial in [0xf9, 0xfa, 0xfb] {
        for fill in [0x61, 0x00, 0xff] {
            let mut bytes = vec![fill; 9];
            bytes[0] = initial;
            assert_eq!(
                decode_dag_cbor(&bytes),
                Err(CborError::DisallowedSimpleValue {
                    value: u64::from(initial & 0x1f)
                })
            );
        }
    }
}

#[test]
fn dag_cbor_values_support_explicit_recursive_zeroization() {
    use codec_cbor::CborValue;
    use zeroize::Zeroize;

    let mut value = CborValue::Map(vec![(
        "identity-key".to_owned(),
        CborValue::Array(vec![
            CborValue::String("example".to_owned()),
            CborValue::Bytes(vec![0xa5; 32]),
        ]),
    )]);
    value.zeroize();
    assert_eq!(value, CborValue::Map(Vec::new()));
    let mut scalar = CborValue::Int(42);
    scalar.zeroize();
    assert_eq!(scalar, CborValue::Int(0));
}
