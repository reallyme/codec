// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{validate_disjoint_ranges, validate_write_pair};
use crate::status::CODEC_INVALID_ARGUMENT;
use core::ptr;

#[kani::proof]
fn byte_range_validation_matches_address_and_slice_limits() {
    let address: usize = kani::any();
    let length: usize = kani::any();
    // These pointers are never dereferenced. The address check has no access
    // to the caller's allocation, provenance, or aliasing guarantees.
    let result = validate_write_pair(ptr::without_provenance_mut(address), length);
    let valid = valid_byte_range(address, length);
    kani::cover!(
        address == 0 && length == 0 && result.is_ok(),
        "empty null range"
    );
    kani::cover!(
        address == usize::MAX && length == 1 && result.is_err(),
        "wrapping endpoint"
    );
    assert_eq!(
        result,
        if valid {
            Ok(())
        } else {
            Err(CODEC_INVALID_ARGUMENT)
        }
    );
}

#[kani::proof]
fn disjoint_byte_ranges_match_half_open_interval_specification() {
    let read: usize = kani::any();
    let read_len: usize = kani::any();
    let write: usize = kani::any();
    let write_len: usize = kani::any();
    let result = validate_disjoint_ranges(
        ptr::without_provenance(read),
        read_len,
        ptr::without_provenance_mut(write),
        write_len,
    );

    // For two nonempty intervals, the first byte of the later-starting
    // interval witnesses any overlap. Checking that byte models the rule
    // independently of the implementation's two end comparisons.
    let later_start = wide(read.max(write));
    let overlaps =
        later_start - wide(read) < wide(read_len) && later_start - wide(write) < wide(write_len);
    let allowed =
        valid_byte_range(read, read_len) && valid_byte_range(write, write_len) && !overlaps;
    kani::cover!(
        read_len > 0
            && write_len > 0
            && wide(read) + wide(read_len) == wide(write)
            && result.is_ok(),
        "adjacent ranges"
    );
    kani::cover!(
        valid_byte_range(read, read_len)
            && valid_byte_range(write, write_len)
            && overlaps
            && result.is_err(),
        "overlapping ranges"
    );
    assert_eq!(
        result,
        if allowed {
            Ok(())
        } else {
            Err(CODEC_INVALID_ARGUMENT)
        }
    );
}

fn valid_byte_range(address: usize, length: usize) -> bool {
    length == 0
        || (address != 0
            && wide(length) < (1_u128 << (usize::BITS - 1))
            && wide(address) + wide(length) <= wide(usize::MAX))
}

fn wide(value: usize) -> u128 {
    u128::from(u64::try_from(value).unwrap())
}
