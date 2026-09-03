// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use crate::HexError;

/// Decode canonical lowercase hexadecimal bytes.
pub fn lower_hex_to_bytes(input: &str) -> Result<Vec<u8>, HexError> {
    let (pairs, remainder) = input.as_bytes().as_chunks::<2>();
    if !remainder.is_empty() {
        return Err(HexError::OddLength);
    }

    let mut output = Vec::with_capacity(pairs.len());
    for [high_byte, low_byte] in pairs {
        let high = lower_hex_value(*high_byte)?;
        let low = lower_hex_value(*low_byte)?;
        output.push((high << 4) | low);
    }

    Ok(output)
}

fn lower_hex_value(byte: u8) -> Result<u8, HexError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Err(HexError::Uppercase),
        _ => Err(HexError::InvalidCharacter),
    }
}
