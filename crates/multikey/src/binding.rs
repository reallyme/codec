// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use crate::error::{
    classify_binding_algorithm, classify_binding_type, classify_multikey_codec, MultikeyError,
};
use crate::parse::ParsedMultikey;

/// Generic binding compatibility rules.
/// Binding labels are protocol-facing metadata and are validated here
/// as algorithm constraints over multikey-encoded public keys.
/// Returns whether a binding-type label is compatible with a codec name.
///
/// Generic `Multikey` matches any supported codec; profile-specific labels
/// match only their one codec. Unknown labels return `false`.
pub fn binding_type_matches_codec(binding_type: &str, codec_name: &str) -> bool {
    match binding_type {
        // Generic Multikey (ALL supported algorithms)
        "Multikey" => matches!(
            codec_name,
            "ed25519-pub"
                | "ed448-pub"
                | "x25519-pub"
                | "p256-pub"
                | "p384-pub"
                | "p521-pub"
                | "rsa-pub"
                | "secp256k1-pub"
                | "mldsa-44-pub"
                | "mldsa-65-pub"
                | "mldsa-87-pub"
                | "mlkem-512-pub"
                | "mlkem-768-pub"
                | "mlkem-1024-pub"
        ),

        // Profile-specific / constrained bindings
        "P256Key2024" => codec_name == "p256-pub",
        "P384Key2024" => codec_name == "p384-pub",
        "P521Key2024" => codec_name == "p521-pub",
        "RsaVerificationKey2024" => codec_name == "rsa-pub",
        "ML_DSA_44Key2024" => codec_name == "mldsa-44-pub",
        "ML_DSA_65Key2024" => codec_name == "mldsa-65-pub",
        "ML_DSA_87Key2024" => codec_name == "mldsa-87-pub",
        "MLKEM512Key2024" => codec_name == "mlkem-512-pub",
        "MLKEM768Key2024" => codec_name == "mlkem-768-pub",
        "MLKEM1024Key2024" => codec_name == "mlkem-1024-pub",

        _ => false,
    }
}

/// Binding metadata to validate against a parsed multikey.
pub struct KeyBindingInput<'a> {
    /// The binding-type label (e.g. `Multikey`, `P256Key2024`).
    pub binding_type: &'a str,
    /// Optional explicit algorithm label; required for non-`Multikey` types.
    pub algorithm: Option<&'a str>,
}

/// Validates that a binding's type and algorithm agree with a parsed key.
///
/// Fails closed: returns an error on a type/codec mismatch, an algorithm
/// mismatch, or a missing required algorithm.
pub fn validate_key_binding(
    binding: KeyBindingInput<'_>,
    parsed: &ParsedMultikey,
) -> Result<(), MultikeyError> {
    if !binding_type_matches_codec(binding.binding_type, parsed.codec_name) {
        return Err(MultikeyError::BindingTypeCodecMismatch {
            binding_type: classify_binding_type(binding.binding_type),
            codec: classify_multikey_codec(parsed.codec_name),
            algorithm: classify_binding_algorithm(parsed.alg),
        });
    }

    if let Some(binding_alg) = binding.algorithm {
        if binding_alg != parsed.alg {
            return Err(MultikeyError::BindingAlgorithmMismatch {
                binding_alg: classify_binding_algorithm(binding_alg),
                codec_algorithm: classify_binding_algorithm(parsed.alg),
            });
        }
    } else if binding.binding_type != "Multikey" {
        return Err(MultikeyError::BindingAlgorithmMissing {
            binding_type: classify_binding_type(binding.binding_type),
        });
    }

    Ok(())
}
