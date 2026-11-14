// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{descend, read_argument, CborError, MAX_NESTING_DEPTH};

#[kani::proof]
#[kani::unwind(5)]
fn extended_argument_requires_the_shortest_encoding() {
    let additional: u8 = kani::any();
    let value: u64 = kani::any();
    let available: usize = kani::any();
    let width = match additional {
        24 => 1,
        25 => 2,
        26 => 4,
        27 => 8,
        _ => return,
    };
    let encoded = value.to_be_bytes();
    let argument = &encoded[encoded.len() - width..];
    kani::assume(available <= width);
    // The leading byte partitions the extended widths: one-byte arguments
    // start at 24, and wider arguments cannot have their high half all zero.
    let shortest = if width == 1 {
        argument[0] >= 24
    } else {
        argument[..width / 2].iter().any(|&byte| byte != 0)
    };
    let actual = read_argument(&argument[..available], 0, additional);
    kani::cover!(matches!(actual, Ok((_, 8))), "eight-byte argument");
    kani::cover!(
        actual == Err(CborError::NonCanonicalInteger),
        "overlong argument"
    );
    kani::cover!(
        actual == Err(CborError::TruncatedArgument),
        "truncated argument"
    );
    if available < width {
        assert_eq!(actual, Err(CborError::TruncatedArgument));
    } else if shortest {
        let decoded = value & (u64::MAX >> ((encoded.len() - width) * 8));
        assert_eq!(actual, Ok((decoded, width)));
    } else {
        assert_eq!(actual, Err(CborError::NonCanonicalInteger));
    }
}

#[kani::proof]
fn descent_enforces_the_depth_limit_without_wrapping() {
    let depth: usize = kani::any();
    let actual = descend(depth);
    kani::cover!(
        depth == MAX_NESTING_DEPTH - 1 && actual == Ok(MAX_NESTING_DEPTH),
        "depth reaches ceiling"
    );
    kani::cover!(
        depth == MAX_NESTING_DEPTH && actual == Err(CborError::DepthExceeded),
        "depth ceiling exceeded"
    );
    assert_eq!(
        actual,
        match depth {
            usize::MAX => Err(CborError::OffsetOverflow),
            _ if depth >= MAX_NESTING_DEPTH => Err(CborError::DepthExceeded),
            _ => Ok(depth + 1),
        }
    );
}
