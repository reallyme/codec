// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Fuzz the multicodec prefix parser on arbitrary bytes.
//! Property: prefix lookup/strip over untrusted bytes must never panic and must
//! never read out of bounds (e.g. on truncated varints).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = codec_multicodec::lookup_codec_prefix(data);
    let stripped = codec_multicodec::strip_codec_prefix(data);
    // The stripped view must be a suffix of the input, never longer than it.
    assert!(stripped.len() <= data.len());
});
