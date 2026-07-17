// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use reallyme_codec::{
    base64::{base64_to_bytes, bytes_to_base64},
    base64url::{base64url_to_bytes, bytes_to_base64url},
    cbor::{
        compute_cid_dag_cbor, dag_cbor_multihash, decode_dag_cbor, encode_dag_cbor,
        is_valid_cid_string, sha2_256_content_hash, try_parse_cid, verify_dag_cbor_cid, CborValue,
        DAG_CBOR_CODEC,
    },
    hex::{bytes_to_lower_hex, lower_hex_to_bytes},
    jcs::canonicalize_json_text,
    multibase::{
        base58btc_decode, base58btc_encode, bytes_to_multibase58btc, bytes_to_multibase_base64url,
        multibase_to_bytes,
    },
    multicodec::{lookup_codec_prefix, strip_codec_prefix, MULTICODEC_TABLE},
    multikey::{
        binding_type_matches_codec, encode_multikey, parse_multikey, validate_key_binding,
        KeyBindingInput,
    },
    pem::{decode_pem, encode_pem, PemDecodePolicy, PemEncodeOptions, PemLabel},
    proto_process::{process_proto, process_proto_json},
};
use serde_json::Value;

const VECTORS_JSON: &str = include_str!("../../../test-vectors/codec-vectors.json");

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
}

fn decode_hex(value: &str) -> Vec<u8> {
    lower_hex_to_bytes(value).expect("hex vector decodes")
}

fn encode_hex(value: &[u8]) -> String {
    bytes_to_lower_hex(value)
}

fn tagged_json_to_cbor(value: &Value) -> CborValue {
    match value["type"].as_str().expect("tagged value has type") {
        "null" => CborValue::Null,
        "bool" => CborValue::Bool(value["value"].as_bool().expect("bool value")),
        "int" => CborValue::Int(value["value"].as_i64().expect("int value")),
        "string" => CborValue::String(value["value"].as_str().expect("string value").to_owned()),
        "bytes" => CborValue::Bytes(
            base64_to_bytes(value["value"].as_str().expect("bytes value")).unwrap(),
        ),
        "array" => CborValue::Array(
            value["value"]
                .as_array()
                .expect("array value")
                .iter()
                .map(tagged_json_to_cbor)
                .collect(),
        ),
        "map" => CborValue::Map(
            value["value"]
                .as_array()
                .expect("map entries")
                .iter()
                .map(|entry| {
                    (
                        entry["key"].as_str().expect("map key").to_owned(),
                        tagged_json_to_cbor(&entry["value"]),
                    )
                })
                .collect(),
        ),
        other => panic!("unsupported tagged CBOR type {other}"),
    }
}

#[test]
fn shared_vector_suite_covers_core_codec_methods() {
    let vectors = Vectors::load();
    let base_input = decode_hex(vectors.string("baseInputHex"));

    assert_eq!(bytes_to_base64(&base_input), vectors.string("base64Padded"));
    assert_eq!(
        base64_to_bytes(vectors.string("base64Padded")).unwrap(),
        base_input
    );
    assert_eq!(
        bytes_to_base64url(&base_input),
        vectors.string("base64urlUnpadded")
    );
    assert_eq!(
        base64url_to_bytes(vectors.string("base64urlUnpadded")).unwrap(),
        base_input
    );
    assert_eq!(bytes_to_lower_hex(&base_input), vectors.string("lowerHex"));
    assert_eq!(
        lower_hex_to_bytes(vectors.string("lowerHex")).unwrap(),
        base_input
    );
    assert_eq!(
        base58btc_encode(&base_input).unwrap(),
        vectors.string("base58btcEncoded")
    );
    assert_eq!(
        base58btc_decode(vectors.string("base58btcEncoded")).unwrap(),
        base_input
    );

    let public_key = decode_hex(vectors.string("publicKeyHex"));
    let prefixed_public_key = decode_hex(vectors.string("ed25519PrefixedPublicKeyHex"));
    assert_eq!(
        base58btc_encode(&public_key).unwrap(),
        vectors.string("publicKeyBase58btc")
    );
    assert_eq!(
        bytes_to_multibase58btc(&public_key).unwrap(),
        vectors.string("publicKeyMultibaseBase58btc")
    );
    assert_eq!(
        bytes_to_multibase_base64url(&public_key),
        vectors.string("publicKeyMultibaseBase64url")
    );
    assert_eq!(
        multibase_to_bytes(vectors.string("publicKeyMultibaseBase58btc")).unwrap(),
        public_key
    );
    assert_eq!(
        multibase_to_bytes(vectors.string("publicKeyMultibaseBase64url")).unwrap(),
        public_key
    );

    let multicodec = lookup_codec_prefix(&prefixed_public_key).expect("ed25519 prefix resolves");
    assert_eq!(multicodec.name, vectors.string("ed25519CodecName"));
    assert_eq!(multicodec.alg, vectors.string("ed25519AlgorithmName"));
    assert_eq!(
        encode_hex(multicodec.codec),
        vectors.string("ed25519PrefixHex")
    );
    assert_eq!(
        multicodec.key_length,
        usize::try_from(vectors.u64("ed25519ExpectedKeyLength")).unwrap()
    );
    assert_eq!(
        strip_codec_prefix(&prefixed_public_key),
        public_key.as_slice()
    );
    assert!(MULTICODEC_TABLE
        .iter()
        .any(|(name, _)| *name == vectors.string("multicodecTableRequiredName")));

    let multikey = encode_multikey(vectors.string("ed25519CodecName"), &public_key).unwrap();
    assert_eq!(multikey, vectors.string("ed25519Multikey"));
    let parsed = parse_multikey(vectors.string("ed25519Multikey")).unwrap();
    assert_eq!(parsed.codec_name, vectors.string("ed25519CodecName"));
    assert_eq!(parsed.alg, vectors.string("ed25519AlgorithmName"));
    assert_eq!(parsed.public_key, public_key);
    assert_eq!(
        parsed.key_length,
        usize::try_from(vectors.u64("ed25519ExpectedKeyLength")).unwrap()
    );
    assert!(binding_type_matches_codec(
        vectors.string("multikeyBindingType"),
        parsed.codec_name
    ));
    validate_key_binding(
        KeyBindingInput {
            binding_type: vectors.string("multikeyBindingType"),
            algorithm: None,
        },
        &parsed,
    )
    .unwrap();
    assert!(validate_key_binding(
        KeyBindingInput {
            binding_type: vectors.string("mismatchedBindingType"),
            algorithm: Some(vectors.string("mismatchedBindingAlgorithm")),
        },
        &parsed
    )
    .is_err());

    let tagged_json: Value =
        serde_json::from_str(vectors.string("dagCborTaggedJson")).expect("tagged DAG-CBOR JSON");
    let canonical_tagged_json: Value =
        serde_json::from_str(vectors.string("dagCborCanonicalTaggedJson"))
            .expect("canonical tagged DAG-CBOR JSON");
    let cbor_value = tagged_json_to_cbor(&tagged_json);
    let canonical_cbor_value = tagged_json_to_cbor(&canonical_tagged_json);
    let encoded = encode_dag_cbor(&cbor_value).unwrap();
    assert_eq!(encode_hex(&encoded), vectors.string("dagCborEncodedHex"));
    assert_eq!(decode_dag_cbor(&encoded).unwrap(), canonical_cbor_value);
    assert_eq!(compute_cid_dag_cbor(&encoded), vectors.string("dagCborCid"));
    assert_eq!(
        encode_hex(&sha2_256_content_hash(&encoded)),
        vectors.string("dagCborSha256Hex")
    );
    assert_eq!(
        encode_hex(&dag_cbor_multihash(&encoded).to_bytes()),
        vectors.string("dagCborMultihashHex")
    );
    assert_eq!(DAG_CBOR_CODEC, vectors.u64("dagCborCodecCode"));
    assert!(is_valid_cid_string(vectors.string("dagCborCid")));
    assert_eq!(
        try_parse_cid(vectors.string("dagCborCid")).map(|cid| cid.to_string()),
        Some(vectors.string("dagCborCid").to_owned())
    );
    assert!(!is_valid_cid_string(vectors.string("invalidCid")));
    assert_eq!(try_parse_cid(vectors.string("invalidCid")), None);
    let (valid, expected, actual) = verify_dag_cbor_cid(vectors.string("dagCborCid"), &encoded);
    assert!(valid);
    assert_eq!(expected, vectors.string("dagCborCid"));
    assert_eq!(actual, vectors.string("dagCborCid"));

    assert_eq!(
        canonicalize_json_text(vectors.string("jcsObjectInputJson")).unwrap(),
        vectors.string("jcsObjectCanonicalJson")
    );
    assert_eq!(
        canonicalize_json_text(vectors.string("jcsNumberInputJson")).unwrap(),
        vectors.string("jcsNumberCanonicalJson")
    );

    let private_der = decode_hex(vectors.string("pemPrivateDerHex"));
    let pem = encode_pem(
        PemLabel::PrivateKey,
        &private_der,
        PemEncodeOptions::default(),
    )
    .unwrap();
    assert_eq!(pem.as_str(), vectors.string("pemPrivatePem"));
    let decoded = decode_pem(vectors.string("pemPrivatePem"), PemDecodePolicy::default()).unwrap();
    assert_eq!(decoded.label, PemLabel::PrivateKey);
    assert_eq!(decoded.der.as_slice(), private_der.as_slice());

    let proto_envelope = process_proto(&decode_hex(
        vectors.string("protoMulticodecTableRequestHex"),
    ));
    let proto_json_envelope =
        process_proto_json(vectors.string("protoMulticodecTableRequestJson").as_bytes());
    assert!(!proto_envelope.is_empty());
    assert_eq!(proto_envelope.as_slice(), proto_json_envelope.as_slice());
}

#[test]
fn shared_vector_suite_rejects_non_canonical_inputs() {
    let vectors = Vectors::load();

    assert!(base64_to_bytes(vectors.string("base64MissingPadding")).is_err());
    assert!(base64_to_bytes(vectors.string("base64NonCanonicalTrailingBits")).is_err());
    assert!(base64url_to_bytes(vectors.string("base64urlPadded")).is_err());
    assert!(base64url_to_bytes(vectors.string("base64urlNonCanonicalTrailingBits")).is_err());
    assert!(multibase_to_bytes(vectors.string("unsupportedMultibase")).is_err());
    assert!(parse_multikey(vectors.string("nonCanonicalBase64urlMultikey")).is_err());
    assert!(decode_dag_cbor(&decode_hex(vectors.string("dagCborNonCanonicalIntegerHex"))).is_err());
    assert!(decode_dag_cbor(&decode_hex(vectors.string("dagCborDuplicateKeyHex"))).is_err());
    assert!(decode_dag_cbor(&decode_hex(vectors.string("dagCborOutOfOrderKeyHex"))).is_err());
    assert!(canonicalize_json_text(vectors.string("jcsDuplicateMemberJson")).is_err());
    assert!(canonicalize_json_text(vectors.string("jcsNonInteroperableIntegerJson")).is_err());
    assert!(canonicalize_json_text(vectors.string("jcsLoneSurrogateJson")).is_err());
}
