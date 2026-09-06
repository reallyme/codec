// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(missing_docs)]
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unwrap_used
)]
use codec_base64url::{base64url_bytes_to_bytes, base64url_to_bytes, bytes_to_base64url};

#[test]
fn roundtrip_unpadded() {
    let data = b"hello world";
    let s = bytes_to_base64url(data);
    let out = base64url_to_bytes(&s).unwrap();
    assert_eq!(out, data);
}

#[test]
fn decode_with_padding_is_rejected() {
    // Standard Base64 with padding is NOT valid Base64URL
    let s = "aGVsbG8=";
    assert!(base64url_to_bytes(s).is_err());
}

#[test]
fn decode_rejects_standard_base64_alphabet() {
    assert!(base64url_to_bytes("aGVsbG8+").is_err());
    assert!(base64url_to_bytes("aGVsbG8/").is_err());
    assert!(base64url_bytes_to_bytes(b"aGVsbG8+").is_err());
    assert!(base64url_bytes_to_bytes(b"aGVsbG8/").is_err());
}

#[test]
fn decode_rejects_non_canonical_trailing_bits() {
    // "AA" is the canonical encoding of one zero byte. "AB" carries non-zero
    // trailing bits and must not decode to the same bytes.
    assert_eq!(base64url_to_bytes("AA").unwrap(), vec![0x00]);
    assert!(base64url_to_bytes("AB").is_err());

    // "AAA" is the canonical encoding of two zero bytes. "AAB" has non-zero
    // trailing bits in the final sextet.
    assert_eq!(base64url_to_bytes("AAA").unwrap(), vec![0x00, 0x00]);
    assert!(base64url_to_bytes("AAB").is_err());
}

#[test]
fn decode_rejects_invalid_single_character_length() {
    assert!(base64url_to_bytes("A").is_err());
    assert!(base64url_bytes_to_bytes(b"A").is_err());
}

#[test]
fn decode_without_padding() {
    let s = "aGVsbG8"; // "hello" without padding
    let out = base64url_to_bytes(s).unwrap();
    assert_eq!(out, b"hello");
}

#[test]
fn decode_from_ascii_bytes_without_utf8_bridge() {
    let segment = b"aGVsbG8";
    let out = base64url_bytes_to_bytes(segment).unwrap();
    assert_eq!(out, b"hello");
}

#[test]
fn rejects_invalid() {
    assert!(base64url_to_bytes("$$$").is_err());
    assert!(base64url_bytes_to_bytes(b"$$$").is_err());
}

#[test]
fn roundtrip_random_bytes() {
    // Deterministic high-entropy data that will produce '-' and '_'
    let data: Vec<u8> = (0u8..=255u8).collect();

    let encoded = bytes_to_base64url(&data);

    // Must contain URL-safe characters in practice
    assert!(
        encoded.contains('-') || encoded.contains('_'),
        "expected base64url output to contain '-' or '_'"
    );

    let decoded = base64url_to_bytes(&encoded).unwrap();

    assert_eq!(decoded, data);
}

#[test]
fn rejects_late_decode_errors_without_returning_partial_bytes() {
    let prefix = "YWJj".repeat(1024);
    for suffix in ["!!!!", "Zh", "Zg==", "Z"] {
        assert!(matches!(
            base64url_to_bytes(&(prefix.clone() + suffix)),
            Err(codec_base64url::Base64UrlError::Invalid)
        ));
    }
}

#[test]
fn owned_decode_matches_original_backend_for_every_two_byte_input() {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    // Enumerate canonical tails, nonzero pad bits, and non-UTF-8 byte input.
    for first in 0..=u8::MAX {
        for second in 0..=u8::MAX {
            let input = [first, second];
            assert_eq!(
                base64url_bytes_to_bytes(&input).ok(),
                URL_SAFE_NO_PAD.decode(input).ok()
            );
        }
    }
    let data: Vec<u8> = (0..=255).collect();
    for length in 0..=data.len() {
        let encoded = URL_SAFE_NO_PAD.encode(&data[..length]);
        for suffix in ["", "=", "!", "A", "AA", "AB", "\n"] {
            let input = encoded.clone() + suffix;
            assert_eq!(
                base64url_to_bytes(&input).ok(),
                URL_SAFE_NO_PAD.decode(&input).ok()
            );
        }
    }
}
