// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{lower_hex_value, HexError};

#[kani::proof]
#[kani::unwind(18)]
fn lowercase_hex_digit_admission_matches_the_alphabet() {
    let byte: u8 = kani::any();
    let index = b"0123456789abcdef".iter().position(|digit| *digit == byte);
    let expected = if let Some(index) = index {
        Ok(u8::try_from(index).unwrap())
    } else if b"ABCDEF".contains(&byte) {
        Err(HexError::Uppercase)
    } else {
        Err(HexError::InvalidCharacter)
    };
    let actual = lower_hex_value(byte);
    match (actual, expected) {
        (Ok(actual), Ok(expected)) => assert_eq!(actual, expected),
        (Err(HexError::Uppercase), Err(HexError::Uppercase))
        | (Err(HexError::InvalidCharacter), Err(HexError::InvalidCharacter)) => {}
        _ => kani::assert(false, "the implementation uses the specified alphabet"),
    }
    kani::cover!(byte == b'0', "lowest accepted digit");
    kani::cover!(byte == b'f', "highest accepted digit");
    kani::cover!(byte == b'F', "uppercase digit is rejected");
    kani::cover!(byte > 127, "non-ASCII byte is rejected");
}
