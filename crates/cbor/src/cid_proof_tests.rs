// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    classify_parsed_cid, dag_cbor_cid, CidVerificationStatus, ContentHash, DagCborMultihash,
};
use cid::{Cid, Version};
use multihash::Multihash;

// Use the wire registry values so a change in the expected CID parameters
// cannot change the reference decision along with the implementation.
const DAG_CBOR: u64 = 0x71;
const SHA2_256: u64 = 0x12;
const SHA2_256_DIGEST_BYTES: usize = 32;
const MAX_DIGEST_BYTES: usize = 64;

#[kani::proof]
#[kani::unwind(65)]
fn cid_match_requires_canonical_text_and_the_complete_digest_identity() {
    let expected_digest: ContentHash = kani::any();
    let actual_digest: [u8; MAX_DIGEST_BYTES] = kani::any();
    let actual_len: usize = kani::any();
    let actual_codec: u64 = kani::any();
    let actual_algorithm: u64 = kani::any();
    let v0: bool = kani::any();
    let text_matches: bool = kani::any();
    kani::assume(actual_len <= actual_digest.len());

    let expected = dag_cbor_cid(DagCborMultihash(
        Multihash::wrap(u64::from(super::Code::Sha2_256), &expected_digest).unwrap(),
    ));
    let hash = Multihash::wrap(actual_algorithm, &actual_digest[..actual_len]).unwrap();
    let actual = if v0 {
        // A parsed v0 is constructible only for a full SHA-256 digest.
        kani::assume(actual_algorithm == SHA2_256 && actual_len == SHA2_256_DIGEST_BYTES);
        Cid::new_v0(hash).unwrap()
    } else {
        Cid::new_v1(actual_codec, hash)
    };

    // Text rendering and SHA-256 are covered separately. Model equality with
    // the canonical rendering using two distinct representative strings.
    let canonical = "canonical";
    let supplied = if text_matches { canonical } else { "alternate" };
    let status = classify_parsed_cid(supplied, canonical, &expected, &actual);
    let same_digest = actual_digest[..SHA2_256_DIGEST_BYTES] == expected_digest;
    let same_identity = !v0
        && actual_codec == DAG_CBOR
        && actual_algorithm == SHA2_256
        && actual_len == SHA2_256_DIGEST_BYTES
        && same_digest;
    let reference = if !text_matches {
        CidVerificationStatus::NonCanonical
    } else if same_identity {
        CidVerificationStatus::Match
    } else {
        CidVerificationStatus::Mismatch
    };

    kani::cover!(
        status == CidVerificationStatus::Match,
        "canonical matching CID"
    );
    kani::cover!(
        !text_matches && same_identity && status == CidVerificationStatus::NonCanonical,
        "alternate text for matching CID"
    );
    kani::cover!(
        v0 && same_digest && status == CidVerificationStatus::Mismatch,
        "v0 with matching digest"
    );
    kani::cover!(
        !v0 && actual_codec != DAG_CBOR
            && same_digest
            && actual_algorithm == SHA2_256
            && actual_len == SHA2_256_DIGEST_BYTES
            && status == CidVerificationStatus::Mismatch,
        "wrong codec with matching digest"
    );
    kani::cover!(
        !v0 && actual_codec == DAG_CBOR
            && same_digest
            && actual_algorithm != SHA2_256
            && actual_len == SHA2_256_DIGEST_BYTES
            && status == CidVerificationStatus::Mismatch,
        "wrong algorithm with matching digest"
    );
    kani::cover!(
        !v0 && actual_codec == DAG_CBOR
            && same_digest
            && actual_algorithm == SHA2_256
            && actual_len < SHA2_256_DIGEST_BYTES
            && status == CidVerificationStatus::Mismatch,
        "truncated digest with matching prefix"
    );
    kani::cover!(
        !v0 && actual_codec == DAG_CBOR
            && !same_digest
            && actual_algorithm == SHA2_256
            && actual_len == SHA2_256_DIGEST_BYTES
            && status == CidVerificationStatus::Mismatch,
        "wrong digest with matching parameters"
    );
    assert_eq!(status, reference);
    assert_eq!(actual.version() == Version::V0, v0);
}
