// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_multibase::multibase_to_bytes;
use codec_multicodec::{lookup_codec_prefix, KeyMaterialKind, VARIABLE_KEY_LENGTH};

use crate::encode::MAX_RSA_PUBLIC_KEY_DER_LEN;
use crate::error::MultikeyError;

/// A multikey decoded into its codec metadata and raw public key bytes.
pub struct ParsedMultikey {
    /// Canonical multicodec name of the key type (e.g. `ed25519-pub`).
    pub codec_name: &'static str,
    /// Human-readable algorithm name implied by the codec (e.g. `Ed25519`).
    pub alg: &'static str,
    /// Raw public key bytes with the multicodec prefix stripped.
    pub public_key: Vec<u8>,
    /// Expected public key length for the codec.
    pub key_length: usize,
}

/// Parses a multibase-encoded multikey string into its codec and key bytes.
///
/// Fails closed: returns an error on non-canonical multibase, unknown codec
/// prefix, or a key length that does not match the codec.
pub fn parse_multikey(multibase_key: &str) -> Result<ParsedMultikey, MultikeyError> {
    if multibase_key.len() < 2 {
        return Err(MultikeyError::InvalidMultibase);
    }
    // Multikey has a single canonical string form: base58btc (`z`).
    // Rejecting alternate multibase alphabets prevents duplicate encodings of
    // the same key from bypassing string-based blocklists or deduplication.
    if !multibase_key.starts_with('z') {
        return Err(MultikeyError::InvalidMultibase);
    }

    // 1) multibase decode
    let raw = multibase_to_bytes(multibase_key).map_err(|_| MultikeyError::InvalidMultibase)?;

    if raw.len() < 2 {
        return Err(MultikeyError::DecodedTooShort(raw.len()));
    }

    // 2) multicodec prefix lookup
    let found = lookup_codec_prefix(&raw).ok_or(MultikeyError::UnknownCodecPrefix)?;

    if found.key_material != KeyMaterialKind::PublicKey {
        return Err(MultikeyError::UnknownCodecPrefix);
    }

    let public_key = raw[found.codec.len()..].to_vec();

    // 3) key length validation
    if found.key_length == VARIABLE_KEY_LENGTH && public_key.is_empty() {
        return Err(MultikeyError::KeyLengthMismatch {
            codec_name: found.name,
            expected: found.key_length,
            actual: public_key.len(),
        });
    }

    if found.name == "rsa-pub" && public_key.len() > MAX_RSA_PUBLIC_KEY_DER_LEN {
        return Err(MultikeyError::KeyTooLarge {
            codec_name: found.name,
            max: MAX_RSA_PUBLIC_KEY_DER_LEN,
            actual: public_key.len(),
        });
    }

    if found.key_length != VARIABLE_KEY_LENGTH && public_key.len() != found.key_length {
        return Err(MultikeyError::KeyLengthMismatch {
            codec_name: found.name,
            expected: found.key_length,
            actual: public_key.len(),
        });
    }

    Ok(ParsedMultikey {
        codec_name: found.name,
        alg: found.alg,
        public_key,
        key_length: found.key_length,
    })
}
