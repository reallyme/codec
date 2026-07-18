// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Operation-specific semantic layer for structured DAG-CBOR operations.
//!
//! Boundary adapters own protobuf, FFI, JNI, WASM, and SDK representation.
//! This module owns operation-level limits and result meaning so adapters do
//! not reimplement DAG-CBOR CID verification policy.

use codec_cbor::{
    decode_dag_cbor as decode_primitive_dag_cbor,
    decode_deterministic_cbor as decode_primitive_deterministic_cbor,
    encode_dag_cbor as encode_primitive_dag_cbor,
    encode_deterministic_cbor as encode_primitive_deterministic_cbor,
    verify_dag_cbor_cid as verify_primitive_dag_cbor_cid, CborError, CborValue,
    DeterministicCborError, DeterministicCborValue, MAX_DAG_CBOR_INPUT_LEN,
};
use zeroize::Zeroizing;

/// Semantic failure reasons for structured DAG-CBOR operations.
///
/// The display strings are fixed and do not include caller input or payload
/// bytes. Boundary adapters map these reasons into their public typed error
/// contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DagCborOperationError {
    /// The supplied DAG-CBOR payload exceeds the operation input limit.
    #[error("dag-cbor payload too large")]
    PayloadTooLarge,
}

/// Result of verifying a supplied CID against a DAG-CBOR payload.
pub struct DagCborCidVerification {
    valid: bool,
    expected_cid: String,
    actual_cid: String,
}

impl DagCborCidVerification {
    /// Return whether the supplied CID equals the canonical CID for the payload.
    pub const fn valid(&self) -> bool {
        self.valid
    }

    /// Return the canonical CID computed from the supplied payload.
    pub fn expected_cid(&self) -> &str {
        self.expected_cid.as_str()
    }

    /// Return the canonical supplied CID, or an empty string for invalid input.
    pub fn actual_cid(&self) -> &str {
        self.actual_cid.as_str()
    }

    /// Consume the verification result without cloning its owned CID strings.
    ///
    /// Structured boundary adapters use this ownership transfer when building
    /// generated result messages. Keeping the transfer here prevents each
    /// adapter from allocating a second copy of the same canonical result.
    pub(crate) fn into_parts(self) -> (bool, String, String) {
        (self.valid, self.expected_cid, self.actual_cid)
    }
}

/// Verify a supplied CID against a bounded DAG-CBOR payload.
pub fn verify_dag_cbor_cid(
    cid: &str,
    payload: &[u8],
) -> Result<DagCborCidVerification, DagCborOperationError> {
    if payload.len() > MAX_DAG_CBOR_INPUT_LEN {
        return Err(DagCborOperationError::PayloadTooLarge);
    }

    let (valid, expected_cid, actual_cid) = verify_primitive_dag_cbor_cid(cid, payload);
    Ok(DagCborCidVerification {
        valid,
        expected_cid,
        actual_cid,
    })
}

/// Encode one validated DAG-CBOR domain value.
pub fn encode_dag_cbor_value(value: &CborValue) -> Result<Zeroizing<Vec<u8>>, CborError> {
    encode_primitive_dag_cbor(value).map(Zeroizing::new)
}

/// Decode one DAG-CBOR byte sequence into the domain value.
pub fn decode_dag_cbor_value(bytes: &[u8]) -> Result<CborValue, CborError> {
    decode_primitive_dag_cbor(bytes)
}

/// Encode one validated deterministic-CBOR domain value.
pub fn encode_deterministic_cbor_value(
    value: &DeterministicCborValue,
) -> Result<Zeroizing<Vec<u8>>, DeterministicCborError> {
    encode_primitive_deterministic_cbor(value)
}

/// Decode one deterministic-CBOR byte sequence into the domain value.
pub fn decode_deterministic_cbor_value(
    bytes: &[u8],
) -> Result<DeterministicCborValue, DeterministicCborError> {
    decode_primitive_deterministic_cbor(bytes)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use codec_cbor::compute_cid_dag_cbor;

    use super::{
        verify_dag_cbor_cid, verify_primitive_dag_cbor_cid, DagCborOperationError,
        MAX_DAG_CBOR_INPUT_LEN,
    };

    #[test]
    fn verify_dag_cbor_cid_preserves_primitive_verification_semantics() {
        let payload = [0xa0];
        let cid = compute_cid_dag_cbor(&payload);
        let (primitive_valid, primitive_expected, primitive_actual) =
            verify_primitive_dag_cbor_cid(&cid, &payload);
        let verification = verify_dag_cbor_cid(&cid, &payload).unwrap();

        assert_eq!(verification.valid(), primitive_valid);
        assert_eq!(verification.expected_cid(), primitive_expected);
        assert_eq!(verification.actual_cid(), primitive_actual);
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
}
