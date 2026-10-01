// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuzz the multikey parser (multibase + multicodec prefix + key binding) on
//! arbitrary text. Property: parsing an untrusted `did:key`-style multikey
//! string must never panic.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = core::str::from_utf8(data) {
        if codec_multikey::parse_multikey(text).is_ok() {
            assert!(codec_multikey::validate_multikey_binding(
                codec_multikey::KeyBindingInput {
                    binding_type: "Multikey",
                    algorithm: None,
                },
                text,
            )
            .is_ok());
        }
    }
});
