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
use codec_multikey::{
    binding_type_matches_codec, encode_multikey, parse_multikey, validate_key_binding,
    KeyBindingInput,
};

#[test]
fn binding_type_compatibility() {
    // Classical
    assert!(binding_type_matches_codec("Multikey", "ed25519-pub"));
    assert!(binding_type_matches_codec("Multikey", "ed448-pub"));
    assert!(binding_type_matches_codec("Multikey", "x25519-pub"));
    assert!(binding_type_matches_codec("Multikey", "rsa-pub"));
    assert!(binding_type_matches_codec("Multikey", "secp256k1-pub"));

    // PQ (now explicitly valid Multikeys)
    assert!(binding_type_matches_codec("Multikey", "mldsa-44-pub"));
    assert!(binding_type_matches_codec("Multikey", "mldsa-65-pub"));
    assert!(binding_type_matches_codec("Multikey", "mldsa-87-pub"));
    assert!(binding_type_matches_codec("Multikey", "mlkem-512-pub"));
    assert!(binding_type_matches_codec("Multikey", "mlkem-768-pub"));
    assert!(binding_type_matches_codec("Multikey", "mlkem-1024-pub"));

    // Profile-specific
    assert!(binding_type_matches_codec("P256Key2024", "p256-pub"));
    assert!(binding_type_matches_codec("P384Key2024", "p384-pub"));
    assert!(binding_type_matches_codec("P521Key2024", "p521-pub"));
    assert!(binding_type_matches_codec(
        "RsaVerificationKey2024",
        "rsa-pub"
    ));
    assert!(binding_type_matches_codec(
        "ML_DSA_44Key2024",
        "mldsa-44-pub"
    ));
    assert!(binding_type_matches_codec(
        "ML_DSA_65Key2024",
        "mldsa-65-pub"
    ));
    assert!(binding_type_matches_codec(
        "ML_DSA_87Key2024",
        "mldsa-87-pub"
    ));
    assert!(binding_type_matches_codec(
        "MLKEM512Key2024",
        "mlkem-512-pub"
    ));
    assert!(binding_type_matches_codec(
        "MLKEM768Key2024",
        "mlkem-768-pub"
    ));
    assert!(binding_type_matches_codec(
        "MLKEM1024Key2024",
        "mlkem-1024-pub"
    ));

    // Mismatches
    assert!(!binding_type_matches_codec("P256Key2024", "ed25519-pub"));
    assert!(!binding_type_matches_codec(
        "RsaVerificationKey2024",
        "ed25519-pub"
    ));
    assert!(!binding_type_matches_codec(
        "ML_DSA_44Key2024",
        "ed25519-pub"
    ));
    assert!(!binding_type_matches_codec(
        "ML_DSA_87Key2024",
        "ed25519-pub"
    ));
}

#[test]
fn validate_binding_success_ed25519() {
    let pk = vec![9u8; 32];
    let mk = encode_multikey("ed25519-pub", &pk).unwrap();
    let parsed = parse_multikey(&mk).unwrap();

    validate_key_binding(
        KeyBindingInput {
            binding_type: "Multikey",
            algorithm: Some("Ed25519"),
        },
        &parsed,
    )
    .unwrap();
}

#[test]
fn validate_binding_success_rsa() {
    let pk = vec![0x30, 0x82, 0x01, 0x0a, 0x02, 0x82, 0x01, 0x01, 0xaa];
    let mk = encode_multikey("rsa-pub", &pk).unwrap();
    let parsed = parse_multikey(&mk).unwrap();

    validate_key_binding(
        KeyBindingInput {
            binding_type: "RsaVerificationKey2024",
            algorithm: Some("RSA"),
        },
        &parsed,
    )
    .unwrap();
}

#[test]
fn validate_binding_success_p384() {
    let mut pk = vec![0u8; 49];
    pk[0] = 0x02;
    let mk = encode_multikey("p384-pub", &pk).unwrap();
    let parsed = parse_multikey(&mk).unwrap();

    validate_key_binding(
        KeyBindingInput {
            binding_type: "P384Key2024",
            algorithm: Some("P-384"),
        },
        &parsed,
    )
    .unwrap();
}

#[test]
fn validate_binding_success_p521() {
    let mut pk = vec![0u8; 67];
    pk[0] = 0x03;
    let mk = encode_multikey("p521-pub", &pk).unwrap();
    let parsed = parse_multikey(&mk).unwrap();

    validate_key_binding(
        KeyBindingInput {
            binding_type: "P521Key2024",
            algorithm: Some("P-521"),
        },
        &parsed,
    )
    .unwrap();
}

#[test]
fn validate_binding_success_ed448() {
    let pk = vec![8u8; 57];
    let mk = encode_multikey("ed448-pub", &pk).unwrap();
    let parsed = parse_multikey(&mk).unwrap();

    validate_key_binding(
        KeyBindingInput {
            binding_type: "Multikey",
            algorithm: Some("Ed448"),
        },
        &parsed,
    )
    .unwrap();
}

#[test]
fn validate_binding_success_ml_dsa_44() {
    let pk = vec![1u8; 1312];
    let mk = encode_multikey("mldsa-44-pub", &pk).unwrap();
    let parsed = parse_multikey(&mk).unwrap();

    validate_key_binding(
        KeyBindingInput {
            binding_type: "Multikey",
            algorithm: Some("ML-DSA-44"),
        },
        &parsed,
    )
    .unwrap();
}

#[test]
fn validate_binding_success_ml_dsa_65() {
    let pk = vec![1u8; 1952];
    let mk = encode_multikey("mldsa-65-pub", &pk).unwrap();
    let parsed = parse_multikey(&mk).unwrap();

    validate_key_binding(
        KeyBindingInput {
            binding_type: "Multikey",
            algorithm: Some("ML-DSA-65"),
        },
        &parsed,
    )
    .unwrap();
}

#[test]
fn validate_binding_success_ml_dsa_87() {
    let pk = vec![1u8; 2592];
    let mk = encode_multikey("mldsa-87-pub", &pk).unwrap();
    let parsed = parse_multikey(&mk).unwrap();

    validate_key_binding(
        KeyBindingInput {
            binding_type: "Multikey",
            algorithm: Some("ML-DSA-87"),
        },
        &parsed,
    )
    .unwrap();
}

#[test]
fn validate_binding_success_mlkem512() {
    let pk = vec![2u8; 800];
    let mk = encode_multikey("mlkem-512-pub", &pk).unwrap();
    let parsed = parse_multikey(&mk).unwrap();

    validate_key_binding(
        KeyBindingInput {
            binding_type: "Multikey",
            algorithm: Some("ML-KEM-512"),
        },
        &parsed,
    )
    .unwrap();
}

#[test]
fn validate_binding_success_mlkem768() {
    let pk = vec![2u8; 1184];
    let mk = encode_multikey("mlkem-768-pub", &pk).unwrap();
    let parsed = parse_multikey(&mk).unwrap();

    validate_key_binding(
        KeyBindingInput {
            binding_type: "Multikey",
            algorithm: Some("ML-KEM-768"),
        },
        &parsed,
    )
    .unwrap();
}

#[test]
fn validate_binding_success_mlkem1024() {
    let pk = vec![2u8; 1568];
    let mk = encode_multikey("mlkem-1024-pub", &pk).unwrap();
    let parsed = parse_multikey(&mk).unwrap();

    validate_key_binding(
        KeyBindingInput {
            binding_type: "Multikey",
            algorithm: Some("ML-KEM-1024"),
        },
        &parsed,
    )
    .unwrap();
}

#[test]
fn validate_binding_algorithm_mismatch() {
    let pk = vec![9u8; 32];
    let mk = encode_multikey("ed25519-pub", &pk).unwrap();
    let parsed = parse_multikey(&mk).unwrap();

    assert!(validate_key_binding(
        KeyBindingInput {
            binding_type: "Multikey",
            algorithm: Some("P256"),
        },
        &parsed,
    )
    .is_err());
}
