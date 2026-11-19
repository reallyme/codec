// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use codec_multicodec::{CodecPrefixError, KeyLength};

use crate::error::{classify_multikey_codec, MultikeyError};

// Preserve the multikey API's detailed failures while admission is shared with
// raw multicodec payloads. This adapter classifies errors; it never revalidates.
pub(crate) fn map_public_key_error(
    codec_name: &str,
    policy: KeyLength,
    length: usize,
    error: CodecPrefixError,
) -> MultikeyError {
    match error {
        CodecPrefixError::InvalidPrefix => MultikeyError::UnknownCodecPrefix,
        CodecPrefixError::NonPublicKeyMaterial => MultikeyError::NonPublicKeyMaterial,
        CodecPrefixError::InvalidLength => match policy {
            KeyLength::Fixed(expected) => MultikeyError::KeyLengthMismatch {
                codec: classify_multikey_codec(codec_name),
                expected,
                actual: length,
            },
            KeyLength::Variable => MultikeyError::EmptyKey,
            KeyLength::NotApplicable => MultikeyError::NonPublicKeyMaterial,
        },
        CodecPrefixError::InvalidCompressedPoint => MultikeyError::InvalidCompressedPoint,
        CodecPrefixError::KeyTooLarge { max, actual } => MultikeyError::KeyTooLarge {
            codec: classify_multikey_codec(codec_name),
            max,
            actual,
        },
    }
}
