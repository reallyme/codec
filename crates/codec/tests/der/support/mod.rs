// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(clippy::unwrap_used, reason = "test fixture lengths are bounded")]

pub fn tlv(tag: u8, contents: &[u8]) -> Vec<u8> {
    let mut encoded = vec![tag];
    if contents.len() < 128 {
        encoded.push(u8::try_from(contents.len()).unwrap());
    } else {
        let bytes = contents.len().to_be_bytes();
        let first = bytes.iter().position(|byte| *byte != 0).unwrap();
        let length = &bytes[first..];
        encoded.push(0x80 | u8::try_from(length.len()).unwrap());
        encoded.extend_from_slice(length);
    }
    encoded.extend_from_slice(contents);
    encoded
}

pub fn unsigned(magnitude: &[u8]) -> Vec<u8> {
    let mut value = Vec::new();
    if magnitude.first().is_some_and(|byte| byte & 0x80 != 0) {
        value.push(0);
    }
    value.extend_from_slice(magnitude);
    tlv(0x02, &value)
}

pub fn pair(first: &[u8], second: &[u8]) -> Vec<u8> {
    let mut contents = unsigned(first);
    contents.extend_from_slice(&unsigned(second));
    tlv(0x30, &contents)
}
