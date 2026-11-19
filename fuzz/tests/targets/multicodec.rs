// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuzz multicodec prefix lookup and fail-closed public-key stripping.

#![no_main]

use codec_multicodec::{CodecPrefixError, KeyLength, KeyMaterialKind, MAX_RSA_PUBLIC_KEY_DER_LEN};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let result = codec_multicodec::strip_codec_prefix(data);
    match codec_multicodec::lookup_codec_prefix(data) {
        None => assert_eq!(result, Err(CodecPrefixError::InvalidPrefix)),
        Some(found) if found.key_material != KeyMaterialKind::PublicKey => {
            assert_eq!(result, Err(CodecPrefixError::NonPublicKeyMaterial));
        }
        Some(found) => {
            let remainder = data
                .get(found.codec.len()..)
                .expect("successful prefix lookup fits the input");
            let valid_length = match found.key_length {
                KeyLength::Fixed(expected) => remainder.len() == expected,
                KeyLength::Variable => !remainder.is_empty(),
                KeyLength::NotApplicable => false,
            };
            if !valid_length {
                assert_eq!(result, Err(CodecPrefixError::InvalidLength));
            } else if found.name == "rsa-pub" && remainder.len() > MAX_RSA_PUBLIC_KEY_DER_LEN {
                assert_eq!(
                    result,
                    Err(CodecPrefixError::KeyTooLarge {
                        max: MAX_RSA_PUBLIC_KEY_DER_LEN,
                        actual: remainder.len(),
                    })
                );
            } else if matches!(
                found.name,
                "p256-pub" | "p384-pub" | "p521-pub" | "secp256k1-pub"
            ) && !matches!(remainder.first(), Some(0x02 | 0x03))
            {
                assert_eq!(result, Err(CodecPrefixError::InvalidCompressedPoint));
            } else {
                assert_eq!(result, Ok(remainder));
            }
        }
    }
});
