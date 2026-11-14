// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(clippy::unwrap_used)]

use codec_cbor::{compute_cid_dag_cbor, CidVerificationStatus};

use super::{
    verify_dag_cbor_cid, verify_primitive_dag_cbor_cid, DagCborOperationError,
    MAX_DAG_CBOR_INPUT_LEN,
};

#[test]
fn verify_dag_cbor_cid_preserves_primitive_verification_semantics() {
    let payload = [0xa0];
    let cid = compute_cid_dag_cbor(&payload);
    let primitive = verify_primitive_dag_cbor_cid(&cid, &payload).unwrap();
    let verification = verify_dag_cbor_cid(&cid, &payload).unwrap();

    assert_eq!(
        verification.valid(),
        primitive.status() == CidVerificationStatus::Match
    );
    assert_eq!(verification.expected_cid(), primitive.expected_cid());
    assert_eq!(verification.actual_cid(), primitive.actual_cid());
}

#[test]
fn verify_dag_cbor_cid_preserves_invalid_cid_sanitization() {
    let payload = [0xa0];
    let verification = verify_dag_cbor_cid("not-a-cid", &payload).unwrap();

    assert!(!verification.valid());
    assert_eq!(verification.expected_cid(), compute_cid_dag_cbor(&payload));
    assert_eq!(verification.actual_cid(), "");
}

#[test]
fn verify_dag_cbor_cid_rejects_payloads_above_limit() {
    let payload = vec![0_u8; MAX_DAG_CBOR_INPUT_LEN + 1];
    let error = match verify_dag_cbor_cid("", &payload) {
        Ok(_) => DagCborOperationError::PayloadTooLarge,
        Err(error) => error,
    };

    assert_eq!(error, DagCborOperationError::PayloadTooLarge);
}

impl super::DagCborCidVerification {
    /// Return whether the supplied CID equals the canonical CID for the payload.
    pub const fn valid(&self) -> bool {
        self.valid
    }
}

impl super::DagCborCidVerification {
    /// Return the canonical CID computed from the supplied payload.
    pub fn expected_cid(&self) -> &str {
        self.expected_cid.as_str()
    }
}

impl super::DagCborCidVerification {
    /// Return the canonical supplied CID, or an empty string for invalid input.
    pub fn actual_cid(&self) -> &str {
        self.actual_cid.as_str()
    }
}
