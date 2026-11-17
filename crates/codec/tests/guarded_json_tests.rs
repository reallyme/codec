// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "json")]
#![allow(clippy::unwrap_used, missing_docs)]

use std::collections::BTreeMap;

use proptest::collection;
use proptest::prelude::{any, Just, Strategy};
use proptest::{prop_assert_eq, prop_oneof, proptest};
use reallyme_codec::json::{decode, from_slice, ErrorReason as R, Limits};
use serde::{Deserialize, Serialize};

fn reason(input: &[u8], limits: &Limits) -> R {
    decode(input, Some(limits)).unwrap_err().reason()
}

#[test]
fn strings_and_unicode_match_the_standard_parser() {
    for text in [
        r#""""#,
        r#""plain""#,
        r#""a\n\r\t\b\f\/\\\"z""#,
        r#""\u0000""#,
        r#""\uD83D\uDE00""#,
        r#""é/中/😀""#,
    ] {
        assert_eq!(
            from_slice::<String>(text.as_bytes(), None).unwrap(),
            serde_json::from_str::<String>(text).unwrap()
        );
    }
    for text in [
        r#""\uD800""#,
        r#""\uDC00""#,
        r#""\uD800\u0000""#,
        r#""\uDBFFx""#,
    ] {
        assert_eq!(
            reason(text.as_bytes(), &Limits::default()),
            R::CODEC_ERROR_REASON_CANONICAL_JSON_INVALID_UNICODE
        );
    }
}

#[test]
fn duplicate_names_are_decoded_before_comparison() {
    for text in [
        r#"{"a":1,"a":2}"#,
        r#"{"a":1,"\u0061":2}"#,
        r#"{"nested":[{"x":null,"x":true}]}"#,
    ] {
        assert_eq!(
            reason(text.as_bytes(), &Limits::default()),
            R::CODEC_ERROR_REASON_CANONICAL_JSON_DUPLICATE_PROPERTY
        );
    }
    assert!(decode(r#"{"é":1,"e\u0301":2,"A":3,"a":4}"#.as_bytes(), None).is_ok());
}

#[test]
fn malformed_input_is_rejected_without_backend_diagnostics() {
    for text in [
        "", " ", "{} {}", "[1,]", "{\"a\":}", "[", "\"\\q\"", "01", "+1", "NaN", "truex",
    ] {
        assert!(decode(text.as_bytes(), None).is_err(), "case {text}");
    }
    assert!(decode(&[b'"', 0xff, b'"'], None).is_err());
    assert_eq!(
        reason(b"1e400", &Limits::default()),
        R::CODEC_ERROR_REASON_CANONICAL_JSON_NUMBER_OUT_OF_RANGE
    );
}

#[test]
fn every_resource_budget_has_a_closed_failure() {
    assert_eq!(
        reason(
            b"null",
            &Limits {
                input_bytes: Some(3),
                ..Default::default()
            }
        ),
        R::CODEC_ERROR_REASON_CANONICAL_JSON_INPUT_LIMIT
    );
    assert!(decode(
        b"[]",
        Some(&Limits {
            nesting_depth: Some(1),
            ..Default::default()
        })
    )
    .is_ok());
    assert_eq!(
        reason(
            b"[[]]",
            &Limits {
                nesting_depth: Some(1),
                ..Default::default()
            }
        ),
        R::CODEC_ERROR_REASON_CANONICAL_JSON_DEPTH_LIMIT
    );
    assert_eq!(
        reason(
            b"[0,1]",
            &Limits {
                container_entries: Some(1),
                ..Default::default()
            }
        ),
        R::CODEC_ERROR_REASON_CANONICAL_JSON_ENTRY_LIMIT
    );
    assert_eq!(
        reason(
            br#"{"a":0,"b":1}"#,
            &Limits {
                container_entries: Some(1),
                ..Default::default()
            }
        ),
        R::CODEC_ERROR_REASON_CANONICAL_JSON_ENTRY_LIMIT
    );
    assert_eq!(
        reason(
            b"[null]",
            &Limits {
                nodes: Some(1),
                ..Default::default()
            }
        ),
        R::CODEC_ERROR_REASON_CANONICAL_JSON_NODE_LIMIT
    );
    let emoji = br#""\ud83d\ude00""#;
    assert!(decode(
        emoji,
        Some(&Limits {
            string_bytes: Some(4),
            ..Default::default()
        })
    )
    .is_ok());
    assert_eq!(
        reason(
            emoji,
            &Limits {
                string_bytes: Some(3),
                ..Default::default()
            }
        ),
        R::CODEC_ERROR_REASON_CANONICAL_JSON_STRING_LIMIT
    );
    assert_eq!(
        reason(
            br#"{"ab":"cd"}"#,
            &Limits {
                total_string_bytes: Some(3),
                ..Default::default()
            }
        ),
        R::CODEC_ERROR_REASON_CANONICAL_JSON_TOTAL_STRING_LIMIT
    );
    assert!(decode(
        br#""abc""#,
        Some(&Limits {
            work_bytes: Some(10),
            ..Default::default()
        })
    )
    .is_ok());
    assert_eq!(
        reason(
            br#""abc""#,
            &Limits {
                work_bytes: Some(9),
                ..Default::default()
            }
        ),
        R::CODEC_ERROR_REASON_CANONICAL_JSON_WORK_LIMIT
    );
    for limits in [
        Limits {
            input_bytes: Some(0),
            ..Default::default()
        },
        Limits {
            nesting_depth: Some(65),
            ..Default::default()
        },
        Limits {
            container_entries: Some(65_537),
            ..Default::default()
        },
        Limits {
            nodes: Some(1_048_577),
            ..Default::default()
        },
        Limits {
            string_bytes: Some(16_777_217),
            ..Default::default()
        },
        Limits {
            total_string_bytes: Some(16_777_217),
            ..Default::default()
        },
        Limits {
            work_bytes: Some(1_073_741_825),
            ..Default::default()
        },
    ] {
        assert_eq!(
            reason(b"null", &limits),
            R::CODEC_ERROR_REASON_CANONICAL_JSON_INVALID_LIMITS
        );
    }
}

#[test]
fn common_prefix_keys_consume_the_validation_work_budget() {
    let entries: BTreeMap<String, u8> = (0..100)
        .map(|index| (format!("{}-{index}", "a".repeat(64)), 1))
        .collect();
    let encoded = serde_json::to_vec(&entries).unwrap();
    assert!(decode(&encoded, None).is_ok());
    let work = u64::try_from(encoded.len().checked_mul(3).unwrap()).unwrap();
    assert_eq!(
        reason(
            &encoded,
            &Limits {
                work_bytes: Some(work),
                ..Default::default()
            }
        ),
        R::CODEC_ERROR_REASON_CANONICAL_JSON_WORK_LIMIT
    );
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Shape {
    Unit,
    New(u64),
    Pair(String, bool),
    Object { value: i64 },
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Record {
    label: String,
    optional: Option<String>,
    variants: Vec<Shape>,
    numbers: BTreeMap<u64, i64>,
}

#[test]
fn typed_projection_preserves_serde_shapes_and_container_exhaustion() {
    let value = Record {
        label: "unicode 😀".into(),
        optional: None,
        variants: vec![
            Shape::Unit,
            Shape::New(42),
            Shape::Pair("pair".into(), true),
            Shape::Object { value: -1 },
        ],
        numbers: [(42, -2)].into_iter().collect(),
    };
    let json = serde_json::to_vec(&value).unwrap();
    assert_eq!(from_slice::<Record>(&json, None).unwrap(), value);
    assert!(from_slice::<(u8,)>(b"[1,2]", None).is_err());
    assert!(from_slice::<(u8, u8)>(b"[1]", None).is_err());
    assert!(from_slice::<BTreeMap<u64, u8>>(br#"{"01":1}"#, None).is_err());
    assert!(from_slice::<BTreeMap<u64, u8>>(br#"{"+1":1}"#, None).is_err());
    assert!(from_slice::<BTreeMap<u64, u8>>(br#"{"1 ":1}"#, None).is_err());
}

#[test]
fn integer_and_float_boundaries_match_typed_serde() {
    for text in [
        "0",
        "-0",
        "1",
        "-1",
        "1.0",
        "1e3",
        "18446744073709551615",
        "18446744073709551616",
        "-9223372036854775808",
        "-9223372036854775809",
        "340282366920938463463374607431768211455",
        "1e-400",
        "1e39",
        "1.000000059604644775390625",
    ] {
        macro_rules! same {
            ($type:ty) => {
                assert_eq!(
                    from_slice::<$type>(text.as_bytes(), None).ok(),
                    serde_json::from_str::<$type>(text).ok(),
                    "{} {text}",
                    stringify!($type)
                );
            };
        }
        same!(i64);
        same!(u64);
        same!(i128);
        same!(u128);
        same!(f64);
        same!(f32);
    }
}

fn json_values() -> impl Strategy<Value = serde_json::Value> {
    let leaf = prop_oneof![
        Just(serde_json::Value::Null),
        any::<bool>().prop_map(serde_json::Value::Bool),
        any::<i64>().prop_map(|v| serde_json::Value::Number(v.into())),
        any::<u64>().prop_map(|v| serde_json::Value::Number(v.into())),
        ".{0,64}".prop_map(serde_json::Value::String)
    ];
    leaf.prop_recursive(5, 128, 8, |inner| {
        prop_oneof![
            collection::vec(inner.clone(), 0..8).prop_map(serde_json::Value::Array),
            collection::btree_map(".{0,24}", inner, 0..8)
                .prop_map(|entries| serde_json::Value::Object(entries.into_iter().collect()))
        ]
    })
}
proptest! {
    #[test]
    fn document_projection_matches_serde(value in json_values()) {
        let encoded = serde_json::to_vec(&value).unwrap();
        let decoded: serde_json::Value = from_slice(&encoded, None).unwrap();
        prop_assert_eq!(decoded, value);
    }
    #[test]
    fn float_projection_matches_roundtrip_parser(value in any::<f64>().prop_filter("finite", |v| v.is_finite())) {
        let encoded = serde_json::to_vec(&value).unwrap();
        let decoded: f64 = from_slice(&encoded, None).unwrap();
        let standard: f64 = serde_json::from_slice(&encoded).unwrap();
        prop_assert_eq!(decoded.to_bits(), standard.to_bits());
    }
    #[test]
    fn float32_projection_matches_roundtrip_parser(value in any::<f32>().prop_filter("finite", |v| v.is_finite())) {
        let encoded = serde_json::to_vec(&value).unwrap();
        let decoded: f32 = from_slice(&encoded, None).unwrap();
        let standard: f32 = serde_json::from_slice(&encoded).unwrap();
        prop_assert_eq!(decoded.to_bits(), standard.to_bits());
    }
    #[test]
    fn arbitrary_bytes_never_panic(input in collection::vec(any::<u8>(), 0..1024)) {
        let _result = decode(&input, None);
    }
}
