// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

const LOWER_HEX: &[u8; 16] = b"0123456789abcdef";

/// Encode bytes as canonical lowercase hexadecimal.
pub fn bytes_to_lower_hex(bytes: &[u8]) -> String {
    let mut output = String::new();
    write_lower_hex(bytes, &mut output);
    output
}

/// Append canonical lowercase hexadecimal to an existing string buffer.
pub fn write_lower_hex(bytes: &[u8], output: &mut String) {
    for byte in bytes {
        let high = usize::from(byte >> 4);
        let low = usize::from(byte & 0x0f);
        output.push(char::from(LOWER_HEX[high]));
        output.push(char::from(LOWER_HEX[low]));
    }
}
