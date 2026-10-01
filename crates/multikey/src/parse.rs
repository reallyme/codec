// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use codec_multibase::multibase_to_bytes;
use codec_multicodec::{lookup_codec_prefix, KeyLength, KeyMaterialKind};

use crate::encode::MAX_RSA_PUBLIC_KEY_DER_LEN;
use crate::error::{classify_multikey_codec, MultikeyError};
use zeroize::Zeroizing;

/// A multikey decoded into its codec metadata and raw public key bytes.
///
/// Its metadata cannot be replaced after parsing: binding validation relies
/// on the codec and algorithm being the ones established by the parser.
///
/// ```compile_fail
/// use codec_multikey::parse_multikey;
///
/// if let Ok(mut parsed) = parse_multikey("invalid") {
///     parsed.alg = "Ed25519";
/// }
/// ```
pub struct ParsedMultikey {
    /// Canonical multicodec name of the key type (e.g. `ed25519-pub`).
    codec_name: &'static str,
    /// Human-readable algorithm name implied by the codec (e.g. `Ed25519`).
    alg: &'static str,
    /// Raw public key bytes with the multicodec prefix stripped.
    public_key: Vec<u8>,
    /// Expected public key length for the codec.
    key_length: KeyLength,
}

impl ParsedMultikey {
    /// Returns the canonical multicodec name established by parsing.
    #[must_use]
    pub const fn codec_name(&self) -> &'static str {
        self.codec_name
    }

    /// Returns the algorithm name associated with the parsed codec.
    #[must_use]
    pub const fn algorithm_name(&self) -> &'static str {
        self.alg
    }

    /// Borrows the validated public key bytes.
    #[must_use]
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    /// Returns the length policy associated with the parsed codec.
    #[must_use]
    pub const fn key_length(&self) -> KeyLength {
        self.key_length
    }

    /// Transfers ownership of the validated public key bytes.
    #[must_use]
    pub fn into_public_key(self) -> Vec<u8> {
        self.public_key
    }
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
    let raw = Zeroizing::new(
        multibase_to_bytes(multibase_key).map_err(|_| MultikeyError::InvalidMultibase)?,
    );

    if raw.len() < 2 {
        return Err(MultikeyError::DecodedTooShort(raw.len()));
    }

    // 2) multicodec prefix lookup
    let found = lookup_codec_prefix(&raw).ok_or(MultikeyError::UnknownCodecPrefix)?;

    if found.key_material != KeyMaterialKind::PublicKey {
        return Err(MultikeyError::NonPublicKeyMaterial);
    }

    let public_key = raw[found.codec.len()..].to_vec();

    // 3) key length validation
    match found.key_length {
        KeyLength::Fixed(expected) if public_key.len() != expected => {
            return Err(MultikeyError::KeyLengthMismatch {
                codec: classify_multikey_codec(found.name),
                expected,
                actual: public_key.len(),
            });
        }
        KeyLength::Variable if public_key.is_empty() => return Err(MultikeyError::EmptyKey),
        KeyLength::NotApplicable => return Err(MultikeyError::NonPublicKeyMaterial),
        KeyLength::Fixed(_) | KeyLength::Variable => {}
    }

    if found.name == "rsa-pub" && public_key.len() > MAX_RSA_PUBLIC_KEY_DER_LEN {
        return Err(MultikeyError::KeyTooLarge {
            codec: classify_multikey_codec(found.name),
            max: MAX_RSA_PUBLIC_KEY_DER_LEN,
            actual: public_key.len(),
        });
    }

    if matches!(
        found.name,
        "p256-pub" | "p384-pub" | "p521-pub" | "secp256k1-pub"
    ) && !matches!(public_key.first(), Some(0x02 | 0x03))
    {
        return Err(MultikeyError::InvalidCompressedPoint);
    }

    Ok(ParsedMultikey {
        codec_name: found.name,
        alg: found.alg,
        public_key,
        key_length: found.key_length,
    })
}
