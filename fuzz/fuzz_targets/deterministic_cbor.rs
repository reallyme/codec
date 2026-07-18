// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Fuzz the generic deterministic-CBOR decoder on arbitrary bytes.
//!
//! Accepted inputs must be canonical for the documented deterministic-CBOR
//! subset. Re-encoding a decoded value is therefore expected to reproduce the
//! exact byte sequence; any drift would indicate a parser, ordering, or
//! canonicalization bug rather than a caller-specific formatting choice.

#![no_main]

use codec_cbor::{decode_deterministic_cbor, encode_deterministic_cbor};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(value) = decode_deterministic_cbor(data) {
        match encode_deterministic_cbor(&value) {
            Ok(encoded) => {
                assert_eq!(encoded.as_slice(), data);
                match decode_deterministic_cbor(&encoded) {
                    Ok(decoded_again) => match encode_deterministic_cbor(&decoded_again) {
                        Ok(encoded_again) => assert_eq!(encoded_again, encoded),
                        Err(_) => panic!("re-decoded deterministic CBOR failed to re-encode"),
                    },
                    Err(_) => panic!("encoded deterministic CBOR failed to re-decode"),
                }
            }
            Err(_) => panic!("accepted deterministic CBOR failed to re-encode"),
        }
    }
});
