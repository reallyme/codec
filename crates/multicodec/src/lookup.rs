// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::table::{CodecTag, KeyLength, KeyMaterialKind, MULTICODEC_TABLE};

/// Failure to strip a public-key multicodec prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CodecPrefixError {
    /// No supported prefix was found.
    #[error("invalid multicodec prefix")]
    InvalidPrefix,
    /// The prefix identifies private, symmetric, or non-key material.
    #[error("multicodec does not identify public key material")]
    NonPublicKeyMaterial,
    /// A recognized public-key prefix carries an invalid payload length.
    #[error("invalid multicodec payload length")]
    InvalidLength,
}

/// Codec metadata resolved from a multicodec prefix.
#[derive(Debug, Clone)]
pub struct CodecLookupResult {
    /// Canonical multicodec name (e.g. `ed25519-pub`).
    pub name: &'static str,
    /// Multicodec table tag.
    pub tag: CodecTag,
    /// Key-material class for `key` codecs.
    pub key_material: KeyMaterialKind,
    /// Human-readable algorithm name (e.g. `Ed25519`).
    pub alg: &'static str,
    /// The multicodec varint prefix bytes.
    pub codec: &'static [u8],
    /// Expected raw public key length after the prefix.
    pub key_length: KeyLength,
}

/// Lookup multicodec prefix → codec metadata
pub fn lookup_codec_prefix(bytes: &[u8]) -> Option<CodecLookupResult> {
    for (name, spec) in MULTICODEC_TABLE {
        let prefix = spec.codec;
        if bytes.len() >= prefix.len() && bytes.starts_with(prefix) {
            return Some(CodecLookupResult {
                name,
                tag: spec.tag,
                key_material: spec.key_material,
                alg: spec.alg,
                codec: spec.codec,
                key_length: spec.key_length,
            });
        }
    }
    None
}

/// Strip a recognized public-key multicodec prefix from a complete key.
///
/// # Errors
///
/// Rejects unknown prefixes, non-public key classes, and invalid key lengths
/// so callers cannot accidentally use prefixed or private bytes as raw keys.
pub fn strip_codec_prefix(bytes: &[u8]) -> Result<&[u8], CodecPrefixError> {
    let found = lookup_codec_prefix(bytes).ok_or(CodecPrefixError::InvalidPrefix)?;
    if found.key_material != KeyMaterialKind::PublicKey {
        return Err(CodecPrefixError::NonPublicKeyMaterial);
    }
    let remainder = bytes
        .get(found.codec.len()..)
        .ok_or(CodecPrefixError::InvalidLength)?;
    let valid = match found.key_length {
        KeyLength::Fixed(expected) => remainder.len() == expected,
        KeyLength::Variable => !remainder.is_empty(),
        KeyLength::NotApplicable => false,
    };
    if !valid {
        return Err(CodecPrefixError::InvalidLength);
    }
    Ok(remainder)
}
