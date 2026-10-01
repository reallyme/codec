// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::HexError;

const LOWER_HEX: &[u8; 16] = b"0123456789abcdef";

/// Encode bytes as canonical lowercase hexadecimal.
pub fn bytes_to_lower_hex(bytes: &[u8]) -> String {
    // A valid slice is at most isize::MAX bytes, so doubling its length fits
    // in usize on every supported target. Reserve once to avoid leaving
    // secret-bearing prefixes in freed String allocations.
    let Some(capacity) = bytes.len().checked_mul(2) else {
        return String::new();
    };
    let mut output = String::with_capacity(capacity);
    append_lower_hex(bytes, &mut output);
    output
}

/// Append canonical lowercase hexadecimal to an existing string buffer.
pub fn write_lower_hex(bytes: &[u8], output: &mut String) -> Result<(), HexError> {
    let additional = bytes
        .len()
        .checked_mul(2)
        .ok_or(HexError::OutputCapacityExceeded)?;
    let _total = output
        .len()
        .checked_add(additional)
        .ok_or(HexError::OutputCapacityExceeded)?;
    // Reserve before writing so growth cannot leave earlier encoded prefixes
    // in discarded allocations when the caller supplied a short buffer.
    output
        .try_reserve_exact(additional)
        .map_err(|_| HexError::OutputCapacityExceeded)?;
    append_lower_hex(bytes, output);
    Ok(())
}

fn append_lower_hex(bytes: &[u8], output: &mut String) {
    for byte in bytes {
        let high = usize::from(byte >> 4);
        let low = usize::from(byte & 0x0f);
        output.push(char::from(LOWER_HEX[high]));
        output.push(char::from(LOWER_HEX[low]));
    }
}
