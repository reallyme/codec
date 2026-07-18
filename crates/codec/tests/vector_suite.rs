// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use reallyme_codec::{
    base64::base64_to_bytes,
    cbor::{
        decode_deterministic_cbor, encode_deterministic_cbor, sha2_256_content_hash, CborValue,
        DeterministicCborError, DeterministicCborInteger, DeterministicCborMapEntry,
        DeterministicCborMapKey, DeterministicCborValue,
    },
    hex::{bytes_to_lower_hex, lower_hex_to_bytes},
};
use serde_json::Value;

const VECTORS_JSON: &str = include_str!("../../../vectors/codec-vectors.json");

#[path = "vector_suite/core_methods.rs"]
mod core_methods;

#[derive(Debug)]
struct Vectors {
    value: Value,
}

impl Vectors {
    fn load() -> Self {
        let value: Value =
            serde_json::from_str(VECTORS_JSON).expect("codec vector manifest is valid JSON");
        assert_eq!(value["schemaVersion"].as_u64(), Some(2));
        Self { value }
    }

    fn string(&self, key: &str) -> &str {
        self.value["vectors"][key]
            .as_str()
            .unwrap_or_else(|| panic!("missing string vector key {key}"))
    }

    fn u64(&self, key: &str) -> u64 {
        self.value["vectors"][key]
            .as_u64()
            .unwrap_or_else(|| panic!("missing integer vector key {key}"))
    }

    fn deterministic_cbor_array(&self, key: &str) -> &[Value] {
        self.value["deterministicCbor"][key]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_else(|| panic!("missing deterministic CBOR vector array {key}"))
    }
}

fn decode_hex(value: &str) -> Vec<u8> {
    lower_hex_to_bytes(value).expect("hex vector decodes")
}

fn encode_hex(value: &[u8]) -> String {
    bytes_to_lower_hex(value)
}

fn find_named_vector<'a>(vectors: &'a [Value], name: &str) -> &'a Value {
    vectors
        .iter()
        .find(|vector| vector["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("missing named vector {name}"))
}

fn find_named_hex<'a>(vectors: &'a [Value], name: &str) -> &'a str {
    find_named_vector(vectors, name)["hex"]
        .as_str()
        .unwrap_or_else(|| panic!("missing named hex vector {name}"))
}

fn dag_cbor_vector_value() -> CborValue {
    CborValue::Map(vec![
        ("b".to_owned(), CborValue::Int(2)),
        ("a".to_owned(), CborValue::String("one".to_owned())),
        ("bytes".to_owned(), CborValue::Bytes(vec![0, 1, 2])),
    ])
}

fn vector_integer(value: &Value) -> DeterministicCborInteger {
    if let Some(unsigned) = value["unsigned"].as_str() {
        return DeterministicCborInteger::unsigned(
            unsigned
                .parse::<u64>()
                .expect("unsigned integer vector parses"),
        );
    }
    let negative = value["negative"]
        .as_str()
        .expect("deterministic integer vector has signed variant")
        .parse::<i64>()
        .expect("negative integer vector parses");
    DeterministicCborInteger::negative(negative).expect("negative vector is valid")
}

fn vector_map_key(value: &Value) -> DeterministicCborMapKey {
    if let Some(text) = value["text"].as_str() {
        return DeterministicCborMapKey::text(text.to_owned());
    }
    DeterministicCborMapKey::Integer(vector_integer(&value["integer"]))
}

fn vector_deterministic_value(value: &Value) -> DeterministicCborValue {
    if value.get("null").and_then(Value::as_bool) == Some(true) {
        return DeterministicCborValue::Null;
    }
    if let Some(boolean) = value["bool"].as_bool() {
        return DeterministicCborValue::Bool(boolean);
    }
    if value.get("unsigned").is_some() || value.get("negative").is_some() {
        return DeterministicCborValue::Integer(vector_integer(value));
    }
    if let Some(text) = value["text"].as_str() {
        return DeterministicCborValue::Text(text.to_owned());
    }
    if let Some(bytes) = value["bytes"].as_str() {
        return DeterministicCborValue::Bytes(base64_to_bytes(bytes).unwrap());
    }
    if let Some(array) = value["array"].as_array() {
        return DeterministicCborValue::Array(
            array.iter().map(vector_deterministic_value).collect(),
        );
    }
    vector_deterministic_map_entries(&value["map"])
}

fn vector_deterministic_map_entries(value: &Value) -> DeterministicCborValue {
    let entries = value
        .as_array()
        .expect("deterministic map fixture has an entry array");
    DeterministicCborValue::Map(
        entries
            .iter()
            .map(|entry| {
                DeterministicCborMapEntry::new(
                    vector_map_key(&entry["key"]),
                    vector_deterministic_value(&entry["value"]),
                )
            })
            .collect(),
    )
}

fn resource_count(recipe: &Value, field: &str) -> usize {
    usize::try_from(
        recipe["construction"][field]
            .as_u64()
            .unwrap_or_else(|| panic!("resource recipe is missing integer field {field}")),
    )
    .expect("resource recipe count fits usize")
}

fn balanced_array_tree(branching: usize, levels: usize) -> DeterministicCborValue {
    if levels == 0 {
        return DeterministicCborValue::Null;
    }
    DeterministicCborValue::Array(
        (0..branching)
            .map(|_| balanced_array_tree(branching, levels - 1))
            .collect(),
    )
}

#[test]
fn shared_vectors_include_deterministic_cbor_literals() {
    let vectors = Vectors::load();
    assert_eq!(
        vectors.value["deterministicCbor"]["profile"].as_str(),
        Some("rfc8949-core-deterministic-reallyme-0.2.0")
    );
    let golden_vectors = vectors.deterministic_cbor_array("positive");

    for vector in golden_vectors {
        let hex = vector["hex"].as_str().expect("deterministic CBOR hex");
        let bytes = decode_hex(hex);
        assert!(!bytes.is_empty());
        assert!(
            vector.get("value").is_some(),
            "positive deterministic CBOR vector must declare an independent semantic value"
        );
    }

    assert_eq!(
        find_named_hex(golden_vectors, "unsigned-2pow53-minus-1"),
        "1b001fffffffffffff"
    );
    assert_eq!(
        find_named_hex(golden_vectors, "unsigned-2pow53"),
        "1b0020000000000000"
    );
    assert_eq!(
        find_named_hex(golden_vectors, "unsigned-i64-max"),
        "1b7fffffffffffffff"
    );
    assert_eq!(
        find_named_hex(golden_vectors, "unsigned-u64-max"),
        "1bffffffffffffffff"
    );
    assert_eq!(
        find_named_hex(golden_vectors, "negative-i64-min"),
        "3b7fffffffffffffff"
    );
    assert_eq!(
        find_named_hex(golden_vectors, "mixed-integer-text-key-map"),
        "a2016b696e74656765722d6b6579616168746578742d6b6579"
    );

    assert_eq!(
        find_named_hex(golden_vectors, "text-multibyte-u-umlaut"),
        "62c3bc"
    );
    assert_eq!(find_named_hex(golden_vectors, "nested-array"), "8201820203");
    assert_eq!(
        find_named_hex(golden_vectors, "text-key-map"),
        "a2616101616202"
    );

    let rejection_vectors = vectors.deterministic_cbor_array("negative");
    for vector in rejection_vectors {
        assert!(vector["name"].as_str().is_some());
        assert!(vector["reason"].as_str().is_some());
        assert!(!decode_hex(vector["hex"].as_str().expect("rejection hex")).is_empty());
    }
    assert_eq!(
        find_named_hex(rejection_vectors, "duplicate-integer-key"),
        "a201010102"
    );
    assert_eq!(
        find_named_hex(rejection_vectors, "non-minimal-unsigned-zero"),
        "1800"
    );
    assert_eq!(
        find_named_hex(rejection_vectors, "non-minimal-negative-minus-one"),
        "3800"
    );

    let equivalent_inputs = vectors.deterministic_cbor_array("equivalentInputOrders");
    let fixture_classes = vectors.value["deterministicCbor"]["fixtureClasses"]
        .as_object()
        .expect("deterministic CBOR fixture classes");
    assert_eq!(fixture_classes["positive"].as_str(), Some("golden"));
    assert_eq!(
        fixture_classes["negative"].as_str(),
        Some("rejection-fixture")
    );
    assert_eq!(
        fixture_classes["equivalentInputOrders"].as_str(),
        Some("golden")
    );
    assert_eq!(
        fixture_classes["resourceRejections"].as_str(),
        Some("construction-recipe")
    );
    assert_eq!(
        fixture_classes["interoperability"].as_str(),
        Some("interop-fixture")
    );
    let equivalent = find_named_vector(equivalent_inputs, "mixed-key-map-order-independent");
    let input_orders = equivalent["inputs"]
        .as_array()
        .expect("equivalent deterministic CBOR input orders");
    assert_eq!(input_orders.len(), 2);
    assert_ne!(input_orders[0], input_orders[1]);
    assert_eq!(equivalent["hex"].as_str(), Some("a301616961316174616202"));

    let resource_rejections = vectors.deterministic_cbor_array("resourceRejections");
    assert_eq!(resource_rejections.len(), 5);
    for vector in resource_rejections {
        assert!(vector["name"].as_str().is_some());
        assert!(vector["construction"]["kind"].as_str().is_some());
        assert!(vector["reason"].as_str().is_some());
    }
    let input_limit = find_named_vector(resource_rejections, "input-byte-limit-plus-one");
    assert_eq!(
        input_limit["construction"]["count"].as_u64(),
        Some(1_048_577)
    );
    assert_eq!(
        input_limit["construction"]["fillByteHex"].as_str(),
        Some("00")
    );
    let output_limit = find_named_vector(resource_rejections, "output-byte-limit-plus-header");
    assert_eq!(
        output_limit["construction"]["count"].as_u64(),
        Some(1_048_576)
    );
    let node_limit = find_named_vector(resource_rejections, "node-limit-exceeded-balanced-tree");
    assert_eq!(node_limit["construction"]["branching"].as_u64(), Some(256));
    assert_eq!(node_limit["construction"]["levels"].as_u64(), Some(2));
    let container_limit = find_named_vector(resource_rejections, "container-entry-limit-plus-one");
    assert_eq!(
        container_limit["construction"]["count"].as_u64(),
        Some(16_385)
    );
    let depth_limit = find_named_vector(resource_rejections, "nesting-depth-limit-plus-one");
    assert_eq!(depth_limit["construction"]["depth"].as_u64(), Some(65));

    let interoperability = vectors.deterministic_cbor_array("interoperability");
    assert!(
        find_named_vector(interoperability, "idkit-ios-synthetic-passport-claims-v1").is_object()
    );
    assert!(find_named_vector(
        interoperability,
        "idkit-ios-synthetic-passport-claims-null-place-of-birth-v1"
    )
    .is_object());
    assert!(
        find_named_vector(interoperability, "idkit-ios-synthetic-fingerprint-map-v1").is_object()
    );
    assert!(find_named_vector(
        interoperability,
        "idkit-ios-synthetic-mixed-integer-claim-tags-v1"
    )
    .is_object());
    for fixture in interoperability {
        assert_eq!(fixture["fixtureKind"].as_str(), Some("synthetic"));
        assert_eq!(fixture["sourceRepo"].as_str(), Some("reallyme/idkit-ios"));
        assert_eq!(
            fixture["sourceCommit"].as_str(),
            Some("content-hash-pinned")
        );
        assert!(fixture["source"].as_str().is_some());
        assert!(fixture["explanation"].as_str().is_some());
        let source_files = fixture["sourceFiles"]
            .as_array()
            .expect("interoperability fixture pins source files");
        assert!(!source_files.is_empty());
        for source_file in source_files {
            assert!(source_file["path"].as_str().is_some());
            assert_eq!(
                source_file["sha256"].as_str().map(str::len),
                Some(64),
                "source file digest must be a lowercase SHA-256 hex string"
            );
            assert!(source_file["sha256"]
                .as_str()
                .expect("source file digest")
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
        }
        let fixture_bytes = decode_hex(
            fixture["hex"]
                .as_str()
                .expect("interoperability fixture hex"),
        );
        assert_eq!(
            u64::try_from(fixture_bytes.len()).expect("fixture length fits u64"),
            fixture["byteLength"]
                .as_u64()
                .expect("interoperability fixture byte length")
        );
        assert_eq!(
            encode_hex(&sha2_256_content_hash(&fixture_bytes)),
            fixture["sha256"]
                .as_str()
                .expect("interoperability fixture digest")
        );
    }
}

#[test]
fn deterministic_cbor_semantics_consume_literal_vectors() {
    let vectors = Vectors::load();
    for vector in vectors.deterministic_cbor_array("positive") {
        let bytes = decode_hex(vector["hex"].as_str().expect("positive vector hex"));
        let declared_value = vector_deterministic_value(&vector["value"]);
        assert_eq!(
            encode_deterministic_cbor(&declared_value)
                .expect("declared positive vector value encodes")
                .as_slice(),
            bytes.as_slice()
        );
        let decoded = decode_deterministic_cbor(&bytes).expect("positive vector decodes");
        assert_eq!(
            encode_deterministic_cbor(&decoded).unwrap().as_slice(),
            bytes.as_slice()
        );
    }

    for vector in vectors.deterministic_cbor_array("negative") {
        let bytes = decode_hex(vector["hex"].as_str().expect("negative vector hex"));
        assert!(decode_deterministic_cbor(&bytes).is_err());
    }

    for vector in vectors.deterministic_cbor_array("equivalentInputOrders") {
        let expected = decode_hex(vector["hex"].as_str().expect("equivalent vector hex"));
        let inputs = vector["inputs"]
            .as_array()
            .expect("equivalent vector inputs");
        for input in inputs {
            let value = vector_deterministic_map_entries(input);
            assert_eq!(
                encode_deterministic_cbor(&value).unwrap().as_slice(),
                expected.as_slice()
            );
        }
    }
}

#[test]
fn deterministic_cbor_semantics_execute_shared_resource_recipes() {
    let vectors = Vectors::load();
    for recipe in vectors.deterministic_cbor_array("resourceRejections") {
        let kind = recipe["construction"]["kind"]
            .as_str()
            .expect("resource recipe has a construction kind");
        let expected_reason = recipe["reason"]
            .as_str()
            .expect("resource recipe has a reason");
        let actual = match kind {
            "encoded-byte-count" => {
                let fill = decode_hex(
                    recipe["construction"]["fillByteHex"]
                        .as_str()
                        .expect("encoded-byte-count recipe has a fill byte"),
                );
                assert_eq!(fill.len(), 1);
                decode_deterministic_cbor(&vec![fill[0]; resource_count(recipe, "count")])
                    .expect_err("resource recipe must be rejected")
            }
            "byte-string-length" => {
                encode_deterministic_cbor(&DeterministicCborValue::Bytes(vec![
                    0;
                    resource_count(
                        recipe, "count"
                    )
                ]))
                .expect_err("resource recipe must be rejected")
            }
            "balanced-array-tree" => encode_deterministic_cbor(&balanced_array_tree(
                resource_count(recipe, "branching"),
                resource_count(recipe, "levels"),
            ))
            .expect_err("resource recipe must be rejected"),
            "array-of-null" => encode_deterministic_cbor(&DeterministicCborValue::Array(
                (0..resource_count(recipe, "count"))
                    .map(|_| DeterministicCborValue::Null)
                    .collect(),
            ))
            .expect_err("resource recipe must be rejected"),
            "nested-singleton-arrays" => {
                let mut value = DeterministicCborValue::Null;
                for _ in 0..resource_count(recipe, "depth") {
                    value = DeterministicCborValue::Array(vec![value]);
                }
                encode_deterministic_cbor(&value).expect_err("resource recipe must be rejected")
            }
            other => panic!("unsupported deterministic-CBOR resource recipe {other}"),
        };
        let actual_reason = match actual {
            DeterministicCborError::InputTooLarge => "input-too-large",
            DeterministicCborError::OutputTooLarge => "output-too-large",
            DeterministicCborError::NodeLimitExceeded => "node-limit-exceeded",
            DeterministicCborError::ContainerEntriesExceeded => "container-entries-exceeded",
            DeterministicCborError::DepthExceeded => "depth-exceeded",
            other => panic!("unexpected resource recipe error {other:?}"),
        };
        assert_eq!(actual_reason, expected_reason);
    }
}
