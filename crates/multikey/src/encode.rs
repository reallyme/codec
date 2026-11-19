// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use codec_multibase::bytes_to_multibase58btc;
use codec_multicodec::{validate_public_key_payload, KeyMaterialKind, MULTICODEC_TABLE};

use crate::error::{CodecNameReason, MultikeyError};
use crate::map_public_key_error::map_public_key_error;

/// Encodes a public key as a multibase (base58btc) multikey string.
///
/// Fails closed on unknown codecs, incorrect lengths, invalid compressed EC
/// tags, or RSA payloads beyond the shared public-key size ceiling.
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

    validate_public_key_payload(canonical_codec_name, public_key).map_err(|error| {
        map_public_key_error(
            canonical_codec_name,
            spec.key_length,
            public_key.len(),
            error,
        )
    })?;

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
