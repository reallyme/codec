// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(deprecated)]

use super::{dag_cbor_vector_value, decode_hex, encode_hex, Vectors};
#[cfg(feature = "operation-contract")]
use reallyme_codec::operation_contract::{
    process_operation_response, process_operation_response_json,
};
use reallyme_codec::{
    base64::{base64_to_bytes, bytes_to_base64, Base64Error},
    base64url::{base64url_to_bytes, bytes_to_base64url, Base64UrlError},
    cbor::{
        compute_cid_dag_cbor, dag_cbor_multihash, decode_dag_cbor, encode_dag_cbor,
        is_valid_cid_string, sha2_256_content_hash, try_parse_cid, verify_dag_cbor_cid, CborError,
        DAG_CBOR_CODEC,
    },
    hex::{bytes_to_lower_hex, lower_hex_to_bytes},
    jcs::{canonicalize_json_text, JcsError},
    multibase::{
        base58btc_decode, base58btc_encode, bytes_to_multibase58btc, bytes_to_multibase_base64url,
        multibase_to_bytes, MultibaseError,
    },
    multicodec::{lookup_prefix, strip_prefix, supported_table, MulticodecLength},
    multikey::{
        binding_type_matches_codec, encode_multikey, parse_multikey, validate_key_binding,
        KeyBindingInput, MultikeyError,
    },
    pem::{decode_pem, encode_pem, PemDecodePolicy, PemEncodeOptions, PemLabel},
};

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
        bytes_to_multibase_base64url(&public_key).unwrap(),
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

    let multicodec = lookup_prefix(&prefixed_public_key).expect("ed25519 prefix resolves");
    assert_eq!(multicodec.name(), vectors.string("ed25519CodecName"));
    assert_eq!(
        multicodec.metadata().algorithm_name(),
        vectors.string("ed25519AlgorithmName")
    );
    assert_eq!(
        encode_hex(multicodec.metadata().prefix()),
        vectors.string("ed25519PrefixHex")
    );
    assert_eq!(
        multicodec.metadata().length(),
        MulticodecLength::Fixed(usize::try_from(vectors.u64("ed25519ExpectedKeyLength")).unwrap())
    );
    assert_eq!(
        strip_prefix(&prefixed_public_key).expect("known prefix strips"),
        public_key.as_slice()
    );
    let table = supported_table().expect("multicodec table is valid");
    assert!(table
        .entries()
        .iter()
        .any(|entry| entry.name() == vectors.string("multicodecTableRequiredName")));

    let multikey = encode_multikey(vectors.string("ed25519CodecName"), &public_key).unwrap();
    assert_eq!(multikey, vectors.string("ed25519Multikey"));
    let parsed = parse_multikey(vectors.string("ed25519Multikey")).unwrap();
    assert!(parse_multikey(vectors.string("ed25519PrivateMultikey")).is_err());
    assert_eq!(parsed.codec_name(), vectors.string("ed25519CodecName"));
    assert_eq!(
        parsed.algorithm_name(),
        vectors.string("ed25519AlgorithmName")
    );
    assert_eq!(parsed.public_key(), public_key);
    assert_eq!(
        parsed.key_length(),
        codec_multicodec::KeyLength::Fixed(
            usize::try_from(vectors.u64("ed25519ExpectedKeyLength")).unwrap()
        )
    );
    assert!(binding_type_matches_codec(
        vectors.string("multikeyBindingType"),
        parsed.codec_name()
    ));
    validate_key_binding(
        KeyBindingInput {
            binding_type: vectors.string("multikeyBindingType"),
            algorithm: None,
        },
        &parsed,
    )
    .unwrap();
    let p256 = parse_multikey(vectors.string("p256Multikey")).unwrap();
    assert!(matches!(
        validate_key_binding(
            KeyBindingInput {
                binding_type: "P256Key2024",
                algorithm: None,
            },
            &p256,
        ),
        Err(MultikeyError::BindingAlgorithmMissing { .. })
    ));
    assert!(matches!(
        validate_key_binding(
            KeyBindingInput {
                binding_type: vectors.string("mismatchedBindingType"),
                algorithm: Some(vectors.string("mismatchedBindingAlgorithm")),
            },
            &parsed
        ),
        Err(MultikeyError::BindingTypeCodecMismatch { .. })
    ));
    assert!(matches!(
        validate_key_binding(
            KeyBindingInput {
                binding_type: vectors.string("multikeyBindingType"),
                algorithm: Some(vectors.string("mismatchedBindingAlgorithm")),
            },
            &parsed
        ),
        Err(MultikeyError::BindingAlgorithmMismatch { .. })
    ));

    let cbor_value = dag_cbor_vector_value();
    let encoded = encode_dag_cbor(&cbor_value).unwrap();
    assert_eq!(encode_hex(&encoded), vectors.string("dagCborEncodedHex"));
    assert_eq!(
        encode_hex(&encode_dag_cbor(&decode_dag_cbor(&encoded).unwrap()).unwrap()),
        vectors.string("dagCborEncodedHex")
    );
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
    let verification = verify_dag_cbor_cid(vectors.string("dagCborCid"), &encoded).unwrap();
    assert_eq!(
        verification.status(),
        codec_cbor::CidVerificationStatus::Match
    );
    assert_eq!(verification.expected_cid(), vectors.string("dagCborCid"));
    assert_eq!(verification.actual_cid(), vectors.string("dagCborCid"));

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
    let decoded = decode_pem(
        vectors.string("pemPrivatePem"),
        PemDecodePolicy {
            allowed_labels: &[PemLabel::PrivateKey],
            ..PemDecodePolicy::default()
        },
    )
    .unwrap();
    assert_eq!(decoded.label, PemLabel::PrivateKey);
    assert_eq!(decoded.der.as_slice(), private_der.as_slice());

    #[cfg(feature = "operation-contract")]
    {
        let proto_envelope = process_operation_response(&decode_hex(
            vectors.string("protoMulticodecTableRequestHex"),
        ));
        let proto_json_envelope = process_operation_response_json(
            vectors.string("protoMulticodecTableRequestJson").as_bytes(),
        );
        assert!(!proto_envelope.is_empty());
        assert_eq!(proto_envelope.as_slice(), proto_json_envelope.as_slice());
    }
}

#[test]
fn shared_vector_suite_rejects_non_canonical_inputs() {
    let vectors = Vectors::load();

    for key in [
        "base64MissingPadding",
        "base64NonCanonicalTrailingBits",
        "base64Whitespace",
    ] {
        assert!(
            matches!(
                base64_to_bytes(vectors.string(key)),
                Err(Base64Error::Invalid)
            ),
            "{key}"
        );
    }
    for key in [
        "base64urlPadded",
        "base64urlNonCanonicalTrailingBits",
        "base64urlInvalidLength",
        "base64urlWhitespace",
    ] {
        assert!(
            matches!(
                base64url_to_bytes(vectors.string(key)),
                Err(Base64UrlError::Invalid)
            ),
            "{key}"
        );
    }
    for key in ["unsupportedMultibase", "multibaseMultibytePrefix"] {
        assert!(
            matches!(
                multibase_to_bytes(vectors.string(key)),
                Err(MultibaseError::UnsupportedPrefix)
            ),
            "{key}"
        );
    }
    assert!(matches!(
        parse_multikey(vectors.string("nonCanonicalBase64urlMultikey")),
        Err(MultikeyError::InvalidMultibase)
    ));
    assert_eq!(
        decode_dag_cbor(&decode_hex(vectors.string("dagCborNonCanonicalIntegerHex"))),
        Err(CborError::NonCanonicalInteger)
    );
    assert_eq!(
        decode_dag_cbor(&decode_hex(vectors.string("dagCborDuplicateKeyHex"))),
        Err(CborError::DuplicateMapKey)
    );
    assert_eq!(
        decode_dag_cbor(&decode_hex(vectors.string("dagCborOutOfOrderKeyHex"))),
        Err(CborError::MapKeysOutOfOrder)
    );
    assert_eq!(
        canonicalize_json_text(vectors.string("jcsDuplicateMemberJson")),
        Err(JcsError::DuplicateProperty)
    );
    assert_eq!(
        canonicalize_json_text(vectors.string("jcsNonInteroperableIntegerJson")),
        Err(JcsError::IntegerOutsideInteroperableRange)
    );
    assert_eq!(
        canonicalize_json_text(vectors.string("jcsLoneSurrogateJson")),
        Err(JcsError::InvalidJson)
    );
    assert_eq!(
        canonicalize_json_text(vectors.string("jcsUtf16KeyOrderInputJson")).unwrap(),
        vectors.string("jcsUtf16KeyOrderCanonicalJson")
    );
}
