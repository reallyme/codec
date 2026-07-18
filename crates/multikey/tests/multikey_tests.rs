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
use codec_base64url::bytes_to_base64url;
use codec_multibase::bytes_to_multibase58btc;
use codec_multikey::{encode_multikey, parse_multikey};

const MAX_RSA_PUBLIC_KEY_DER_LEN: usize = 4 * 1024;

#[test]
fn encode_and_parse_ed25519() {
    let pk = vec![7u8; 32];
    let mk = encode_multikey("ed25519-pub", &pk).unwrap();
    assert!(mk.starts_with('z'));

    let parsed = parse_multikey(&mk).unwrap();
    assert_eq!(parsed.codec_name, "ed25519-pub");
    assert_eq!(parsed.alg, "Ed25519");
    assert_eq!(parsed.public_key, pk);
}

#[test]
fn encode_and_parse_p384() {
    let mut pk = vec![0u8; 49];
    pk[0] = 0x02;
    let mk = encode_multikey("p384-pub", &pk).unwrap();

    let parsed = parse_multikey(&mk).unwrap();
    assert_eq!(parsed.codec_name, "p384-pub");
    assert_eq!(parsed.alg, "P-384");
    assert_eq!(parsed.public_key, pk);
}

#[test]
fn encode_and_parse_p521() {
    let mut pk = vec![0u8; 67];
    pk[0] = 0x03;
    let mk = encode_multikey("p521-pub", &pk).unwrap();

    let parsed = parse_multikey(&mk).unwrap();
    assert_eq!(parsed.codec_name, "p521-pub");
    assert_eq!(parsed.alg, "P-521");
    assert_eq!(parsed.public_key, pk);
}

#[test]
fn encode_and_parse_rsa_variable_length_der() {
    let pkcs1_der_like = vec![0x30, 0x82, 0x01, 0x0a, 0x02, 0x82, 0x01, 0x01, 0xaa];
    let mk = encode_multikey("rsa-pub", &pkcs1_der_like).unwrap();

    let parsed = parse_multikey(&mk).unwrap();
    assert_eq!(parsed.codec_name, "rsa-pub");
    assert_eq!(parsed.alg, "RSA");
    assert_eq!(parsed.public_key, pkcs1_der_like);
    assert_eq!(parsed.key_length, codec_multicodec::VARIABLE_KEY_LENGTH);
}

#[test]
fn rsa_variable_length_der_has_semantic_upper_bound() {
    let too_large = vec![0x30; MAX_RSA_PUBLIC_KEY_DER_LEN + 1];
    assert!(encode_multikey("rsa-pub", &too_large).is_err());
}

#[test]
fn parse_rejects_oversized_rsa_variable_length_der() {
    let mut payload = vec![0x85, 0x24];
    payload.extend_from_slice(&vec![0x30; MAX_RSA_PUBLIC_KEY_DER_LEN + 1]);
    let multikey = bytes_to_multibase58btc(&payload).unwrap();
    assert!(parse_multikey(&multikey).is_err());
}

#[test]
fn encode_and_parse_ed448() {
    let pk = vec![8u8; 57];
    let mk = encode_multikey("ed448-pub", &pk).unwrap();

    let parsed = parse_multikey(&mk).unwrap();
    assert_eq!(parsed.codec_name, "ed448-pub");
    assert_eq!(parsed.alg, "Ed448");
    assert_eq!(parsed.public_key, pk);
}

#[test]
fn encode_and_parse_ml_dsa_44() {
    let pk = vec![1u8; 1312];
    let mk = encode_multikey("mldsa-44-pub", &pk).unwrap();

    let parsed = parse_multikey(&mk).unwrap();
    assert_eq!(parsed.codec_name, "mldsa-44-pub");
    assert_eq!(parsed.alg, "ML-DSA-44");
    assert_eq!(parsed.public_key, pk);
}

#[test]
fn encode_and_parse_ml_dsa_65() {
    let pk = vec![1u8; 1952];
    let mk = encode_multikey("mldsa-65-pub", &pk).unwrap();

    let parsed = parse_multikey(&mk).unwrap();
    assert_eq!(parsed.codec_name, "mldsa-65-pub");
    assert_eq!(parsed.alg, "ML-DSA-65");
    assert_eq!(parsed.public_key, pk);
}

#[test]
fn encode_and_parse_ml_dsa_87() {
    let pk = vec![1u8; 2592];
    let mk = encode_multikey("mldsa-87-pub", &pk).unwrap();

    let parsed = parse_multikey(&mk).unwrap();
    assert_eq!(parsed.codec_name, "mldsa-87-pub");
    assert_eq!(parsed.alg, "ML-DSA-87");
    assert_eq!(parsed.public_key, pk);
}

#[test]
fn encode_and_parse_mlkem512() {
    let pk = vec![2u8; 800];
    let mk = encode_multikey("mlkem-512-pub", &pk).unwrap();

    let parsed = parse_multikey(&mk).unwrap();
    assert_eq!(parsed.codec_name, "mlkem-512-pub");
    assert_eq!(parsed.alg, "ML-KEM-512");
    assert_eq!(parsed.public_key, pk);
}

#[test]
fn encode_and_parse_mlkem768() {
    let pk = vec![2u8; 1184];
    let mk = encode_multikey("mlkem-768-pub", &pk).unwrap();

    let parsed = parse_multikey(&mk).unwrap();
    assert_eq!(parsed.codec_name, "mlkem-768-pub");
    assert_eq!(parsed.alg, "ML-KEM-768");
    assert_eq!(parsed.public_key, pk);
}

#[test]
fn encode_and_parse_mlkem1024() {
    let pk = vec![2u8; 1568];
    let mk = encode_multikey("mlkem-1024-pub", &pk).unwrap();

    let parsed = parse_multikey(&mk).unwrap();
    assert_eq!(parsed.codec_name, "mlkem-1024-pub");
    assert_eq!(parsed.alg, "ML-KEM-1024");
    assert_eq!(parsed.public_key, pk);
}

#[test]
fn rejects_wrong_length() {
    let pk = vec![7u8; 31];
    assert!(encode_multikey("ed25519-pub", &pk).is_err());
}

#[test]
fn parse_rejects_unknown_prefix() {
    let mk = "z1"; // invalid / too short
    assert!(parse_multikey(mk).is_err());
}

#[test]
fn parse_rejects_non_canonical_base64url_multikey() {
    let pk = vec![7u8; 32];
    let canonical = encode_multikey("ed25519-pub", &pk).unwrap();
    let parsed = parse_multikey(&canonical).unwrap();

    let mut payload = vec![0xed, 0x01];
    payload.extend_from_slice(&pk);
    let malleable = format!("u{}", bytes_to_base64url(&payload));

    assert!(parse_multikey(&malleable).is_err());
    assert_ne!(canonical, malleable);
    assert_eq!(parsed.public_key, pk);
}

#[test]
fn encode_rejects_non_public_key_codecs() {
    let secret_key = vec![7u8; 32];
    assert!(encode_multikey("ed25519-priv", &secret_key).is_err());
    assert!(encode_multikey("aes-256", &secret_key).is_err());

    let digest = vec![7u8; 32];
    assert!(encode_multikey("sha2-256", &digest).is_err());
}

#[test]
fn parse_rejects_non_public_key_prefixes() {
    let private_key_payload = [0x80, 0x26, 7, 7, 7, 7];
    let private_key_multibase = bytes_to_multibase58btc(&private_key_payload).unwrap();
    assert!(parse_multikey(&private_key_multibase).is_err());

    let symmetric_key_payload = [0xa2, 0x01, 7, 7, 7, 7];
    let symmetric_key_multibase = bytes_to_multibase58btc(&symmetric_key_payload).unwrap();
    assert!(parse_multikey(&symmetric_key_multibase).is_err());

    let hash_payload = [0x12, 7, 7, 7, 7];
    let hash_multibase = bytes_to_multibase58btc(&hash_payload).unwrap();
    assert!(parse_multikey(&hash_multibase).is_err());
}
