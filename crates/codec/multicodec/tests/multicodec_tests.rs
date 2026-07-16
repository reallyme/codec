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
use codec_multicodec::{
    lookup_codec_prefix, strip_codec_prefix, CodecTag, KeyMaterialKind, VARIABLE_KEY_LENGTH,
};

fn assert_codec(
    name: &'static str,
    expected_prefix: &[u8],
    expected_tag: CodecTag,
    expected_key_material: KeyMaterialKind,
) {
    let found = lookup_codec_prefix(expected_prefix).unwrap();
    assert_eq!(found.name, name);
    assert_eq!(found.codec, expected_prefix);
    assert_eq!(found.tag, expected_tag);
    assert_eq!(found.key_material, expected_key_material);
}

#[test]
fn lookup_ed25519_prefix() {
    let bytes = [0xed, 0x01, 9, 9, 9];
    let found = lookup_codec_prefix(&bytes).unwrap();

    assert_eq!(found.name, "ed25519-pub");
    assert_eq!(found.alg, "Ed25519");
    assert_eq!(found.key_length, 32);
}

#[test]
fn lookup_p384_and_p521_prefixes() {
    let p384 = lookup_codec_prefix(&[0x81, 0x24, 1, 2, 3]).unwrap();
    assert_eq!(p384.name, "p384-pub");
    assert_eq!(p384.alg, "P-384");
    assert_eq!(p384.key_length, 49);

    let p521 = lookup_codec_prefix(&[0x82, 0x24, 1, 2, 3]).unwrap();
    assert_eq!(p521.name, "p521-pub");
    assert_eq!(p521.alg, "P-521");
    assert_eq!(p521.key_length, 67);
}

#[test]
fn lookup_hash_prefixes_from_multicodec_table() {
    assert_codec(
        "sha2-256",
        &[0x12],
        CodecTag::Multihash,
        KeyMaterialKind::NotKey,
    );
    assert_codec(
        "sha2-384",
        &[0x20],
        CodecTag::Multihash,
        KeyMaterialKind::NotKey,
    );
    assert_codec(
        "sha2-512",
        &[0x13],
        CodecTag::Multihash,
        KeyMaterialKind::NotKey,
    );
    assert_codec(
        "sha3-224",
        &[0x17],
        CodecTag::Multihash,
        KeyMaterialKind::NotKey,
    );
    assert_codec(
        "sha3-256",
        &[0x16],
        CodecTag::Multihash,
        KeyMaterialKind::NotKey,
    );
    assert_codec(
        "sha3-384",
        &[0x15],
        CodecTag::Multihash,
        KeyMaterialKind::NotKey,
    );
    assert_codec(
        "sha3-512",
        &[0x14],
        CodecTag::Multihash,
        KeyMaterialKind::NotKey,
    );
}

#[test]
fn lookup_public_key_prefixes_from_multicodec_table() {
    assert_codec(
        "p256-pub",
        &[0x80, 0x24],
        CodecTag::Key,
        KeyMaterialKind::PublicKey,
    );
    assert_codec(
        "p384-pub",
        &[0x81, 0x24],
        CodecTag::Key,
        KeyMaterialKind::PublicKey,
    );
    assert_codec(
        "p521-pub",
        &[0x82, 0x24],
        CodecTag::Key,
        KeyMaterialKind::PublicKey,
    );
    assert_codec(
        "ed448-pub",
        &[0x83, 0x24],
        CodecTag::Key,
        KeyMaterialKind::PublicKey,
    );
    assert_codec(
        "rsa-pub",
        &[0x85, 0x24],
        CodecTag::Key,
        KeyMaterialKind::PublicKey,
    );
    assert_codec(
        "mlkem-512-pub",
        &[0x8b, 0x24],
        CodecTag::Key,
        KeyMaterialKind::PublicKey,
    );
    assert_codec(
        "mlkem-768-pub",
        &[0x8c, 0x24],
        CodecTag::Key,
        KeyMaterialKind::PublicKey,
    );
    assert_codec(
        "mlkem-1024-pub",
        &[0x8d, 0x24],
        CodecTag::Key,
        KeyMaterialKind::PublicKey,
    );
    assert_codec(
        "mldsa-44-pub",
        &[0x90, 0x24],
        CodecTag::Key,
        KeyMaterialKind::PublicKey,
    );
    assert_codec(
        "mldsa-65-pub",
        &[0x91, 0x24],
        CodecTag::Key,
        KeyMaterialKind::PublicKey,
    );
    assert_codec(
        "mldsa-87-pub",
        &[0x92, 0x24],
        CodecTag::Key,
        KeyMaterialKind::PublicKey,
    );
}

#[test]
fn lookup_private_and_symmetric_prefixes_from_multicodec_table() {
    assert_codec(
        "aes-256",
        &[0xa2, 0x01],
        CodecTag::Key,
        KeyMaterialKind::SymmetricKey,
    );
    assert_codec(
        "chacha-256",
        &[0xa4, 0x01],
        CodecTag::Key,
        KeyMaterialKind::SymmetricKey,
    );
    assert_codec(
        "ed25519-priv",
        &[0x80, 0x26],
        CodecTag::Key,
        KeyMaterialKind::PrivateKey,
    );
    assert_codec(
        "secp256k1-priv",
        &[0x81, 0x26],
        CodecTag::Key,
        KeyMaterialKind::PrivateKey,
    );
    assert_codec(
        "x25519-priv",
        &[0x82, 0x26],
        CodecTag::Key,
        KeyMaterialKind::PrivateKey,
    );
    assert_codec(
        "rsa-priv",
        &[0x85, 0x26],
        CodecTag::Key,
        KeyMaterialKind::PrivateKey,
    );
    assert_codec(
        "p256-priv",
        &[0x86, 0x26],
        CodecTag::Key,
        KeyMaterialKind::PrivateKey,
    );
    assert_codec(
        "p384-priv",
        &[0x87, 0x26],
        CodecTag::Key,
        KeyMaterialKind::PrivateKey,
    );
    assert_codec(
        "p521-priv",
        &[0x88, 0x26],
        CodecTag::Key,
        KeyMaterialKind::PrivateKey,
    );
    assert_codec(
        "ed448-priv",
        &[0x91, 0x26],
        CodecTag::Key,
        KeyMaterialKind::PrivateKey,
    );
    assert_codec(
        "mlkem-512-priv",
        &[0x93, 0x26],
        CodecTag::Key,
        KeyMaterialKind::PrivateKey,
    );
    assert_codec(
        "mlkem-768-priv",
        &[0x94, 0x26],
        CodecTag::Key,
        KeyMaterialKind::PrivateKey,
    );
    assert_codec(
        "mlkem-1024-priv",
        &[0x95, 0x26],
        CodecTag::Key,
        KeyMaterialKind::PrivateKey,
    );
}

#[test]
fn lookup_scheme_prefixes_from_multicodec_table() {
    assert_codec(
        "aes-gcm-256",
        &[0x80, 0x40],
        CodecTag::Encryption,
        KeyMaterialKind::NotKey,
    );
    assert_codec(
        "chacha20-poly1305",
        &[0x80, 0xc0, 0x02],
        CodecTag::Multikey,
        KeyMaterialKind::NotKey,
    );
}

#[test]
fn lookup_rsa_prefix() {
    let rsa = lookup_codec_prefix(&[0x85, 0x24, 0x30, 0x82, 0x01, 0x0a]).unwrap();
    assert_eq!(rsa.name, "rsa-pub");
    assert_eq!(rsa.alg, "RSA");
    assert_eq!(rsa.key_length, VARIABLE_KEY_LENGTH);
}

#[test]
fn lookup_unknown_prefix_returns_none() {
    let bytes = [0x01, 0x02, 0x03];
    assert!(lookup_codec_prefix(&bytes).is_none());
}

#[test]
fn strip_prefix_when_known() {
    let bytes = [0xec, 0x01, 1, 2, 3, 4];
    let stripped = strip_codec_prefix(&bytes);
    assert_eq!(stripped, &[1, 2, 3, 4]);
}

#[test]
fn strip_prefix_when_unknown_returns_original() {
    let bytes = [0x01, 0x02, 0x03];
    let stripped = strip_codec_prefix(&bytes);
    assert_eq!(stripped, &bytes);
}

#[test]
fn longest_prefix_wins() {
    // Ensure multi-byte prefixes match correctly
    let bytes = [0x92, 0x24, 0xAA, 0xBB];
    let found = lookup_codec_prefix(&bytes).unwrap();
    assert_eq!(found.name, "mldsa-87-pub");
}
