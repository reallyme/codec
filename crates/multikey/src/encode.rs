// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use codec_multibase::bytes_to_multibase58btc;
use codec_multicodec::{KeyLength, KeyMaterialKind, MULTICODEC_TABLE};

use crate::error::{classify_multikey_codec, CodecNameReason, MultikeyError};

/// Maximum accepted RSA public-key DER payload in a multikey.
///
/// The supported RSA multicodec is variable-length because DER size depends on
/// modulus size and encoding details. A 4 KiB cap leaves room for common public
/// keys while preventing unbounded base58 and FFI amplification through RSA.
pub const MAX_RSA_PUBLIC_KEY_DER_LEN: usize = 4 * 1024;

/// Encodes a public key as a multibase (base58btc) multikey string.
///
/// Fails closed: returns an error if the codec name is unknown or the key
/// length does not match the codec.
pub fn encode_multikey(codec_name: &str, public_key: &[u8]) -> Result<String, MultikeyError> {
    let (canonical_codec_name, spec) = MULTICODEC_TABLE
        .iter()
        .find(|(name, _)| *name == codec_name)
        .ok_or(MultikeyError::UnknownCodecName {
            reason: CodecNameReason::Unsupported,
        })?;

    if spec.key_material != KeyMaterialKind::PublicKey {
        return Err(MultikeyError::UnknownCodecName {
            reason: CodecNameReason::Unsupported,
        });
    }

    match spec.key_length {
        KeyLength::Fixed(expected) if public_key.len() != expected => {
            return Err(MultikeyError::KeyLengthMismatch {
                codec: classify_multikey_codec(canonical_codec_name),
                expected,
                actual: public_key.len(),
            });
        }
        KeyLength::Variable if public_key.is_empty() => return Err(MultikeyError::EmptyKey),
        KeyLength::NotApplicable => {
            return Err(MultikeyError::UnknownCodecName {
                reason: CodecNameReason::Unsupported,
            });
        }
        KeyLength::Fixed(_) | KeyLength::Variable => {}
    }

    if *canonical_codec_name == "rsa-pub" && public_key.len() > MAX_RSA_PUBLIC_KEY_DER_LEN {
        return Err(MultikeyError::KeyTooLarge {
            codec: classify_multikey_codec(canonical_codec_name),
            max: MAX_RSA_PUBLIC_KEY_DER_LEN,
            actual: public_key.len(),
        });
    }

    if matches!(
        *canonical_codec_name,
        "p256-pub" | "p384-pub" | "p521-pub" | "secp256k1-pub"
    ) && !matches!(public_key.first(), Some(0x02 | 0x03))
    {
        return Err(MultikeyError::InvalidCompressedPoint);
    }

    let capacity = spec
        .codec
        .len()
        .checked_add(public_key.len())
        .ok_or(MultikeyError::EncodedPayloadTooLarge)?;
    let mut payload = Vec::with_capacity(capacity);
    payload.extend_from_slice(spec.codec);
    payload.extend_from_slice(public_key);

    bytes_to_multibase58btc(&payload).map_err(|_| MultikeyError::EncodedPayloadTooLarge)
}
