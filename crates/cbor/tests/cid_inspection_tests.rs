// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(
    missing_docs,
    clippy::unwrap_used,
    reason = "fixed CID syntax fixtures"
)]

use cid::multibase::{encode, Base};
use codec_cbor::{
    compute_cid_dag_cbor, is_valid_cid_string, sha2_256_content_hash, try_parse_cid,
    verify_dag_cbor_cid, CidVerificationStatus,
};

fn syntax_fixtures() -> [(&'static str, Vec<u8>, usize); 3] {
    // Use a genuine digest prefix so the short-hash case also proves that
    // matching a prefix is insufficient for DAG-CBOR content verification.
    let digest = sha2_256_content_hash(&[0xf6]);
    let mut short = vec![1, 0x71, 0x12, 4];
    short.extend_from_slice(&digest[..4]);
    let mut unknown = vec![1, 0x71, 0xff, 0xff, 3, 32]; // Hash code 0xffff.
    unknown.extend_from_slice(&digest);
    let mut identity = vec![1, 0x71, 0, 64];
    identity.extend_from_slice(&[0xa5; 64]);
    [
        ("bafyrebfqwkmiw", short, 3),
        (
            "bafy777ydecylfgelno7hes5m3jpj4ushg3paxr624qoennbbhriodu25jzprg",
            unknown,
            5,
        ),
        (
            concat!(
                "bafyqaqffuws2ljnfuws2ljnfuws2ljnfuws2ljnfuws2ljnfuws2ljnfuws2ljn",
                "fuws2ljnfuws2ljnfuws2ljnfuws2ljnfuws2ljnfuws2k"
            ),
            identity,
            3,
        ),
    ]
}

#[test]
fn cid_inspection_preserves_opaque_hash_codes_and_bounded_digest_lengths() {
    for (text, wire, _) in syntax_fixtures() {
        assert_eq!(encode(Base::Base32Lower, &wire), text);
        assert!(is_valid_cid_string(text));
        let parsed = try_parse_cid(text).unwrap();
        assert_eq!(parsed.to_bytes(), wire);
        assert_eq!(parsed.to_string(), text);
    }
}

#[test]
fn dag_cbor_verification_requires_the_full_supported_cid_identity() {
    let block = [0xf6];
    let expected = compute_cid_dag_cbor(&block);
    assert_eq!(
        verify_dag_cbor_cid(&expected, &block).unwrap().status(),
        CidVerificationStatus::Match
    );
    for (text, mut wire, length_offset) in syntax_fixtures() {
        let verification = verify_dag_cbor_cid(text, &block).unwrap();
        assert_eq!(verification.status(), CidVerificationStatus::Mismatch);
        assert_eq!(verification.expected_cid(), expected);
        assert_eq!(verification.actual_cid(), text);

        // Syntax admission still checks exact extents even for opaque hashes.
        wire[length_offset] = wire[length_offset].checked_add(1).unwrap();
        let malformed = encode(Base::Base32Lower, &wire);
        assert!(!is_valid_cid_string(&malformed));
        assert!(try_parse_cid(&malformed).is_none());
        assert_eq!(
            verify_dag_cbor_cid(&malformed, &block).unwrap().status(),
            CidVerificationStatus::InvalidCid
        );
    }
    let mut oversized = vec![1, 0x71, 0, 65];
    oversized.extend_from_slice(&[0xa5; 65]);
    assert!(try_parse_cid(&encode(Base::Base32Lower, oversized)).is_none());
}
