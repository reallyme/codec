// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(missing_docs)]

use codec_multibase::{Base58Error, MultibaseError, MAX_BASE58BTC_INPUT_LEN};
use reallyme_codec_adapter::scalar_ops::{
    compute_dag_cbor_cid, dag_cbor_content_hash, dag_cbor_multihash_value, decode_base58btc,
    decode_multibase, ScalarOperationError, MAX_DAG_CBOR_INPUT_BYTES,
};

#[test]
fn dag_cbor_hash_operations_accept_exact_input_cap_and_reject_one_more() {
    let mut input = vec![0_u8; MAX_DAG_CBOR_INPUT_BYTES];
    assert!(compute_dag_cbor_cid(&input).is_ok());
    assert!(dag_cbor_content_hash(&input).is_ok());
    assert!(dag_cbor_multihash_value(&input).is_ok());

    input.push(0);
    assert!(matches!(
        compute_dag_cbor_cid(&input),
        Err(ScalarOperationError::InputTooLarge)
    ));
    assert!(matches!(
        dag_cbor_content_hash(&input),
        Err(ScalarOperationError::InputTooLarge)
    ));
    assert!(matches!(
        dag_cbor_multihash_value(&input),
        Err(ScalarOperationError::InputTooLarge)
    ));
}

#[test]
fn base58_text_caps_are_applied_before_decoding() {
    // An invalid alphabet character separates the text-length guard from the
    // decoder's independent decoded-byte limit at the exact boundary.
    let exact = "0".repeat(MAX_BASE58BTC_INPUT_LEN);
    assert!(matches!(
        decode_base58btc(&exact),
        Err(Base58Error::InvalidCharacter)
    ));
    assert!(matches!(
        decode_multibase(&format!("z{exact}")),
        Err(MultibaseError::Base58(Base58Error::InvalidCharacter))
    ));

    let over_limit = "0".repeat(MAX_BASE58BTC_INPUT_LEN + 1);
    assert!(matches!(
        decode_base58btc(&over_limit),
        Err(Base58Error::InputTooLarge)
    ));
    assert!(matches!(
        decode_multibase(&format!("z{over_limit}")),
        Err(MultibaseError::Base58(Base58Error::InputTooLarge))
    ));
}
