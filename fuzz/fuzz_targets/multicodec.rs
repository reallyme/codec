// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuzz multicodec prefix lookup and fail-closed public-key stripping.

#![no_main]

use codec_multicodec::{CodecPrefixError, KeyLength, KeyMaterialKind};
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
            if valid_length {
                assert_eq!(result, Ok(remainder));
            } else {
                assert_eq!(result, Err(CodecPrefixError::InvalidLength));
            }
        }
    }
});
