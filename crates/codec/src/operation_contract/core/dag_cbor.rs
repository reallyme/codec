// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

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
    CidVerificationStatus, DeterministicCborError, DeterministicCborValue, MAX_DAG_CBOR_INPUT_LEN,
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
    /// The bytes do not form one canonical DAG-CBOR block.
    #[error("invalid dag-cbor payload")]
    InvalidPayload(CborError),
}

/// Result of verifying a supplied CID against a DAG-CBOR payload.
pub struct DagCborCidVerification {
    valid: bool,
    expected_cid: String,
    actual_cid: String,
}

impl DagCborCidVerification {
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

    let verified = verify_primitive_dag_cbor_cid(cid, payload)
        .map_err(DagCborOperationError::InvalidPayload)?;
    let (status, expected_cid, actual_cid) = verified.into_parts();
    Ok(DagCborCidVerification {
        valid: status == CidVerificationStatus::Match,
        expected_cid,
        actual_cid,
    })
}

/// Encode one validated DAG-CBOR domain value.
pub fn encode_dag_cbor_value(value: &CborValue) -> Result<Zeroizing<Vec<u8>>, CborError> {
    encode_primitive_dag_cbor(value).map(Zeroizing::new)
}

/// Decode one DAG-CBOR byte sequence into the domain value.
pub fn decode_dag_cbor_value(bytes: &[u8]) -> Result<Zeroizing<CborValue>, CborError> {
    decode_primitive_dag_cbor(bytes).map(Zeroizing::new)
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
#[path = "dag_cbor_tests.rs"]
mod tests;
