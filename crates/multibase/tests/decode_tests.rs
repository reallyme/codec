// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Decoder robustness: multibase decoding must reject malformed input with
//! a typed error and never panic, including on inputs whose first
//! character is multi-byte (a byte-index split would panic off a UTF-8
//! character boundary).

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codec_multibase::{multibase_to_bytes, MultibaseError};

#[test]
fn multibyte_first_char_does_not_panic() {
    // "é" is two bytes (C3 A9). A byte-index `split_at(1)` would panic
    // mid-character; the decoder must instead reject it cleanly. A lone
    // multi-byte character has no data after the prefix; the point of the test
    // is that it returns an error rather than panicking.
    assert!(matches!(
        multibase_to_bytes("é"),
        Err(MultibaseError::UnsupportedPrefix)
    ));
    // Multi-byte prefix followed by data: unsupported prefix, no panic.
    assert!(matches!(
        multibase_to_bytes("€zzz"),
        Err(MultibaseError::UnsupportedPrefix)
    ));
    // Emoji (4-byte) prefix with trailing data.
    assert!(matches!(
        multibase_to_bytes("🔑abc"),
        Err(MultibaseError::UnsupportedPrefix)
    ));
}

#[test]
fn empty_input_is_rejected_but_empty_multibase_payloads_round_trip() {
    assert!(matches!(
        multibase_to_bytes(""),
        Err(MultibaseError::TooShort)
    ));
    // Empty bytes encode as just the multibase prefix for both currently
    // supported bases, so decoding must mirror both encoders.
    assert_eq!(
        multibase_to_bytes("z").expect("empty base58btc"),
        Vec::new()
    );
    assert_eq!(
        multibase_to_bytes("u").expect("empty base64url"),
        Vec::new()
    );
}

#[test]
fn ascii_unsupported_prefix_rejected() {
    // A single-byte but unsupported prefix.
    assert!(matches!(
        multibase_to_bytes("xdata"),
        Err(MultibaseError::UnsupportedPrefix)
    ));
}

#[test]
fn valid_base64url_prefix_round_trips() {
    // 'u' is the multibase code for base64url (no padding).
    let decoded = multibase_to_bytes("uaGVsbG8").expect("valid base64url multibase");
    assert_eq!(decoded, b"hello");
}
