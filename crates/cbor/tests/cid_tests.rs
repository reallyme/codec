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
use cid::multibase::Base;
use cid::Cid;
use codec_cbor::CborValue;
use codec_cbor::{
    compute_cid_dag_cbor, dag_cbor_multihash, encode_dag_cbor, is_valid_cid_string,
    sha2_256_content_hash, try_parse_cid, verify_dag_cbor_cid, MAX_CID_STRING_LEN,
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

    let (ok, _, _) = verify_dag_cbor_cid(&cid, &bytes);
    assert!(ok);
}

#[test]
fn verify_rejects_invalid_uppercase_base32_payload() {
    let bytes = enc(&CborValue::String("canonical cid input".into()));
    let cid = compute_cid_dag_cbor(&bytes);
    let mut invalid_upper_payload = cid.clone();
    invalid_upper_payload.replace_range(1.., &cid[1..].to_ascii_uppercase());

    let (ok, expected, actual) = verify_dag_cbor_cid(&invalid_upper_payload, &bytes);

    assert!(!ok);
    assert_eq!(expected, cid);
    assert!(actual.is_empty());
}

#[test]
fn verify_rejects_base32_upper_cid_string() {
    let bytes = enc(&CborValue::String("uppercase base32 cid".into()));
    let canonical = compute_cid_dag_cbor(&bytes);
    let parsed = Cid::try_from(canonical.as_str()).unwrap();
    let base32_upper = parsed.to_string_of_base(Base::Base32Upper).unwrap();

    let (ok, expected, actual) = verify_dag_cbor_cid(&base32_upper, &bytes);

    assert!(!ok);
    assert_eq!(expected, canonical);
    assert!(actual.is_empty());
}

#[test]
fn verify_accepts_valid_alternate_multibase_cid_strings() {
    let bytes = enc(&CborValue::Map(vec![(
        "cid".into(),
        CborValue::String("alternate multibase".into()),
    )]));
    let canonical = compute_cid_dag_cbor(&bytes);
    let parsed = Cid::try_from(canonical.as_str()).unwrap();
    let base58 = parsed.to_string_of_base(Base::Base58Btc).unwrap();
    let base16 = parsed.to_string_of_base(Base::Base16Lower).unwrap();

    for alternate in [base58, base16] {
        let (ok, expected, actual) = verify_dag_cbor_cid(&alternate, &bytes);
        assert!(ok);
        assert_eq!(expected, canonical);
        assert_eq!(actual, canonical);
    }
}

#[test]
fn detect_cid_mismatch() {
    let b1 = enc(&CborValue::Int(1));
    let b2 = enc(&CborValue::Int(2));

    let cid_wrong = compute_cid_dag_cbor(&b2);
    let (ok, _, _) = verify_dag_cbor_cid(&cid_wrong, &b1);

    assert!(!ok);
}

#[test]
fn cid_syntax_validation() {
    let v = enc(&CborValue::Int(123));
    let cid = compute_cid_dag_cbor(&v);

    assert!(is_valid_cid_string(&cid));
    assert!(!is_valid_cid_string("not-a-cid"));
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
        assert_eq!(
            verify_dag_cbor_cid(&invalid, &payload),
            (false, canonical.clone(), String::new())
        );
    }
}

#[test]
fn cid_parsers_preserve_versions_and_alternate_bases() {
    let hash = dag_cbor_multihash(b"version compatibility");
    let v0 = Cid::new_v0(hash).unwrap();
    assert_eq!(try_parse_cid(&v0.to_string()), Some(v0));
    let v1 = Cid::new_v1(0x71, hash);
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
        assert_eq!(try_parse_cid(&text), Some(v1));
    }
    let mut invalid_v0 = v0.to_bytes();
    invalid_v0.push(0);
    assert!(try_parse_cid(&cid::multibase::encode(Base::Base58Btc, invalid_v0)).is_none());
}

#[test]
fn cid_parsers_reject_nonminimal_varints_and_oversized_text() {
    let cid = Cid::new_v1(0x71, dag_cbor_multihash(b"minimal"));
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
        assert!(!verify_dag_cbor_cid(&oversized, b"payload").0);
    }
    let largest = Cid::new_v1(
        u64::MAX,
        multihash::Multihash::wrap(u64::MAX, &[0xff; 64]).unwrap(),
    );
    let base2 = largest.to_string_of_base(Base::Base2).unwrap();
    assert!(base2.len() <= MAX_CID_STRING_LEN);
    assert_eq!(try_parse_cid(&base2), Some(largest));
}
