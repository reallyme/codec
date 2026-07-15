// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_multibase::bytes_to_multibase58btc;
use codec_multicodec::{KeyMaterialKind, MULTICODEC_TABLE, VARIABLE_KEY_LENGTH};

use crate::error::{CodecNameReason, MultikeyError};

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

    if spec.key_length == VARIABLE_KEY_LENGTH && public_key.is_empty() {
        return Err(MultikeyError::KeyLengthMismatch {
            codec_name: canonical_codec_name,
            expected: spec.key_length,
            actual: public_key.len(),
        });
    }

    if *canonical_codec_name == "rsa-pub" && public_key.len() > MAX_RSA_PUBLIC_KEY_DER_LEN {
        return Err(MultikeyError::KeyTooLarge {
            codec_name: canonical_codec_name,
            max: MAX_RSA_PUBLIC_KEY_DER_LEN,
            actual: public_key.len(),
        });
    }

    if spec.key_length != VARIABLE_KEY_LENGTH && public_key.len() != spec.key_length {
        return Err(MultikeyError::KeyLengthMismatch {
            codec_name: canonical_codec_name,
            expected: spec.key_length,
            actual: public_key.len(),
        });
    }

    let capacity =
        spec.codec
            .len()
            .checked_add(public_key.len())
            .ok_or(MultikeyError::KeyLengthMismatch {
                codec_name: canonical_codec_name,
                expected: spec.key_length,
                actual: public_key.len(),
            })?;
    let mut payload = Vec::with_capacity(capacity);
    payload.extend_from_slice(spec.codec);
    payload.extend_from_slice(public_key);

    bytes_to_multibase58btc(&payload).map_err(|_| MultikeyError::EncodedPayloadTooLarge)
}
