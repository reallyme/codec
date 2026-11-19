// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::lookup::CodecPrefixError;
use crate::table::{KeyLength, KeyMaterialKind, MULTICODEC_TABLE};

/// Maximum accepted opaque RSA public-key DER payload: 4 KiB.
///
/// A semantic ceiling prevents variable-length keys from amplifying encoding
/// and FFI output. This codec layer does not establish DER or key validity.
pub const MAX_RSA_PUBLIC_KEY_DER_LEN: usize = 4_096;
const SEC1_COMPRESSED_EVEN_TAG: u8 = 0x02;
const SEC1_COMPRESSED_ODD_TAG: u8 = 0x03;

/// Validate a registered public-key payload before stripping, encoding, or use.
///
/// Resolves immutable registry metadata so callers cannot weaken validation by
/// supplying a forged length policy. Checks fixed lengths, compressed SEC1 tags,
/// and the nonempty, bounded RSA payload. Cryptographic validity remains the
/// responsibility of the consuming cryptography layer.
///
/// # Errors
///
/// Returns a typed prefix, key-material, length, point-tag, or size failure.
pub fn validate_public_key_payload(
    codec_name: &str,
    payload: &[u8],
) -> Result<(), CodecPrefixError> {
    let (_, spec) = MULTICODEC_TABLE
        .iter()
        .find(|(name, _)| *name == codec_name)
        .ok_or(CodecPrefixError::InvalidPrefix)?;
    if spec.key_material != KeyMaterialKind::PublicKey {
        return Err(CodecPrefixError::NonPublicKeyMaterial);
    }
    validate_key_length(codec_name, spec.key_length, payload.len())?;
    if matches!(
        codec_name,
        "p256-pub" | "p384-pub" | "p521-pub" | "secp256k1-pub"
    ) && !matches!(
        payload.first(),
        Some(&SEC1_COMPRESSED_EVEN_TAG) | Some(&SEC1_COMPRESSED_ODD_TAG)
    ) {
        return Err(CodecPrefixError::InvalidCompressedPoint);
    }
    Ok(())
}

fn validate_key_length(
    codec_name: &str,
    policy: KeyLength,
    length: usize,
) -> Result<(), CodecPrefixError> {
    match policy {
        KeyLength::Fixed(expected) if length != expected => {
            return Err(CodecPrefixError::InvalidLength);
        }
        KeyLength::Variable if length == 0 => return Err(CodecPrefixError::InvalidLength),
        KeyLength::NotApplicable => return Err(CodecPrefixError::NonPublicKeyMaterial),
        KeyLength::Fixed(_) | KeyLength::Variable => {}
    }
    if codec_name == "rsa-pub" && length > MAX_RSA_PUBLIC_KEY_DER_LEN {
        return Err(CodecPrefixError::KeyTooLarge {
            max: MAX_RSA_PUBLIC_KEY_DER_LEN,
            actual: length,
        });
    }
    Ok(())
}

#[cfg(kani)]
#[path = "public_key_length_proof_tests.rs"]
mod proofs;
