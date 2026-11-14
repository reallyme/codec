// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(missing_docs)]
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unwrap_used
)]
use codec_cbor::{
    decode_dag_cbor, decode_deterministic_cbor, encode_dag_cbor, encode_deterministic_cbor,
    CborError, CborValue, DeterministicCborError, DeterministicCborInteger,
    DeterministicCborMapEntry, DeterministicCborMapKey, DeterministicCborValue,
    MAX_DAG_CBOR_INPUT_LEN, MAX_NESTING_DEPTH,
};
use std::cmp::Ordering;

fn enc(value: &CborValue) -> Vec<u8> {
    encode_dag_cbor(value).unwrap()
}

#[test]
fn nested_unsigned_and_text_keys_reject_bytewise_order() {
    let canonical = [0xa1, 0x60, 0xa2, 0x61, b'z', 0xf4, 0x19, 0x01, 0x00, 0xf5];
    let bytewise = [0xa1, 0x60, 0xa2, 0x19, 0x01, 0x00, 0xf5, 0x61, b'z', 0xf4];
    let decoded = decode_deterministic_cbor(&canonical).unwrap();
    assert_eq!(
        encode_deterministic_cbor(&decoded).unwrap().as_slice(),
        canonical
    );
    assert_eq!(
        decode_deterministic_cbor(&bytewise).unwrap_err(),
        DeterministicCborError::MapKeysOutOfOrder
    );

    let mut invalid_value = bytewise;
    // A tag is unsupported as a value in this profile. The decoder must
    // reject the key order before inspecting the value after the second key.
    invalid_value[9] = 0xc0;
    assert_eq!(
        decode_deterministic_cbor(&invalid_value).unwrap_err(),
        DeterministicCborError::MapKeysOutOfOrder
    );

    for unsigned_first in [false, true] {
        let text = DeterministicCborMapEntry::new(
            DeterministicCborMapKey::text("z".into()),
            DeterministicCborValue::Bool(false),
        );
        let unsigned = DeterministicCborMapEntry::new(
            DeterministicCborMapKey::Integer(DeterministicCborInteger::unsigned(256)),
            DeterministicCborValue::Bool(true),
        );
        let inner = DeterministicCborValue::Map(if unsigned_first {
            vec![unsigned, text]
        } else {
            vec![text, unsigned]
        });
        let outer = DeterministicCborValue::Map(vec![DeterministicCborMapEntry::new(
            DeterministicCborMapKey::text(String::new()),
            inner,
        )]);
        assert_eq!(
            encode_deterministic_cbor(&outer).unwrap().as_slice(),
            canonical
        );
    }
}

#[test]
fn nested_mixed_length_text_keys_use_length_first_order() {
    let canonical = [0xa1, 0x60, 0xa2, 0x61, b'z', 0xf4, 0x62, b'a', b'a', 0xf5];
    let reversed = [0xa1, 0x60, 0xa2, 0x62, b'a', b'a', 0xf5, 0x61, b'z', 0xf4];
    let decoded = decode_deterministic_cbor(&canonical).unwrap();
    assert_eq!(
        encode_deterministic_cbor(&decoded).unwrap().as_slice(),
        canonical
    );
    assert_eq!(
        decode_deterministic_cbor(&reversed).unwrap_err(),
        DeterministicCborError::MapKeysOutOfOrder
    );

    let inner = DeterministicCborValue::Map(vec![
        DeterministicCborMapEntry::new(
            DeterministicCborMapKey::text("aa".into()),
            DeterministicCborValue::Bool(true),
        ),
        DeterministicCborMapEntry::new(
            DeterministicCborMapKey::text("z".into()),
            DeterministicCborValue::Bool(false),
        ),
    ]);
    let outer = DeterministicCborValue::Map(vec![DeterministicCborMapEntry::new(
        DeterministicCborMapKey::text(String::new()),
        inner,
    )]);
    assert_eq!(
        encode_deterministic_cbor(&outer).unwrap().as_slice(),
        canonical
    );
}

#[test]
fn nested_empty_keys_and_boolean_leaf_round_trip() {
    // Exercise the public recursive dispatch on the wire shape used by
    // the deterministic CBOR map-layer proofs.
    for leaf in [0xf4, 0xf5] {
        let wire = [0xa1, 0x60, 0xa1, 0x60, leaf];
        assert_eq!(enc(&decode_dag_cbor(&wire).unwrap()), wire);
        assert_eq!(
            encode_deterministic_cbor(&decode_deterministic_cbor(&wire).unwrap())
                .unwrap()
                .as_slice(),
            wire
        );
    }
}

#[test]
fn every_inner_key_and_leaf_byte_has_expected_admission_and_round_trip() {
    // {"": {<one-byte text key>: <one-byte item>}}
    for key in u8::MIN..=u8::MAX {
        for leaf in u8::MIN..=u8::MAX {
            let wire = [0xa1, 0x60, 0xa1, 0x61, key, leaf];
            let dag = decode_dag_cbor(&wire);
            let deterministic = decode_deterministic_cbor(&wire);

            if !key.is_ascii() {
                assert_eq!(dag.err(), Some(CborError::InvalidUtf8), "{wire:02x?}");
                assert_eq!(
                    deterministic.err(),
                    Some(DeterministicCborError::InvalidUtf8),
                    "{wire:02x?}"
                );
                continue;
            }

            // Only inline integers, empty strings and containers, and the
            // three supported simple values fit in the final byte.
            let canonical_leaf =
                matches!(leaf, 0x00..=0x17 | 0x20..=0x37 | 0x40 | 0x60 | 0x80 | 0xa0 | 0xf4..=0xf6);
            assert_eq!(dag.is_ok(), canonical_leaf, "{wire:02x?}");
            assert_eq!(deterministic.is_ok(), canonical_leaf, "{wire:02x?}");
            if canonical_leaf {
                assert_eq!(enc(&dag.unwrap()), wire, "{wire:02x?}");
                assert_eq!(
                    encode_deterministic_cbor(&deterministic.unwrap())
                        .unwrap()
                        .as_slice(),
                    wire,
                    "{wire:02x?}"
                );
            }
        }
    }
}

#[test]
fn every_pair_of_single_byte_inner_text_keys_has_expected_order_and_round_trip() {
    for first in 0..=0x7f_u8 {
        for second in 0..=0x7f_u8 {
            // Distinct leaves make a change to the key/value association
            // visible even when the re-encoded key order is correct.
            let wire = [0xa1, 0x60, 0xa2, 0x61, first, 0xf4, 0x61, second, 0xf5];
            let dag = decode_dag_cbor(&wire);
            let deterministic = decode_deterministic_cbor(&wire);
            match first.cmp(&second) {
                Ordering::Less => {
                    assert_eq!(enc(&dag.unwrap()), wire, "{wire:02x?}");
                    assert_eq!(
                        encode_deterministic_cbor(&deterministic.unwrap())
                            .unwrap()
                            .as_slice(),
                        wire,
                        "{wire:02x?}"
                    );
                }
                Ordering::Equal => {
                    assert_eq!(dag.err(), Some(CborError::DuplicateMapKey), "{wire:02x?}");
                    assert_eq!(
                        deterministic.err(),
                        Some(DeterministicCborError::DuplicateMapKey),
                        "{wire:02x?}"
                    );
                }
                Ordering::Greater => {
                    assert_eq!(dag.err(), Some(CborError::MapKeysOutOfOrder), "{wire:02x?}");
                    assert_eq!(
                        deterministic.err(),
                        Some(DeterministicCborError::MapKeysOutOfOrder),
                        "{wire:02x?}"
                    );
                }
            }
        }
    }
}

#[test]
fn nested_key_order_errors_precede_invalid_second_child() {
    // The last byte would produce an unsupported major type if the decoder
    // visited the second value before checking the key's wire span.
    for (first, second, expected) in [
        (b'a', b'b', DeterministicCborError::UnsupportedMajorType),
        (b'a', b'a', DeterministicCborError::DuplicateMapKey),
        (b'b', b'a', DeterministicCborError::MapKeysOutOfOrder),
    ] {
        let wire = [0xa1, 0x60, 0xa2, 0x61, first, 0xf4, 0x61, second, 0xc0];
        assert_eq!(decode_deterministic_cbor(&wire).unwrap_err(), expected);
    }
}

#[test]
fn every_extended_one_byte_integer_leaf_requires_minimal_complete_input() {
    for head in [0x18, 0x38] {
        for argument in u8::MIN..=u8::MAX {
            // The final byte is outside the declared outer map when the
            // integer is complete. Check both views of the same buffer.
            let wire = [0xa1, 0x60, 0xa1, 0x60, head, argument, 0x00];
            let complete = &wire[..6];
            let dag = decode_dag_cbor(complete);
            let deterministic = decode_deterministic_cbor(complete);
            if argument < 24 {
                assert_eq!(
                    dag.err(),
                    Some(CborError::NonCanonicalInteger),
                    "{wire:02x?}"
                );
                assert_eq!(
                    deterministic.err(),
                    Some(DeterministicCborError::NonCanonicalInteger),
                    "{wire:02x?}"
                );
                continue;
            }

            assert_eq!(enc(&dag.unwrap()), complete, "{wire:02x?}");
            assert_eq!(
                encode_deterministic_cbor(&deterministic.unwrap())
                    .unwrap()
                    .as_slice(),
                complete,
                "{wire:02x?}"
            );
            assert_eq!(
                decode_dag_cbor(&wire).err(),
                Some(CborError::TrailingBytes),
                "{wire:02x?}"
            );
            assert_eq!(
                decode_deterministic_cbor(&wire).err(),
                Some(DeterministicCborError::TrailingBytes),
                "{wire:02x?}"
            );
        }
    }
}

#[test]
fn all_permutations_of_a_nested_map_have_one_accepted_encoding() {
    // The third key is lexically first but must sort last by encoded length.
    let pairs: [&[u8]; 3] = [&[0x61, b'b', 1], &[0x61, b'c', 2], &[0x62, b'a', b'a', 3]];
    let canonical = [
        0xa1, 0x61, b'x', 0xa3, 0x61, b'b', 1, 0x61, b'c', 2, 0x62, b'a', b'a', 3,
    ];
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let mut wire = vec![0xa1, 0x61, b'x', 0xa3];
        for index in order {
            wire.extend_from_slice(pairs[index]);
        }
        if order == [0, 1, 2] {
            assert_eq!(
                encode_dag_cbor(&decode_dag_cbor(&wire).unwrap()).unwrap(),
                canonical
            );
            assert_eq!(
                encode_deterministic_cbor(&decode_deterministic_cbor(&wire).unwrap())
                    .unwrap()
                    .as_slice(),
                canonical
            );
        } else {
            assert!(matches!(
                decode_dag_cbor(&wire),
                Err(CborError::MapKeysOutOfOrder)
            ));
            assert!(matches!(
                decode_deterministic_cbor(&wire),
                Err(DeterministicCborError::MapKeysOutOfOrder)
            ));
        }
    }
}

#[test]
fn null_roundtrip() {
    let v = CborValue::Null;
    assert_eq!(decode_dag_cbor(&enc(&v)).unwrap(), v);
}

#[test]
fn boolean_roundtrip() {
    assert_eq!(
        decode_dag_cbor(&enc(&CborValue::Bool(true))).unwrap(),
        CborValue::Bool(true)
    );
    assert_eq!(
        decode_dag_cbor(&enc(&CborValue::Bool(false))).unwrap(),
        CborValue::Bool(false)
    );
}

#[test]
fn integer_roundtrip() {
    let cases = [0, 42, -1, -1000];
    for n in cases {
        let v = CborValue::Int(n);
        assert_eq!(decode_dag_cbor(&enc(&v)).unwrap(), v);
    }
}

#[test]
fn string_roundtrip() {
    let v = CborValue::String("hello".into());
    assert_eq!(decode_dag_cbor(&enc(&v)).unwrap(), v);
}

#[test]
fn byte_string_roundtrip() {
    let v = CborValue::Bytes(vec![1, 2, 3]);
    assert_eq!(decode_dag_cbor(&enc(&v)).unwrap(), v);
}

#[test]
fn array_roundtrip() {
    let v = CborValue::Array(vec![
        CborValue::Int(1),
        CborValue::Bool(true),
        CborValue::String("x".into()),
        CborValue::Null,
    ]);
    assert_eq!(decode_dag_cbor(&enc(&v)).unwrap(), v);
}

#[test]
fn map_keys_sorted_utf8() {
    let v = CborValue::Map(vec![
        ("b".into(), CborValue::Int(2)),
        ("a".into(), CborValue::Int(1)),
    ]);

    let encoded = enc(&v);
    let decoded = decode_dag_cbor(&encoded).unwrap();

    assert_eq!(
        decoded,
        CborValue::Map(vec![
            ("a".into(), CborValue::Int(1)),
            ("b".into(), CborValue::Int(2)),
        ])
    );
}

#[test]
fn rejects_trailing_bytes() {
    let mut bytes = enc(&CborValue::Int(42));
    bytes.push(0x00);
    assert!(decode_dag_cbor(&bytes).is_err());
}

#[test]
fn rejects_out_of_order_map() {
    // map(2) { "b":1, "a":2 }
    let bad = vec![0xA2, 0x61, 0x62, 0x01, 0x61, 0x61, 0x02];

    assert!(decode_dag_cbor(&bad).is_err());
}

#[test]
fn deep_nested_roundtrip() {
    let v = CborValue::Map(vec![(
        "a".into(),
        CborValue::Map(vec![(
            "b".into(),
            CborValue::Map(vec![(
                "c".into(),
                CborValue::Map(vec![(
                    "d".into(),
                    CborValue::Array(vec![
                        CborValue::Int(1),
                        CborValue::Int(2),
                        CborValue::Int(3),
                        CborValue::Map(vec![("x".into(), CborValue::Bool(true))]),
                    ]),
                )]),
            )]),
        )]),
    )]);

    assert_eq!(decode_dag_cbor(&enc(&v)).unwrap(), v);
}

#[test]
fn encode_rejects_deeply_nested_values() {
    let mut value = CborValue::Null;
    for _ in 0..(MAX_NESTING_DEPTH + 1) {
        value = CborValue::Array(vec![value]);
    }
    assert_eq!(encode_dag_cbor(&value), Err(CborError::DepthExceeded));
}

#[test]
fn encode_rejects_outputs_above_size_cap() {
    let value = CborValue::Bytes(vec![0_u8; MAX_DAG_CBOR_INPUT_LEN + 1]);
    assert_eq!(encode_dag_cbor(&value), Err(CborError::OutputTooLarge));
}

#[test]
fn rejects_duplicate_map_keys() {
    // map(2) { "a":1, "a":2 }
    let bad = vec![0xA2, 0x61, 0x61, 0x01, 0x61, 0x61, 0x02];

    assert_eq!(decode_dag_cbor(&bad), Err(CborError::DuplicateMapKey));
}

#[test]
fn encoder_rejects_duplicate_map_keys() {
    let duplicate = CborValue::Map(vec![
        ("a".into(), CborValue::Int(1)),
        ("a".into(), CborValue::Int(2)),
    ]);

    assert_eq!(encode_dag_cbor(&duplicate), Err(CborError::DuplicateMapKey));
}

#[test]
fn rejects_cbor_tags() {
    // tag(1) followed by integer 42
    let bad = vec![
        0xC1, // major type 6 (tag), value 1
        0x18, 0x2A, // uint(42)
    ];

    assert!(decode_dag_cbor(&bad).is_err());
}

#[test]
fn rejects_indefinite_length_array() {
    // array(*) [ 1, 2 ]
    let bad = vec![
        0x9F, // array, indefinite length
        0x01, 0x02, 0xFF, // break
    ];

    assert!(decode_dag_cbor(&bad).is_err());
}

#[test]
fn rejects_indefinite_length_map() {
    // map(*) { "a": 1 }
    let bad = vec![
        0xBF, // map, indefinite length
        0x61, 0x61, 0x01, 0xFF,
    ];

    assert!(decode_dag_cbor(&bad).is_err());
}

#[test]
fn duplicate_map_keys_are_rejected_before_child_errors() {
    let mut deep = CborValue::Null;
    for _ in 0..=MAX_NESTING_DEPTH {
        deep = CborValue::Array(vec![deep]);
    }
    for invalid_child in [deep, CborValue::Bytes(vec![0; MAX_DAG_CBOR_INPUT_LEN])] {
        let value = CborValue::Map(vec![
            ("duplicate".into(), invalid_child),
            ("duplicate".into(), CborValue::Null),
        ]);
        assert_eq!(encode_dag_cbor(&value), Err(CborError::DuplicateMapKey));
    }
}

#[test]
fn encoded_output_at_exact_limit_preserves_payload() {
    // A byte string of this size has a five-byte CBOR header.
    let payload = vec![0xa5; MAX_DAG_CBOR_INPUT_LEN - 5];
    let encoded = enc(&CborValue::Bytes(payload.clone()));
    assert_eq!(encoded.len(), MAX_DAG_CBOR_INPUT_LEN);
    assert_eq!(&encoded[..5], &[0x5a, 0x00, 0x0f, 0xff, 0xfb]);
    assert_eq!(&encoded[5..], payload);
    assert_eq!(
        decode_dag_cbor(&encoded).unwrap(),
        CborValue::Bytes(payload)
    );
    assert_eq!(
        encode_dag_cbor(&CborValue::Bytes(vec![0xa5; MAX_DAG_CBOR_INPUT_LEN - 4])),
        Err(CborError::OutputTooLarge)
    );
}
