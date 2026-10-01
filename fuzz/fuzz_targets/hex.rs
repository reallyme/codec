// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = core::str::from_utf8(data) {
        if let Ok(decoded) = codec_hex::lower_hex_to_bytes(text) {
            assert_eq!(codec_hex::bytes_to_lower_hex(&decoded), text);
        }
    }
});
