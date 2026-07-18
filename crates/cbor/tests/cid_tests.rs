// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

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
    sha2_256_content_hash, verify_dag_cbor_cid,
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
