// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    validate_disjoint_ranges, validate_input_scalar_output, validate_output_len_pair,
    validate_output_ptr, validate_write_pair,
};
use crate::status::{CODEC_INVALID_ARGUMENT, CODEC_OK};
use core::mem::{align_of, size_of};
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
    let overlaps = ranges_overlap(read, read_len, write, write_len);
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

#[kani::proof]
fn ffi_scalar_outputs_require_aligned_nonwrapping_storage() {
    let address: usize = kani::any();
    let length_output: bool = kani::any();
    let (actual, alignment, bytes) = if length_output {
        (
            validate_output_ptr(ptr::without_provenance_mut::<usize>(address)),
            align_of::<usize>(),
            size_of::<usize>(),
        )
    } else {
        (
            validate_output_ptr(ptr::without_provenance_mut::<i32>(address)),
            align_of::<i32>(),
            size_of::<i32>(),
        )
    };
    let valid = valid_scalar_range(address, alignment, bytes);
    assert_eq!(
        actual,
        if valid {
            Ok(())
        } else {
            Err(CODEC_INVALID_ARGUMENT)
        }
    );
    kani::cover!(length_output && valid, "aligned produced-length storage");
    kani::cover!(!length_output && valid, "aligned status flag storage");
    kani::cover!(address == 0, "null scalar output");
    kani::cover!(address == 1 && alignment > 1, "misaligned scalar output");
    kani::cover!(
        address != 0 && address % alignment == 0 && wide(address) + wide(bytes) > wide(usize::MAX),
        "aligned scalar whose end wraps"
    );
}

#[kani::proof]
fn ffi_buffer_and_produced_length_match_the_disjoint_storage_rule() {
    let write: usize = kani::any();
    let write_len: usize = kani::any();
    let length: usize = kani::any();
    let scalar_bytes = size_of::<usize>();
    let actual = validate_output_len_pair(
        ptr::without_provenance_mut(write),
        write_len,
        ptr::without_provenance_mut(length),
    );
    let valid = valid_scalar_range(length, align_of::<usize>(), scalar_bytes)
        && valid_byte_range(write, write_len)
        && !ranges_overlap(write, write_len, length, scalar_bytes);
    assert_eq!(
        actual,
        if valid {
            CODEC_OK
        } else {
            CODEC_INVALID_ARGUMENT
        }
    );
    kani::cover!(
        write == 0 && write_len == 0 && valid,
        "null empty output buffer"
    );
    kani::cover!(
        valid && write_len > 0 && wide(write) + wide(write_len) == wide(length),
        "output buffer ends at the first byte of the length scalar"
    );
    kani::cover!(
        write_len > 0
            && write == length
            && valid_scalar_range(length, align_of::<usize>(), scalar_bytes),
        "output buffer starts on its length scalar"
    );
}

#[kani::proof]
fn ffi_numeric_status_output_protects_non_null_input() {
    let read: usize = kani::any();
    let read_len: usize = kani::any();
    let status: usize = kani::any();
    let scalar_bytes = size_of::<i32>();
    let actual = validate_input_scalar_output(
        ptr::without_provenance(read),
        read_len,
        ptr::without_provenance_mut::<i32>(status),
    );
    // Null input is handled by read_slice after initializing this output.
    // Any non-null input must already describe a disjoint numeric range.
    let valid = valid_scalar_range(status, align_of::<i32>(), scalar_bytes)
        && (read == 0
            || (valid_byte_range(read, read_len)
                && !ranges_overlap(read, read_len, status, scalar_bytes)));
    assert_eq!(
        actual,
        if valid {
            Ok(())
        } else {
            Err(CODEC_INVALID_ARGUMENT)
        }
    );
    kani::cover!(
        read == 0 && read_len > 0 && valid,
        "null input is left to read_slice"
    );
    kani::cover!(
        valid && read_len > 0 && wide(status) + wide(scalar_bytes) == wide(read),
        "input begins immediately after the status scalar"
    );
    kani::cover!(
        read_len > 0
            && read == status
            && valid_scalar_range(status, align_of::<i32>(), scalar_bytes),
        "input aliasing a scalar would change the bytes before parsing"
    );
}

fn ranges_overlap(first: usize, first_len: usize, second: usize, second_len: usize) -> bool {
    let later_start = wide(first.max(second));
    later_start - wide(first) < wide(first_len) && later_start - wide(second) < wide(second_len)
}

fn valid_scalar_range(address: usize, alignment: usize, bytes: usize) -> bool {
    address != 0 && address % alignment == 0 && wide(address) + wide(bytes) <= wide(usize::MAX)
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
