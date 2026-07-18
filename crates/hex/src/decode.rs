// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use crate::HexError;

/// Decode canonical lowercase hexadecimal bytes.
pub fn lower_hex_to_bytes(input: &str) -> Result<Vec<u8>, HexError> {
    if !input.len().is_multiple_of(2) {
        return Err(HexError::OddLength);
    }

    let mut output = Vec::with_capacity(input.len() / 2);
    for pair in input.as_bytes().chunks_exact(2) {
        let high = lower_hex_value(pair[0])?;
        let low = lower_hex_value(pair[1])?;
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
