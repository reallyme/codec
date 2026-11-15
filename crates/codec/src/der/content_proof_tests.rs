// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{validate_bit_string_padding, validate_oid, DerError, DerErrorReason};

#[kani::proof]
fn der_bit_string_padding_requires_zero_low_bits() {
    let last: u8 = kani::any();
    let unused: u8 = kani::any();
    let expected = if unused >= 8 {
        Err(DerErrorReason::InvalidValue)
    } else if u16::from(last) % (1_u16 << unused) != 0 {
        Err(DerErrorReason::NonCanonicalEncoding)
    } else {
        Ok(())
    };
    assert_eq!(
        validate_bit_string_padding(last, unused).map_err(DerError::reason),
        expected
    );
    kani::cover!(unused == 0 && last == 0xff, "all bits are used");
    kani::cover!(unused == 7 && last == 0x80, "only the high bit is used");
    kani::cover!(unused == 7 && last == 0x81, "nonzero padding");
    kani::cover!(unused == 8, "invalid padding count");
}

#[kani::proof]
#[kani::unwind(14)]
fn der_oid_contents_require_minimal_terminated_subidentifiers() {
    let bytes: [u8; 12] = kani::any();
    let supplied: u8 = kani::any();
    kani::assume(usize::from(supplied) <= bytes.len());
    let input = &bytes[..usize::from(supplied)];

    // Each subidentifier begins at offset zero or after a terminating
    // octet. A redundant zero group at either position is noncanonical.
    let redundant = input.first() == Some(&0x80)
        || input
            .windows(2)
            .any(|pair| pair[0] < 0x80 && pair[1] == 0x80);
    let expected = if redundant {
        Err(DerErrorReason::NonCanonicalEncoding)
    } else if input.last().is_none_or(|&last| last >= 0x80) {
        Err(DerErrorReason::InvalidValue)
    } else {
        Ok(())
    };
    let actual = validate_oid(input).map_err(DerError::reason);
    assert_eq!(actual, expected);

    kani::cover!(input.is_empty(), "empty object identifier");
    kani::cover!(input == [0x2a, 0x81], "unterminated later subidentifier");
    kani::cover!(
        input == [0x2a, 0x80],
        "nonminimal later subidentifier precedes truncation"
    );
    kani::cover!(
        input == [0x81, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01],
        "complete first subidentifier beyond uint64"
    );
}
