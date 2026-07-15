// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Fuzz the multikey parser (multibase + multicodec prefix + key binding) on
//! arbitrary text. Property: parsing an untrusted `did:key`-style multikey
//! string must never panic.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = core::str::from_utf8(data) {
        let _ = codec_multikey::parse_multikey(text);
    }
});
