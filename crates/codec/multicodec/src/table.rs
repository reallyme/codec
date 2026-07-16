// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

/// Multicodec table tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecTag {
    /// `encryption` table tag.
    Encryption,
    /// `hash` table tag.
    Hash,
    /// `key` table tag.
    Key,
    /// `multihash` table tag.
    Multihash,
    /// `multikey` table tag.
    Multikey,
}

/// Key-material class for codecs that carry raw key bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyMaterialKind {
    /// The codec is not raw key material.
    NotKey,
    /// Public key material that is safe for `multikey` verification methods.
    PublicKey,
    /// Private key material. The codec is known, but `multikey` must reject it.
    PrivateKey,
    /// Symmetric key material. The codec is known, but `multikey` must reject it.
    SymmetricKey,
}

/// Static metadata for one multicodec entry.
#[derive(Debug, Clone)]
pub struct CodecSpec {
    /// Multicodec table tag.
    pub tag: CodecTag,

    /// Key-material class for `key` codecs.
    pub key_material: KeyMaterialKind,

    /// Human-readable algorithm name
    pub alg: &'static str,

    /// Multicodec varint prefix bytes
    pub codec: &'static [u8],

    /// Expected raw public key length AFTER prefix
    pub key_length: usize,
}

/// Variable-length key payload. RSA keys use DER, whose length depends on
/// modulus size and integer leading-byte normalization.
pub const VARIABLE_KEY_LENGTH: usize = 0;

/// Fixed length does not apply to this codec.
pub const FIXED_LENGTH_NOT_APPLICABLE: usize = 0;

/// Single source of truth for multicodec prefixes
pub static MULTICODEC_TABLE: &[(&str, CodecSpec)] = &[
    (
        "sha2-256",
        CodecSpec {
            tag: CodecTag::Multihash,
            key_material: KeyMaterialKind::NotKey,
            alg: "SHA2-256",
            codec: &[0x12],
            key_length: 32,
        },
    ),
    (
        "sha2-512",
        CodecSpec {
            tag: CodecTag::Multihash,
            key_material: KeyMaterialKind::NotKey,
            alg: "SHA2-512",
            codec: &[0x13],
            key_length: 64,
        },
    ),
    (
        "sha3-512",
        CodecSpec {
            tag: CodecTag::Multihash,
            key_material: KeyMaterialKind::NotKey,
            alg: "SHA3-512",
            codec: &[0x14],
            key_length: 64,
        },
    ),
    (
        "sha3-384",
        CodecSpec {
            tag: CodecTag::Multihash,
            key_material: KeyMaterialKind::NotKey,
            alg: "SHA3-384",
            codec: &[0x15],
            key_length: 48,
        },
    ),
    (
        "sha3-256",
        CodecSpec {
            tag: CodecTag::Multihash,
            key_material: KeyMaterialKind::NotKey,
            alg: "SHA3-256",
            codec: &[0x16],
            key_length: 32,
        },
    ),
    (
        "sha3-224",
        CodecSpec {
            tag: CodecTag::Multihash,
            key_material: KeyMaterialKind::NotKey,
            alg: "SHA3-224",
            codec: &[0x17],
            key_length: 28,
        },
    ),
    (
        "sha2-384",
        CodecSpec {
            tag: CodecTag::Multihash,
            key_material: KeyMaterialKind::NotKey,
            alg: "SHA2-384",
            codec: &[0x20],
            key_length: 48,
        },
    ),
    (
        "aes-128",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::SymmetricKey,
            alg: "AES-128",
            codec: &[0xa0, 0x01],
            key_length: 16,
        },
    ),
    (
        "aes-192",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::SymmetricKey,
            alg: "AES-192",
            codec: &[0xa1, 0x01],
            key_length: 24,
        },
    ),
    (
        "aes-256",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::SymmetricKey,
            alg: "AES-256",
            codec: &[0xa2, 0x01],
            key_length: 32,
        },
    ),
    (
        "chacha-128",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::SymmetricKey,
            alg: "ChaCha-128",
            codec: &[0xa3, 0x01],
            key_length: 16,
        },
    ),
    (
        "chacha-256",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::SymmetricKey,
            alg: "ChaCha-256",
            codec: &[0xa4, 0x01],
            key_length: 32,
        },
    ),
    (
        "ed25519-pub",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PublicKey,
            alg: "Ed25519",
            codec: &[0xed, 0x01],
            key_length: 32,
        },
    ),
    (
        "x25519-pub",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PublicKey,
            alg: "X25519",
            codec: &[0xec, 0x01],
            key_length: 32,
        },
    ),
    (
        "p256-pub",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PublicKey,
            alg: "P-256",
            codec: &[0x80, 0x24],
            key_length: 33,
        },
    ),
    (
        "p384-pub",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PublicKey,
            alg: "P-384",
            codec: &[0x81, 0x24],
            key_length: 49,
        },
    ),
    (
        "p521-pub",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PublicKey,
            alg: "P-521",
            codec: &[0x82, 0x24],
            key_length: 67,
        },
    ),
    (
        "ed448-pub",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PublicKey,
            alg: "Ed448",
            codec: &[0x83, 0x24],
            key_length: 57,
        },
    ),
    (
        "rsa-pub",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PublicKey,
            alg: "RSA",
            codec: &[0x85, 0x24],
            key_length: VARIABLE_KEY_LENGTH,
        },
    ),
    (
        "secp256k1-pub",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PublicKey,
            alg: "secp256k1",
            codec: &[0xe7, 0x01],
            key_length: 33,
        },
    ),
    (
        "mldsa-44-pub",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PublicKey,
            alg: "ML-DSA-44",
            codec: &[0x90, 0x24],
            key_length: 1312,
        },
    ),
    (
        "mldsa-65-pub",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PublicKey,
            alg: "ML-DSA-65",
            codec: &[0x91, 0x24],
            key_length: 1952,
        },
    ),
    (
        "mldsa-87-pub",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PublicKey,
            alg: "ML-DSA-87",
            codec: &[0x92, 0x24],
            key_length: 2592,
        },
    ),
    (
        "mlkem-512-pub",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PublicKey,
            alg: "ML-KEM-512",
            codec: &[0x8b, 0x24],
            key_length: 800,
        },
    ),
    (
        "mlkem-768-pub",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PublicKey,
            alg: "ML-KEM-768",
            codec: &[0x8c, 0x24],
            key_length: 1184,
        },
    ),
    (
        "mlkem-1024-pub",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PublicKey,
            alg: "ML-KEM-1024",
            codec: &[0x8d, 0x24],
            key_length: 1568,
        },
    ),
    (
        "ed25519-priv",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PrivateKey,
            alg: "Ed25519",
            codec: &[0x80, 0x26],
            key_length: 32,
        },
    ),
    (
        "secp256k1-priv",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PrivateKey,
            alg: "secp256k1",
            codec: &[0x81, 0x26],
            key_length: 32,
        },
    ),
    (
        "x25519-priv",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PrivateKey,
            alg: "X25519",
            codec: &[0x82, 0x26],
            key_length: 32,
        },
    ),
    (
        "rsa-priv",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PrivateKey,
            alg: "RSA",
            codec: &[0x85, 0x26],
            key_length: VARIABLE_KEY_LENGTH,
        },
    ),
    (
        "p256-priv",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PrivateKey,
            alg: "P-256",
            codec: &[0x86, 0x26],
            key_length: 32,
        },
    ),
    (
        "p384-priv",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PrivateKey,
            alg: "P-384",
            codec: &[0x87, 0x26],
            key_length: 48,
        },
    ),
    (
        "p521-priv",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PrivateKey,
            alg: "P-521",
            codec: &[0x88, 0x26],
            key_length: 66,
        },
    ),
    (
        "ed448-priv",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PrivateKey,
            alg: "Ed448",
            codec: &[0x91, 0x26],
            key_length: 57,
        },
    ),
    (
        "mlkem-512-priv",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PrivateKey,
            alg: "ML-KEM-512",
            codec: &[0x93, 0x26],
            key_length: 1632,
        },
    ),
    (
        "mlkem-768-priv",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PrivateKey,
            alg: "ML-KEM-768",
            codec: &[0x94, 0x26],
            key_length: 2400,
        },
    ),
    (
        "mlkem-1024-priv",
        CodecSpec {
            tag: CodecTag::Key,
            key_material: KeyMaterialKind::PrivateKey,
            alg: "ML-KEM-1024",
            codec: &[0x95, 0x26],
            key_length: 3168,
        },
    ),
    (
        "aes-gcm-256",
        CodecSpec {
            tag: CodecTag::Encryption,
            key_material: KeyMaterialKind::NotKey,
            alg: "AES-256-GCM",
            codec: &[0x80, 0x40],
            key_length: FIXED_LENGTH_NOT_APPLICABLE,
        },
    ),
    (
        "chacha20-poly1305",
        CodecSpec {
            tag: CodecTag::Multikey,
            key_material: KeyMaterialKind::NotKey,
            alg: "ChaCha20-Poly1305",
            codec: &[0x80, 0xc0, 0x02],
            key_length: FIXED_LENGTH_NOT_APPLICABLE,
        },
    ),
];
