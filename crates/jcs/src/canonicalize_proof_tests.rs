// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{utf16_cmp, validate_float, JcsError};
use core::cmp::Ordering;

#[kani::proof]
fn binary64_admission_matches_the_interoperable_range() {
    const EXPONENT_MASK: u64 = 0x7ff;
    const FRACTION_BITS: u32 = 52;
    const EXPONENT_BIAS: u64 = 1023;

    let bits: u64 = kani::any();
    let exponent = (bits >> FRACTION_BITS) & EXPONENT_MASK;
    // The exponent identifies NaN and infinity without floating-point operations.
    // The first magnitude beyond the safe integer range is 2^53. All finite
    // binary64 values in that binade or above are exact integers.
    let expected = match exponent {
        EXPONENT_MASK => Err(JcsError::NonFiniteNumber),
        _ if exponent >= EXPONENT_BIAS + 53 => Err(JcsError::IntegerOutsideInteroperableRange),
        _ => Ok(()),
    };
    let actual = validate_float(f64::from_bits(bits));
    kani::cover!(
        bits == 0x433f_ffff_ffff_ffff && actual.is_ok(),
        "largest permitted integer"
    );
    kani::cover!(
        bits == 0xc340_0000_0000_0000 && actual == Err(JcsError::IntegerOutsideInteroperableRange),
        "negative integer beyond range"
    );
    kani::cover!(bits == 1 && actual.is_ok(), "positive subnormal");
    kani::cover!(
        bits == 0x7ff8_0000_0000_0000 && actual == Err(JcsError::NonFiniteNumber),
        "NaN"
    );
    assert_eq!(actual, expected);
}

#[kani::proof]
#[kani::unwind(5)]
fn utf16_order_matches_the_scalar_rank_for_single_character_names() {
    let left: Option<char> = kani::any();
    let right: Option<char> = kani::any();
    let (mut left_bytes, mut right_bytes) = ([0; 4], [0; 4]);
    let left_text = left.map_or("", |scalar| scalar.encode_utf8(&mut left_bytes));
    let right_text = right.map_or("", |scalar| scalar.encode_utf8(&mut right_bytes));
    let actual = utf16_cmp(left_text, right_text);
    let expected = left
        .map(utf16_scalar_rank)
        .cmp(&right.map(utf16_scalar_rank));
    kani::cover!(
        left == Some('\u{10000}') && right == Some('\u{e000}') && actual == Ordering::Less,
        "surrogate pair sorts before upper BMP"
    );
    kani::cover!(
        left == Some('\u{10000}') && right == Some('\u{10001}') && actual == Ordering::Less,
        "low surrogate determines order"
    );
    kani::cover!(
        left.is_none() && right.is_some() && actual == Ordering::Less,
        "empty name sorts first"
    );
    kani::cover!(
        left.is_some() && left == right && actual == Ordering::Equal,
        "identical nonempty names"
    );
    assert_eq!(actual, expected);
}

fn utf16_scalar_rank(scalar: char) -> u32 {
    let code = u32::from(scalar);
    // Surrogate pairs sort in scalar order among themselves, and their first
    // unit sorts between the lower and upper portions of the BMP.
    match code {
        0..=0xd7ff => code,
        0xe000..=0xffff => code + 0x10_0000,
        _ => 0xd800 + (code - 0x1_0000),
    }
}
