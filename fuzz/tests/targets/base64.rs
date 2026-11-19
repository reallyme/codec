// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = core::str::from_utf8(data) {
        if let Ok(decoded) = codec_base64::base64_to_bytes(text) {
            assert_eq!(codec_base64::bytes_to_base64(&decoded), text);
        }
    }
    if let (Some(input), Some(output), Some(encoded)) = (data.get(..8), data.get(8..16), data.get(16..)) {
        if let (Ok(input), Ok(output), Ok(text)) = (
            <[u8; 8]>::try_from(input),
            <[u8; 8]>::try_from(output),
            core::str::from_utf8(encoded),
        ) {
            let limits = codec_base64::Base64DecodeLimits {
                encoded_input_bytes: Some(u64::from_le_bytes(input)),
                decoded_output_bytes: Some(u64::from_le_bytes(output)),
            };
            if let Ok(decoded) = codec_base64::base64_to_bytes_with_limits(text, &limits) {
                assert_eq!(codec_base64::bytes_to_base64(&decoded), text);
            }
        }
    }

});
