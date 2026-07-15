// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Fuzz the unpadded base64url decoder on arbitrary text.
//! Property: decoding untrusted input must never panic — it either returns
//! bytes or a typed error.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = core::str::from_utf8(data) {
        let _ = codec_base64url::base64url_to_bytes(text);
    }
});
