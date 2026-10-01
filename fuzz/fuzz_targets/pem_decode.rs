// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]

use codec_pem::{decode_pem, encode_pem, PemDecodePolicy, PemEncodeOptions};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = core::str::from_utf8(data) {
        if let Ok(document) = decode_pem(text, PemDecodePolicy::default()) {
            if let Ok(encoded) =
                encode_pem(document.label, &document.der, PemEncodeOptions::default())
            {
                let repeated = decode_pem(&encoded, PemDecodePolicy::default());
                assert!(
                    matches!(repeated, Ok(ref value) if value.label == document.label && value.der == document.der)
                );
            }
        }
    }
});
