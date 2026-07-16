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
use codec_multibase::{base58btc_decode, base58btc_encode, Base58Error, MAX_BASE58BTC_INPUT_LEN};

#[test]
fn roundtrip() {
    let data = b"hello world";
    let s = base58btc_encode(data).unwrap();
    let out = base58btc_decode(&s).unwrap();
    assert_eq!(out, data);
}

#[test]
fn leading_zeros() {
    let data = [0u8, 0u8, 1u8];
    let s = base58btc_encode(&data).unwrap();
    assert!(s.starts_with("11"));
    let out = base58btc_decode(&s).unwrap();
    assert_eq!(out, data);
}

#[test]
fn roundtrip_pq_sized_payload() {
    let data: Vec<u8> = (0..2_700).map(|i| u8::try_from(i % 251).unwrap()).collect();
    let s = base58btc_encode(&data).unwrap();
    let out = base58btc_decode(&s).unwrap();
    assert_eq!(out, data);
}

#[test]
fn decode_cap_leaves_room_for_largest_supported_multikeys() {
    assert_eq!(MAX_BASE58BTC_INPUT_LEN, 8 * 1024);
    let data: Vec<u8> = (0..3_200).map(|i| u8::try_from(i % 251).unwrap()).collect();
    let encoded = base58btc_encode(&data).unwrap();
    assert!(encoded.len() < MAX_BASE58BTC_INPUT_LEN);
    assert_eq!(base58btc_decode(&encoded).unwrap(), data);
}

#[test]
fn rejects_inputs_above_encode_cap_before_base58_conversion() {
    let oversized = vec![0_u8; MAX_BASE58BTC_INPUT_LEN + 1];
    assert!(matches!(
        base58btc_encode(&oversized),
        Err(Base58Error::InputTooLarge)
    ));
}

#[test]
fn rejects_invalid_char() {
    assert!(base58btc_decode("0").is_err()); // '0' not in alphabet
}

#[test]
fn rejects_inputs_above_decode_cap_before_base58_conversion() {
    let oversized = "1".repeat(MAX_BASE58BTC_INPUT_LEN + 1);
    assert!(matches!(
        base58btc_decode(&oversized),
        Err(Base58Error::InputTooLarge)
    ));
}
