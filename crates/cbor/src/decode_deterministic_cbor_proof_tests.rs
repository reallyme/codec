// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{bounded_capacity, read_head, DeterministicCborError};
use crate::check_map_key_order::{MapKeyOrder, MapKeyOrderError};

const MAX_HEADER_BYTES: usize = 9;
const MAX_KEY_BYTES: usize = 8;

#[kani::proof]
#[kani::unwind(9)]
fn map_key_admission_matches_length_first_order() {
    let previous: [u8; MAX_KEY_BYTES] = kani::any();
    let current: [u8; MAX_KEY_BYTES] = kani::any();
    let previous_len: usize = kani::any();
    let current_len: usize = kani::any();
    kani::assume((1..=MAX_KEY_BYTES).contains(&previous_len));
    kani::assume((1..=MAX_KEY_BYTES).contains(&current_len));

    // Equal-length octet strings order like unsigned big-endian integers.
    // Ignore the bytes outside the prefix by shifting them out of each rank.
    let previous_rank = u64::from_be_bytes(previous) >> ((MAX_KEY_BYTES - previous_len) * 8);
    let current_rank = u64::from_be_bytes(current) >> ((MAX_KEY_BYTES - current_len) * 8);
    let expected = if (previous_len, previous_rank) < (current_len, current_rank) {
        Ok(())
    } else if (previous_len, previous_rank) == (current_len, current_rank) {
        Err(MapKeyOrderError::Duplicate)
    } else {
        Err(MapKeyOrderError::Descending)
    };
    let mut order = MapKeyOrder::default();
    assert_eq!(order.admit(&previous[..previous_len]), Ok(()));
    let actual = order.admit(&current[..current_len]);
    kani::cover!(
        previous_len < current_len && previous[0] > current[0] && actual.is_ok(),
        "length takes precedence over the first byte"
    );
    kani::cover!(
        previous_len == current_len && actual.is_ok(),
        "equal-length keys in byte order"
    );
    kani::cover!(actual == Err(MapKeyOrderError::Duplicate), "duplicate key");
    kani::cover!(
        actual == Err(MapKeyOrderError::Descending),
        "descending key order"
    );
    assert_eq!(actual, expected);
}

#[kani::proof]
#[kani::unwind(10)]
fn integer_header_requires_the_shortest_encoding() {
    let mut bytes: [u8; MAX_HEADER_BYTES] = kani::any();
    let available: usize = kani::any();
    kani::assume(available <= bytes.len());
    // Both integer major types share the same unsigned wire argument. Keeping
    // the argument symbolic covers all u64 values, including the sign boundary.
    bytes[0] &= 0x3f;
    let additional = bytes[0] & 0x1f;
    let actual = read_head(&bytes[..available], 0);
    let width = match additional {
        0..=23 => 0,
        24 => 1,
        25 => 2,
        26 => 4,
        27 => 8,
        _ => 0,
    };
    let expected = if available == 0 {
        Err(DeterministicCborError::UnexpectedEnd)
    } else if additional >= 28 {
        Err(DeterministicCborError::UnsupportedAdditionalInfo)
    } else if available <= width {
        Err(DeterministicCborError::TruncatedArgument)
    } else {
        // Interpret the payload as one zero-padded big-endian integer. Its bit
        // length determines the only permitted extended encoding width.
        let end = width + 1;
        let argument = if width == 0 {
            u64::from(additional)
        } else {
            let mut padded = [0; 8];
            padded[8 - width..].copy_from_slice(&bytes[1..end]);
            u64::from_be_bytes(padded)
        };
        let preferred_width = match argument {
            0..=23 => 0,
            _ => (u64::BITS - argument.leading_zeros())
                .div_ceil(8)
                .next_power_of_two(),
        };
        if u32::try_from(width) == Ok(preferred_width) {
            Ok((bytes[0] >> 5, argument, end))
        } else {
            Err(DeterministicCborError::NonCanonicalInteger)
        }
    };
    kani::cover!(
        matches!(actual, Ok((0, _, 9))),
        "eight-byte unsigned argument"
    );
    kani::cover!(
        matches!(actual, Ok((1, _, 9))),
        "eight-byte negative argument"
    );
    kani::cover!(
        actual == Err(DeterministicCborError::NonCanonicalInteger),
        "overlong argument"
    );
    kani::cover!(
        actual == Err(DeterministicCborError::TruncatedArgument),
        "truncated argument"
    );
    assert_eq!(actual, expected);
}

#[kani::proof]
fn container_capacity_fits_remaining_source() {
    let count: usize = kani::any();
    let total: usize = kani::any();
    let offset: usize = kani::any();
    let is_map: bool = kani::any();
    // The array and map callers require at least one and two bytes per
    // entry respectively. Widen the reference arithmetic to avoid wraparound.
    let bytes_per_entry = if is_map { 2 } else { 1 };
    let required_end = wide(count) * wide(bytes_per_entry) + wide(offset);
    let expected = if offset > total {
        Err(DeterministicCborError::OffsetOverflow)
    } else if required_end > wide(total) {
        Err(DeterministicCborError::ContainerLengthExceedsInput)
    } else {
        Ok(())
    };
    let actual = bounded_capacity(count, total, offset, bytes_per_entry);
    kani::cover!(is_map && actual.is_ok(), "map admitted");
    kani::cover!(!is_map && actual.is_ok(), "array admitted");
    kani::cover!(
        actual == Err(DeterministicCborError::ContainerLengthExceedsInput),
        "entry count exceeds remaining bytes"
    );
    assert_eq!(actual, expected);
}

fn wide(value: usize) -> u128 {
    u128::from(u64::try_from(value).unwrap())
}
