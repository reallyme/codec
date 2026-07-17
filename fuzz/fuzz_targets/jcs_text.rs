// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use codec_jcs::canonicalize_json_text;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(json) = core::str::from_utf8(data) {
        let _ = canonicalize_json_text(json);
    }
});
