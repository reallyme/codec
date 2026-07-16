// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Fuzz the multibase and base58btc decoders on arbitrary text input.
//! Property: decoding untrusted input must never panic — it either returns
//! bytes or a typed error.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = core::str::from_utf8(data) {
        let _ = codec_multibase::multibase_to_bytes(text);
        let _ = codec_multibase::base58btc_decode(text);
    }
});
