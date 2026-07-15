// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// Reason a codec name could not be resolved, used in error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecNameReason {
    /// The codec name is not one of the supported multicodec names.
    Unsupported,
}

impl core::fmt::Display for CodecNameReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let detail = match self {
            CodecNameReason::Unsupported => "unsupported codec name",
        };
        write!(f, "{detail}")
    }
}

/// Classified binding-type label, used for stable error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingTypeKind {
    /// Generic `Multikey` binding, allowing any supported algorithm.
    Multikey,
    /// `P256Key2024` profile-specific binding.
    P256Key2024,
    /// `P384Key2024` profile-specific binding.
    P384Key2024,
    /// `P521Key2024` profile-specific binding.
    P521Key2024,
    /// `RsaVerificationKey2024` profile-specific binding.
    RsaVerificationKey2024,
    /// `ML_DSA_44Key2024` profile-specific binding.
    MlDsa44Key2024,
    /// `ML_DSA_65Key2024` profile-specific binding.
    MlDsa65Key2024,
    /// `ML_DSA_87Key2024` profile-specific binding.
    MlDsa87Key2024,
    /// `MLKEM512Key2024` profile-specific binding.
    MlKem512Key2024,
    /// `MLKEM768Key2024` profile-specific binding.
    MlKem768Key2024,
    /// `MLKEM1024Key2024` profile-specific binding.
    MlKem1024Key2024,
    /// An unrecognized binding-type label.
    Unsupported,
}

impl core::fmt::Display for BindingTypeKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let detail = match self {
            BindingTypeKind::Multikey => "Multikey",
            BindingTypeKind::P256Key2024 => "P256Key2024",
            BindingTypeKind::P384Key2024 => "P384Key2024",
            BindingTypeKind::P521Key2024 => "P521Key2024",
            BindingTypeKind::RsaVerificationKey2024 => "RsaVerificationKey2024",
            BindingTypeKind::MlDsa44Key2024 => "ML_DSA_44Key2024",
            BindingTypeKind::MlDsa65Key2024 => "ML_DSA_65Key2024",
            BindingTypeKind::MlDsa87Key2024 => "ML_DSA_87Key2024",
            BindingTypeKind::MlKem512Key2024 => "MLKEM512Key2024",
            BindingTypeKind::MlKem768Key2024 => "MLKEM768Key2024",
            BindingTypeKind::MlKem1024Key2024 => "MLKEM1024Key2024",
            BindingTypeKind::Unsupported => "unsupported binding type",
        };
        write!(f, "{detail}")
    }
}

/// Classified algorithm label, used for stable error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingAlgorithmKind {
    /// Ed25519 signature algorithm.
    Ed25519,
    /// Ed448 signature algorithm.
    Ed448,
    /// X25519 key-agreement algorithm.
    X25519,
    /// NIST P-256 algorithm.
    P256,
    /// NIST P-384 algorithm.
    P384,
    /// NIST P-521 algorithm.
    P521,
    /// RSA signature verification algorithm.
    Rsa,
    /// secp256k1 algorithm.
    Secp256k1,
    /// ML-DSA-44 post-quantum signature algorithm.
    MlDsa44,
    /// ML-DSA-65 post-quantum signature algorithm.
    MlDsa65,
    /// ML-DSA-87 post-quantum signature algorithm.
    MlDsa87,
    /// ML-KEM-512 post-quantum KEM.
    MlKem512,
    /// ML-KEM-768 post-quantum KEM.
    MlKem768,
    /// ML-KEM-1024 post-quantum KEM.
    MlKem1024,
    /// An unrecognized algorithm label.
    Unsupported,
}

impl core::fmt::Display for BindingAlgorithmKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let detail = match self {
            BindingAlgorithmKind::Ed25519 => "Ed25519",
            BindingAlgorithmKind::Ed448 => "Ed448",
            BindingAlgorithmKind::X25519 => "X25519",
            BindingAlgorithmKind::P256 => "P-256",
            BindingAlgorithmKind::P384 => "P-384",
            BindingAlgorithmKind::P521 => "P-521",
            BindingAlgorithmKind::Rsa => "RSA",
            BindingAlgorithmKind::Secp256k1 => "secp256k1",
            BindingAlgorithmKind::MlDsa44 => "ML-DSA-44",
            BindingAlgorithmKind::MlDsa65 => "ML-DSA-65",
            BindingAlgorithmKind::MlDsa87 => "ML-DSA-87",
            BindingAlgorithmKind::MlKem512 => "ML-KEM-512",
            BindingAlgorithmKind::MlKem768 => "ML-KEM-768",
            BindingAlgorithmKind::MlKem1024 => "ML-KEM-1024",
            BindingAlgorithmKind::Unsupported => "unsupported algorithm",
        };
        write!(f, "{detail}")
    }
}

/// Error returned when parsing, encoding, or validating a multikey fails.
///
/// Parsing fails closed: malformed input yields one of these variants.
#[derive(Debug, Error)]
pub enum MultikeyError {
    /// The input is not a valid multibase string.
    #[error("multikey: invalid multibase string")]
    InvalidMultibase,

    /// The decoded bytes are too short to hold a codec prefix and key.
    #[error("multikey: decoded key too short: length={0}")]
    DecodedTooShort(usize),

    /// The multicodec prefix is not a recognized key type.
    #[error("multikey: unknown multicodec prefix")]
    UnknownCodecPrefix,

    /// The given codec name is not one of the supported names.
    #[error("multikey: unknown codec name: {reason}")]
    UnknownCodecName {
        /// Which class of unknown codec name was supplied.
        reason: CodecNameReason,
    },

    /// The key length does not match the length required by the codec.
    #[error("multikey: key length mismatch for {codec_name}: expected {expected}, got {actual}")]
    KeyLengthMismatch {
        /// Canonical codec name whose length requirement was violated.
        codec_name: &'static str,
        /// Key length in bytes the codec requires.
        expected: usize,
        /// Key length in bytes that was supplied.
        actual: usize,
    },

    /// A variable-length key exceeded this crate's semantic safety limit.
    #[error("multikey: key too large for {codec_name}: maximum {max}, got {actual}")]
    KeyTooLarge {
        /// Canonical codec name whose length limit was violated.
        codec_name: &'static str,
        /// Maximum accepted key length in bytes.
        max: usize,
        /// Key length in bytes that was supplied.
        actual: usize,
    },

    /// The final multikey payload exceeded the base58btc safety limit.
    #[error("multikey: encoded payload too large")]
    EncodedPayloadTooLarge,

    /// The binding type is incompatible with the key's multicodec.
    #[error(
        "multikey: binding type '{binding_type}' does not match codec '{codec_name}' (alg={alg})"
    )]
    BindingTypeCodecMismatch {
        /// Declared binding type.
        binding_type: BindingTypeKind,
        /// Canonical codec name implied by the key.
        codec_name: &'static str,
        /// Algorithm string implied by the codec.
        alg: &'static str,
    },

    /// The declared binding algorithm disagrees with the codec's algorithm.
    #[error(
        "multikey: algorithm mismatch: binding.algorithm='{binding_alg}' but codec implies '{codec_alg}'"
    )]
    BindingAlgorithmMismatch {
        /// Algorithm declared in the binding.
        binding_alg: BindingAlgorithmKind,
        /// Algorithm string implied by the codec.
        codec_alg: &'static str,
    },

    /// A required explicit algorithm was omitted for this binding type.
    #[error(
        "multikey: binding.algorithm missing but binding type '{binding_type}' requires an explicit algorithm"
    )]
    BindingAlgorithmMissing {
        /// Binding type that requires an explicit algorithm.
        binding_type: BindingTypeKind,
    },
}

pub(crate) fn classify_binding_type(binding_type: &str) -> BindingTypeKind {
    match binding_type {
        "Multikey" => BindingTypeKind::Multikey,
        "P256Key2024" => BindingTypeKind::P256Key2024,
        "P384Key2024" => BindingTypeKind::P384Key2024,
        "P521Key2024" => BindingTypeKind::P521Key2024,
        "RsaVerificationKey2024" => BindingTypeKind::RsaVerificationKey2024,
        "ML_DSA_44Key2024" => BindingTypeKind::MlDsa44Key2024,
        "ML_DSA_65Key2024" => BindingTypeKind::MlDsa65Key2024,
        "ML_DSA_87Key2024" => BindingTypeKind::MlDsa87Key2024,
        "MLKEM512Key2024" => BindingTypeKind::MlKem512Key2024,
        "MLKEM768Key2024" => BindingTypeKind::MlKem768Key2024,
        "MLKEM1024Key2024" => BindingTypeKind::MlKem1024Key2024,
        _ => BindingTypeKind::Unsupported,
    }
}

pub(crate) fn classify_binding_algorithm(algorithm: &str) -> BindingAlgorithmKind {
    match algorithm {
        "Ed25519" => BindingAlgorithmKind::Ed25519,
        "Ed448" => BindingAlgorithmKind::Ed448,
        "X25519" => BindingAlgorithmKind::X25519,
        "P-256" => BindingAlgorithmKind::P256,
        "P-384" => BindingAlgorithmKind::P384,
        "P-521" => BindingAlgorithmKind::P521,
        "RSA" => BindingAlgorithmKind::Rsa,
        "secp256k1" => BindingAlgorithmKind::Secp256k1,
        "ML-DSA-44" => BindingAlgorithmKind::MlDsa44,
        "ML-DSA-65" => BindingAlgorithmKind::MlDsa65,
        "ML-DSA-87" => BindingAlgorithmKind::MlDsa87,
        "ML-KEM-512" => BindingAlgorithmKind::MlKem512,
        "ML-KEM-768" => BindingAlgorithmKind::MlKem768,
        "ML-KEM-1024" => BindingAlgorithmKind::MlKem1024,
        _ => BindingAlgorithmKind::Unsupported,
    }
}
