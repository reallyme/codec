// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use codec_hex::{bytes_to_lower_hex, lower_hex_to_bytes, write_lower_hex, HexError};

#[test]
fn rfc4648_section_10_base16_known_answer_vectors() -> Result<(), HexError> {
    let cases: &[(&[u8], &str)] = &[
        (b"", ""),
        (b"f", "66"),
        (b"fo", "666f"),
        (b"foo", "666f6f"),
        (b"foob", "666f6f62"),
        (b"fooba", "666f6f6261"),
        (b"foobar", "666f6f626172"),
    ];

    for (plain, encoded) in cases {
        assert_eq!(bytes_to_lower_hex(plain), *encoded);
        assert_eq!(lower_hex_to_bytes(encoded)?, *plain);
    }
    Ok(())
}

#[test]
fn encodes_lowercase_hex() {
    let encoded = bytes_to_lower_hex(&[0x00, 0x01, 0x0f, 0x10, 0xab, 0xff]);

    assert_eq!(encoded, "00010f10abff");
}

#[test]
fn encodes_all_bytes_without_uppercase() {
    let data: Vec<u8> = (0u8..=255u8).collect();

    let encoded = bytes_to_lower_hex(&data);

    assert!(encoded
        .bytes()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')));
    assert_eq!(lower_hex_to_bytes(&encoded), Ok(data));
}

#[test]
fn appends_lowercase_hex() {
    let mut output = String::from("sha256:");

    write_lower_hex(&[0xde, 0xad, 0xbe, 0xef], &mut output);

    assert_eq!(output, "sha256:deadbeef");
}

#[test]
fn decodes_lowercase_hex() -> Result<(), HexError> {
    let decoded = lower_hex_to_bytes("00010f10abff")?;

    assert_eq!(decoded, vec![0x00, 0x01, 0x0f, 0x10, 0xab, 0xff]);
    Ok(())
}

#[test]
fn empty_input_roundtrips() -> Result<(), HexError> {
    assert_eq!(bytes_to_lower_hex(&[]), "");
    assert_eq!(lower_hex_to_bytes("")?, Vec::<u8>::new());
    Ok(())
}

#[test]
fn rejects_odd_length() {
    assert_eq!(lower_hex_to_bytes("abc"), Err(HexError::OddLength));
}

#[test]
fn rejects_uppercase() {
    assert_eq!(lower_hex_to_bytes("AB"), Err(HexError::Uppercase));
    assert_eq!(lower_hex_to_bytes("aB"), Err(HexError::Uppercase));
    assert_eq!(lower_hex_to_bytes("deadBEEF"), Err(HexError::Uppercase));
}

#[test]
fn rejects_non_hex_characters() {
    assert_eq!(lower_hex_to_bytes("0g"), Err(HexError::InvalidCharacter));
    assert_eq!(lower_hex_to_bytes("00\n"), Err(HexError::OddLength));
    assert_eq!(lower_hex_to_bytes("0_"), Err(HexError::InvalidCharacter));
}
