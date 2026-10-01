// SPDX-FileCopyrightText: 2026 ReallyMe LLC
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
});
