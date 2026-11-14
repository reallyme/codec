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
use cid::multibase::Base;
use cid::Cid;
use codec_cbor::CborValue;
use codec_cbor::{
    compute_cid_dag_cbor, dag_cbor_multihash, encode_dag_cbor, is_valid_cid_string,
    sha2_256_content_hash, try_parse_cid, verify_dag_cbor_cid, CidVerificationStatus,
    MAX_CID_STRING_LEN,
};

fn enc(value: &CborValue) -> Vec<u8> {
    encode_dag_cbor(value).unwrap()
}

#[test]
fn deterministic_cid_for_same_input() {
    let v = CborValue::Map(vec![
        ("a".into(), CborValue::Int(1)),
        ("b".into(), CborValue::Bool(true)),
    ]);

    let b1 = enc(&v);
    let b2 = enc(&v);

    let cid1 = compute_cid_dag_cbor(&b1);
    let cid2 = compute_cid_dag_cbor(&b2);

    assert_eq!(cid1, cid2);
}

#[test]
fn different_inputs_produce_different_cids() {
    let cid1 = compute_cid_dag_cbor(&enc(&CborValue::Int(1)));
    let cid2 = compute_cid_dag_cbor(&enc(&CborValue::Int(2)));
    assert_ne!(cid1, cid2);
}

#[test]
fn verify_matching_cid() {
    let v = CborValue::Map(vec![("hello".into(), CborValue::String("world".into()))]);
    let bytes = enc(&v);
    let cid = compute_cid_dag_cbor(&bytes);

    let verification = verify_dag_cbor_cid(&cid, &bytes).unwrap();
    assert_eq!(verification.status(), CidVerificationStatus::Match);
}

#[test]
fn verify_rejects_invalid_uppercase_base32_payload() {
    let bytes = enc(&CborValue::String("canonical cid input".into()));
    let cid = compute_cid_dag_cbor(&bytes);
    let mut invalid_upper_payload = cid.clone();
    invalid_upper_payload.replace_range(1.., &cid[1..].to_ascii_uppercase());

    let verification = verify_dag_cbor_cid(&invalid_upper_payload, &bytes).unwrap();

    assert_eq!(verification.status(), CidVerificationStatus::NonCanonical);
    assert_eq!(verification.expected_cid(), cid);
    assert_eq!(verification.actual_cid(), cid);
}

#[test]
fn verify_rejects_base32_upper_cid_string() {
    let bytes = enc(&CborValue::String("uppercase base32 cid".into()));
    let canonical = compute_cid_dag_cbor(&bytes);
    let parsed = Cid::try_from(canonical.as_str()).unwrap();
    let base32_upper = parsed.to_string_of_base(Base::Base32Upper).unwrap();

    let verification = verify_dag_cbor_cid(&base32_upper, &bytes).unwrap();

    assert_eq!(verification.status(), CidVerificationStatus::NonCanonical);
    assert_eq!(verification.expected_cid(), canonical);
    assert_eq!(verification.actual_cid(), canonical);
}

#[test]
fn verify_rejects_valid_alternate_multibase_cid_strings() {
    let bytes = enc(&CborValue::Map(vec![(
        "cid".into(),
        CborValue::String("alternate multibase".into()),
    )]));
    let canonical = compute_cid_dag_cbor(&bytes);
    let parsed = Cid::try_from(canonical.as_str()).unwrap();
    let base58 = parsed.to_string_of_base(Base::Base58Btc).unwrap();
    let base16 = parsed.to_string_of_base(Base::Base16Lower).unwrap();
    let base2 = parsed.to_string_of_base(Base::Base2).unwrap();
    let emoji = parsed.to_string_of_base(Base::Base256Emoji).unwrap();

    for alternate in [base58, base16, base2, emoji] {
        let verification = verify_dag_cbor_cid(&alternate, &bytes).unwrap();
        assert_eq!(verification.status(), CidVerificationStatus::NonCanonical);
        assert_eq!(verification.expected_cid(), canonical);
        assert_eq!(verification.actual_cid(), canonical);
    }
}

#[test]
fn verification_rejects_non_dag_cbor_payloads() {
    for payload in [
        &[][..],
        &[0xff][..],
        &[0xa2, 0x61, 0x62, 0x01, 0x61, 0x61, 0x02][..],
        &[0x18, 0x01][..],
        &[0xf6, 0xf6][..],
    ] {
        let cid = compute_cid_dag_cbor(payload);
        assert!(verify_dag_cbor_cid(&cid, payload).is_err());
    }
}

#[test]
fn detect_cid_mismatch() {
    let b1 = enc(&CborValue::Int(1));
    let b2 = enc(&CborValue::Int(2));

    let cid_wrong = compute_cid_dag_cbor(&b2);
    let verification = verify_dag_cbor_cid(&cid_wrong, &b1).unwrap();
    assert_eq!(verification.status(), CidVerificationStatus::Mismatch);
}

#[test]
fn verification_binds_the_version_codec_algorithm_length_and_full_digest() {
    const DAG_PB: u64 = 0x70;
    const DAG_CBOR: u64 = 0x71;
    const SHA2_256: u64 = 0x12;
    const SHA2_512: u64 = 0x13;
    const NULL_BLOCK_CID: &str = "bafyreifqwkmiw256ojf2zws6tzjeonw6bpd5vza4i22ccpcq4hjv2ts7cm";

    // The null block is a complete canonical DAG-CBOR value. Keep its digest
    // fixed while varying the CID metadata to isolate the rejection reason.
    let block = [0xf6];
    let digest = sha2_256_content_hash(&block);
    let hash = multihash::Multihash::wrap(SHA2_256, &digest).unwrap();
    // Independent vector from the four CID header bytes and SHA-256 of f6.
    assert_eq!(compute_cid_dag_cbor(&block), NULL_BLOCK_CID);
    let verified = verify_dag_cbor_cid(NULL_BLOCK_CID, &block).unwrap();
    assert_eq!(verified.status(), CidVerificationStatus::Match);
    assert_eq!(verified.expected_cid(), NULL_BLOCK_CID);
    assert_eq!(verified.actual_cid(), NULL_BLOCK_CID);

    let mut different_digest = digest;
    different_digest[31] ^= 1;
    let mut longer_digest = [0; 33];
    longer_digest[..32].copy_from_slice(&digest);
    let mismatches = [
        Cid::new_v0(hash).unwrap(),
        Cid::new_v1(DAG_PB, hash),
        Cid::new_v1(
            DAG_CBOR,
            multihash::Multihash::wrap(SHA2_512, &digest).unwrap(),
        ),
        Cid::new_v1(
            DAG_CBOR,
            multihash::Multihash::wrap(SHA2_256, &digest[..31]).unwrap(),
        ),
        Cid::new_v1(
            DAG_CBOR,
            multihash::Multihash::wrap(SHA2_256, &longer_digest).unwrap(),
        ),
        Cid::new_v1(
            DAG_CBOR,
            multihash::Multihash::wrap(SHA2_256, &different_digest).unwrap(),
        ),
    ];
    for cid in mismatches {
        let supplied = cid.to_string();
        let verified = verify_dag_cbor_cid(&supplied, &block).unwrap();
        assert_eq!(verified.status(), CidVerificationStatus::Mismatch);
        assert_eq!(verified.expected_cid(), NULL_BLOCK_CID);
        assert_eq!(verified.actual_cid(), supplied);
    }
}

#[test]
fn cid_syntax_validation() {
    let v = enc(&CborValue::Int(123));
    let cid = compute_cid_dag_cbor(&v);

    assert!(is_valid_cid_string(&cid));
    assert!(!is_valid_cid_string("not-a-cid"));
}

#[test]
fn cidv0_is_only_accepted_in_its_bare_base58_form() {
    let hash = dag_cbor_multihash(&enc(&CborValue::Int(42)));
    let cid = Cid::new_v0(multihash::Multihash::from_bytes(&hash.to_bytes()).unwrap()).unwrap();
    let bare = cid.to_string();
    assert!(is_valid_cid_string(&bare));
    assert!(try_parse_cid(&bare).is_some());
    let wrapped = format!("z{bare}");
    assert!(!is_valid_cid_string(&wrapped));
    assert!(try_parse_cid(&wrapped).is_none());
}

#[test]
fn content_hash_matches_dag_cbor_multihash_digest() {
    let bytes = enc(&CborValue::String("dag-cbor hash".into()));
    let hash = sha2_256_content_hash(&bytes);
    let multihash = dag_cbor_multihash(&bytes);

    assert_eq!(multihash.code(), 0x12);
    assert_eq!(multihash.size(), 32);
    assert_eq!(multihash.digest(), hash);
}

#[test]
fn random_payloads_produce_valid_cids() {
    for i in 0..50 {
        let v = CborValue::Map(vec![("data".into(), CborValue::Bytes(vec![i; 32]))]);
        let cid = compute_cid_dag_cbor(&enc(&v));
        assert!(is_valid_cid_string(&cid));
    }
}

#[test]
fn cid_parsers_reject_trailing_binary_data_and_paths() {
    let payload = enc(&CborValue::Null);
    let canonical = compute_cid_dag_cbor(&payload);
    let parsed = Cid::try_from(canonical.as_str()).unwrap();
    let mut suffixed = parsed.to_bytes();
    suffixed.extend_from_slice(b"untrusted suffix");
    for invalid in [
        cid::multibase::encode(Base::Base32Lower, &suffixed),
        cid::multibase::encode(Base::Base58Btc, &suffixed),
        format!("/ipfs/{canonical}"),
        format!("https://example.invalid/ipfs/{canonical}"),
    ] {
        assert!(!is_valid_cid_string(&invalid));
        assert!(try_parse_cid(&invalid).is_none());
        let verification = verify_dag_cbor_cid(&invalid, &payload).unwrap();
        assert_eq!(verification.status(), CidVerificationStatus::InvalidCid);
        assert_eq!(verification.expected_cid(), canonical);
        assert!(verification.actual_cid().is_empty());
    }
}

#[test]
fn cid_parsers_preserve_versions_and_alternate_bases() {
    let hash = dag_cbor_multihash(b"version compatibility");
    let v0 = Cid::new_v0(multihash::Multihash::from_bytes(&hash.to_bytes()).unwrap()).unwrap();
    assert_eq!(
        try_parse_cid(&v0.to_string()).map(|cid| cid.to_string()),
        Some(v0.to_string())
    );
    let v1 = Cid::new_v1(
        0x71,
        multihash::Multihash::from_bytes(&hash.to_bytes()).unwrap(),
    );
    for base in [
        Base::Base2,
        Base::Base8,
        Base::Base10,
        Base::Base16Lower,
        Base::Base32Lower,
        Base::Base32Upper,
        Base::Base36Lower,
        Base::Base58Btc,
        Base::Base64,
        Base::Base64Url,
        Base::Base64Pad,
        Base::Base64UrlPad,
        Base::Base32HexLower,
        Base::Base32PadLower,
        Base::Base32Z,
        Base::Base256Emoji,
    ] {
        let text = v1.to_string_of_base(base).unwrap();
        assert_eq!(
            try_parse_cid(&text).map(|cid| cid.to_string()),
            Some(v1.to_string())
        );
    }
    let mut invalid_v0 = v0.to_bytes();
    invalid_v0.push(0);
    assert!(try_parse_cid(&cid::multibase::encode(Base::Base58Btc, invalid_v0)).is_none());
}

#[test]
fn cid_parsers_reject_nonminimal_varints_and_oversized_text() {
    let cid = Cid::new_v1(
        0x71,
        multihash::Multihash::from_bytes(&dag_cbor_multihash(b"minimal").to_bytes()).unwrap(),
    );
    let canonical = cid.to_bytes();
    for index in 0..4 {
        let mut nonminimal = canonical.clone();
        nonminimal[index] |= 0x80;
        nonminimal.insert(index + 1, 0);
        assert!(try_parse_cid(&cid::multibase::encode(Base::Base16Lower, nonminimal)).is_none());
    }
    for prefix in ['z', 'k', 'b'] {
        let oversized = format!("{prefix}{}", "1".repeat(MAX_CID_STRING_LEN));
        assert!(try_parse_cid(&oversized).is_none());
        assert!(!is_valid_cid_string(&oversized));
        let payload = enc(&CborValue::Null);
        assert_eq!(
            verify_dag_cbor_cid(&oversized, &payload).unwrap().status(),
            CidVerificationStatus::InvalidCid
        );
    }
    let largest = Cid::new_v1(
        u64::MAX,
        multihash::Multihash::wrap(u64::MAX, &[0xff; 64]).unwrap(),
    );
    let base2 = largest.to_string_of_base(Base::Base2).unwrap();
    assert!(base2.len() <= MAX_CID_STRING_LEN);
    assert_eq!(
        try_parse_cid(&base2).map(|cid| cid.to_string()),
        Some(largest.to_string())
    );
}

#[test]
fn cid_text_admission_checks_varint_overflow_and_digest_extent() {
    const MAX_DIGEST_BYTES: usize = 64;
    const MAX_U64_VARINT: [u8; 10] = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 1];

    // A canonical CIDv1 can use the full uint64 range for codec and hash
    // codes. Overflow in the tenth octet must not alias the same value.
    let mut wire = vec![1];
    wire.extend_from_slice(&MAX_U64_VARINT);
    wire.extend_from_slice(&MAX_U64_VARINT);
    let length_offset = wire.len();
    wire.push(u8::try_from(MAX_DIGEST_BYTES).unwrap());
    wire.extend_from_slice(&[0xa5; MAX_DIGEST_BYTES]);
    let text = cid::multibase::encode(Base::Base32Lower, &wire);
    assert_eq!(try_parse_cid(&text).unwrap().to_bytes(), wire);

    let mut codec_overflow = wire.clone();
    codec_overflow[10] = 3;
    let mut hash_overflow = wire.clone();
    hash_overflow[20] = 3;
    let mut truncated = wire.clone();
    truncated.pop().unwrap();
    let mut trailing = wire.clone();
    trailing.push(0);
    let mut oversized = trailing.clone();
    oversized[length_offset] = u8::try_from(MAX_DIGEST_BYTES + 1).unwrap();
    for invalid in [
        &codec_overflow,
        &hash_overflow,
        &truncated,
        &trailing,
        &oversized,
    ] {
        let text = cid::multibase::encode(Base::Base32Lower, invalid);
        assert!(try_parse_cid(&text).is_none());
        assert_eq!(
            verify_dag_cbor_cid(&text, &[0xf6]).unwrap().status(),
            CidVerificationStatus::InvalidCid
        );
    }
}

#[test]
fn cid_wire_admission_preserves_upstream_canonical_encodings() {
    let code_width_edges = [0, 0x7f, 0x80, 0x3fff, 0x4000, 1 << 35, 1 << 63, u64::MAX];
    let digest = [0xa5; 64];
    for codec in code_width_edges {
        for algorithm in code_width_edges {
            for length in [0, 1, 31, 32, 63, digest.len()] {
                let cid = Cid::new_v1(
                    codec,
                    multihash::Multihash::wrap(algorithm, &digest[..length]).unwrap(),
                );
                assert_eq!(
                    try_parse_cid(&cid.to_string()).unwrap().to_bytes(),
                    cid.to_bytes()
                );
            }
        }
    }
}
