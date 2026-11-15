// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    utf16_cmp, validate_float, validate_signed_integer, validate_unsigned_integer, JcsError,
};
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

#[kani::proof]
#[kani::unwind(10)]
fn utf16_order_matches_an_explicit_order_for_two_scalar_names() {
    // Each surrogate pair falls between the ASCII/BMP prefix and the upper
    // BMP. Within a shared first scalar the second scalar determines order.
    const SORTED: [&str; 11] = [
        "a",
        "aa",
        "aé",
        "a\u{10000}",
        "a\u{10001}",
        "a\u{e000}",
        "\u{10000}",
        "\u{10000}a",
        "\u{10000}z",
        "\u{10000}\u{10ffff}",
        "\u{e000}a",
    ];
    let left = usize::from(kani::any::<u8>());
    let right = usize::from(kani::any::<u8>());
    kani::assume(left < SORTED.len() && right < SORTED.len());
    assert_eq!(utf16_cmp(SORTED[left], SORTED[right]), left.cmp(&right));
    kani::cover!(
        left == 6 && right == 7,
        "the shared supplementary prefix ends on the left"
    );
    kani::cover!(
        left == 3 && right == 5,
        "upper BMP sorts after a supplementary suffix"
    );
    kani::cover!(
        left == right && left == 9,
        "two surrogate pairs compare equal"
    );
    kani::cover!(
        left == 8 && right == 10,
        "the first scalar decides before the suffix"
    );
}

#[kani::proof]
fn signed_and_unsigned_integer_admission_matches_the_interoperable_range() {
    const FIRST_UNSAFE: i128 = 1_i128 << 53;
    let signed: i64 = kani::any();
    let unsigned: u64 = kani::any();
    let signed_expected = if -FIRST_UNSAFE < i128::from(signed) && i128::from(signed) < FIRST_UNSAFE
    {
        Ok(())
    } else {
        Err(JcsError::IntegerOutsideInteroperableRange)
    };
    let unsigned_expected = if i128::from(unsigned) < FIRST_UNSAFE {
        Ok(())
    } else {
        Err(JcsError::IntegerOutsideInteroperableRange)
    };
    assert_eq!(validate_signed_integer(signed), signed_expected);
    assert_eq!(validate_unsigned_integer(unsigned), unsigned_expected);
    kani::cover!(
        i128::from(signed) == 1 - FIRST_UNSAFE,
        "smallest interoperable integer"
    );
    kani::cover!(
        i128::from(signed) == -FIRST_UNSAFE,
        "first negative integer outside range"
    );
    kani::cover!(signed == i64::MIN, "minimum signed integer");
    kani::cover!(
        i128::from(unsigned) == FIRST_UNSAFE - 1,
        "largest interoperable integer"
    );
    kani::cover!(
        i128::from(unsigned) == FIRST_UNSAFE,
        "first positive integer outside range"
    );
    kani::cover!(unsigned == u64::MAX, "maximum unsigned integer");
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
