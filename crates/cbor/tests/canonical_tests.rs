// SPDX-FileCopyrightText: 2026 ReallyMe LLC
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
    decode_dag_cbor, encode_dag_cbor, CborError, CborValue, MAX_DAG_CBOR_INPUT_LEN,
    MAX_NESTING_DEPTH,
};

fn enc(value: &CborValue) -> Vec<u8> {
    encode_dag_cbor(value).unwrap()
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
