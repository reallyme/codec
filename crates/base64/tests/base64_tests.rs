// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]
use codec_base64::{base64_to_bytes, bytes_to_base64, Base64Error};

#[test]
fn rfc4648_section_10_known_answer_vectors() -> Result<(), Base64Error> {
    let cases: &[(&[u8], &str)] = &[
        (b"", ""),
        (b"f", "Zg=="),
        (b"fo", "Zm8="),
        (b"foo", "Zm9v"),
        (b"foob", "Zm9vYg=="),
        (b"fooba", "Zm9vYmE="),
        (b"foobar", "Zm9vYmFy"),
    ];

    for (plain, encoded) in cases {
        assert_eq!(bytes_to_base64(plain), *encoded);
        assert_eq!(base64_to_bytes(encoded)?, *plain);
    }
    Ok(())
}

#[test]
fn roundtrip_simple() -> Result<(), Base64Error> {
    let data = b"hello world";
    let b64 = bytes_to_base64(data);
    let out = base64_to_bytes(&b64)?;
    assert_eq!(out, data);
    Ok(())
}

#[test]
fn preserves_padding() -> Result<(), Base64Error> {
    // "hello" -> aGVsbG8=
    let data = b"hello";
    let b64 = bytes_to_base64(data);
    assert!(b64.ends_with('='));
    let out = base64_to_bytes(&b64)?;
    assert_eq!(out, data);
    Ok(())
}

#[test]
fn binary_data_roundtrip() -> Result<(), Base64Error> {
    let data = [0x00, 0xff, 0x10, 0x80, 0x42];
    let b64 = bytes_to_base64(&data);
    let out = base64_to_bytes(&b64)?;
    assert_eq!(out, data);
    Ok(())
}

#[test]
fn rejects_invalid_characters() {
    assert!(base64_to_bytes("not base64!!!").is_err());
}

#[test]
fn rejects_url_safe_alphabet() {
    // '-' and '_' are invalid in standard base64
    assert!(base64_to_bytes("aGVsbG8-").is_err());
    assert!(base64_to_bytes("aGVsbG8_").is_err());
}

#[test]
fn empty_roundtrip() -> Result<(), Base64Error> {
    let data: &[u8] = b"";
    let b64 = bytes_to_base64(data);
    let out = base64_to_bytes(&b64)?;
    assert_eq!(out, data);
    Ok(())
}

#[test]
fn deterministic_encoding() {
    let data = b"deterministic";
    let b1 = bytes_to_base64(data);
    let b2 = bytes_to_base64(data);
    assert_eq!(b1, b2);
}

#[test]
fn rejects_whitespace() {
    // Base64 decoders should not accept embedded whitespace
    assert!(base64_to_bytes("aGVs bG8=").is_err());
    assert!(base64_to_bytes("aGVsbG8=\n").is_err());
}

#[test]
fn rejects_missing_padding_and_non_canonical_lengths() {
    assert!(base64_to_bytes("Zg").is_err());
    assert!(base64_to_bytes("Zm8").is_err());
    assert!(base64_to_bytes("Z").is_err());
}

#[test]
fn rejects_non_canonical_trailing_bits() {
    // "Zg==" is canonical for "f"; changing the second sextet to "h"
    // preserves the decoded byte in lax decoders but sets non-zero pad bits.
    assert!(base64_to_bytes("Zh==").is_err());
}
