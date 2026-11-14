// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    read_slice, validate_disjoint_ranges, validate_input_scalar_output, validate_len_output,
    validate_output_len_pair, validate_write_pair, write_i32, write_len, write_slice, CodecStatus,
};
use crate::status::{CODEC_BUFFER_TOO_SMALL, CODEC_INVALID_ARGUMENT, CODEC_OK};
use core::ptr::NonNull;

#[test]
fn read_slice_accepts_null_only_for_empty_input() {
    let empty = unsafe { read_slice(core::ptr::null(), 0) };
    assert!(matches!(empty, Ok(value) if value.is_empty()));

    let status = unsafe { read_slice(core::ptr::null(), 1) };
    assert_eq!(status, Err(CODEC_INVALID_ARGUMENT));
}

#[test]
fn read_slice_rejects_lengths_that_cannot_be_valid_rust_slices() {
    let ptr = NonNull::<u8>::dangling().as_ptr();
    let status = unsafe { read_slice(ptr, usize::MAX) };
    assert_eq!(status, Err(CODEC_INVALID_ARGUMENT));
}

#[test]
fn scalar_overlap_validation_handles_partial_and_adjacent_ranges() {
    let mut storage = [0_usize; 4];
    let scalar = storage.as_mut_ptr().wrapping_add(1);
    let bytes = storage.as_ptr().cast::<u8>();
    let width = core::mem::size_of::<usize>();
    // Inputs ending exactly at the scalar, or beginning immediately after
    // it, remain legal. A single byte of intersection must fail closed.
    for (offset, length, overlaps) in [
        (0, width, false),
        (0, width + 1, true),
        (width - 1, 2, true),
        (width, 0, false),
        (width, 1, true),
        (2 * width - 1, 2, true),
        (2 * width, width, false),
    ] {
        assert_eq!(
            validate_input_scalar_output(bytes.wrapping_add(offset), length, scalar),
            if overlaps {
                Err(CODEC_INVALID_ARGUMENT)
            } else {
                Ok(())
            }
        );
    }
    assert_eq!(storage, [0; 4]);
}

#[test]
fn slice_helpers_reject_address_wraparound() {
    let pointer = core::ptr::without_provenance_mut::<u8>(usize::MAX);
    // SAFETY: These impossible ranges must be rejected before a slice is
    // constructed or any memory is accessed.
    assert_eq!(
        unsafe { read_slice(pointer, 2) },
        Err(CODEC_INVALID_ARGUMENT)
    );
    assert_eq!(
        unsafe { write_slice(pointer, 2) },
        Err(CODEC_INVALID_ARGUMENT)
    );
}

#[test]
fn write_slice_accepts_null_only_for_empty_output() {
    let empty = unsafe { write_slice(core::ptr::null_mut(), 0) };
    assert!(matches!(empty, Ok(value) if value.is_empty()));

    let status = unsafe { write_slice(core::ptr::null_mut(), 1) };
    assert_eq!(status, Err(CODEC_INVALID_ARGUMENT));
}

#[test]
fn write_slice_rejects_lengths_that_cannot_be_valid_rust_slices() {
    let ptr = NonNull::<u8>::dangling().as_ptr();
    let status = unsafe { write_slice(ptr, usize::MAX) };
    assert_eq!(status, Err(CODEC_INVALID_ARGUMENT));
}

#[test]
fn write_fixed_rejects_small_buffers_without_mutating_them() {
    let mut output = [0xA5_u8; 2];
    let status = unsafe { write_fixed(output.as_mut_ptr(), output.len(), &[1, 2, 3]) };
    assert_eq!(status, CODEC_BUFFER_TOO_SMALL);
    assert_eq!(output, [0xA5, 0xA5]);
}

#[test]
fn write_fixed_rejects_overlapping_input_and_output_ranges() {
    let mut storage = [1_u8, 2, 3, 4, 0xA5, 0xA5, 0xA5, 0xA5];
    // Derive both raw pointers from one mutable provenance before creating
    // the shared slice. The rejected call never dereferences the output.
    let base_ptr = storage.as_mut_ptr();
    let output_ptr = unsafe { base_ptr.add(2) };
    let input = unsafe { core::slice::from_raw_parts(base_ptr.cast_const(), 4) };

    let status = unsafe { write_fixed(output_ptr, 4, input) };

    assert_eq!(status, CODEC_INVALID_ARGUMENT);
    assert_eq!(storage, [1, 2, 3, 4, 0xA5, 0xA5, 0xA5, 0xA5]);
}

#[test]
fn output_pair_helpers_reject_overlaps_without_writing() {
    let mut storage = [0_u8; 32];
    let first = storage.as_mut_ptr();
    let second = unsafe { storage.as_mut_ptr().add(8) };

    assert_eq!(
        validate_disjoint_output_pair(first, 16, second, 16),
        CODEC_INVALID_ARGUMENT
    );

    let len_out = unsafe { storage.as_mut_ptr().add(4).cast::<usize>() };
    assert_eq!(
        validate_output_len_pair(first, 16, len_out),
        CODEC_INVALID_ARGUMENT
    );

    let first_len = storage.as_mut_ptr().cast::<usize>();
    let second_len = unsafe { storage.as_mut_ptr().add(1).cast::<usize>() };
    assert_eq!(
        validate_disjoint_len_outputs(first_len, second_len),
        CODEC_INVALID_ARGUMENT
    );
}

#[test]
fn typed_output_writes_reject_null_and_misaligned_pointers() {
    assert_eq!(
        unsafe { write_len(core::ptr::null_mut(), 1) },
        CODEC_INVALID_ARGUMENT
    );
    assert_eq!(
        validate_len_output(core::ptr::null_mut()),
        CODEC_INVALID_ARGUMENT
    );
    assert_eq!(
        unsafe { write_i32(core::ptr::null_mut(), 1) },
        CODEC_INVALID_ARGUMENT
    );

    let mut usize_storage = [0_usize; 2];
    let misaligned_usize = usize_storage
        .as_mut_ptr()
        .cast::<u8>()
        .wrapping_add(1)
        .cast::<usize>();
    assert_eq!(
        unsafe { write_len(misaligned_usize, 1) },
        CODEC_INVALID_ARGUMENT
    );

    let mut i32_storage = [0_i32; 2];
    let misaligned_i32 = i32_storage
        .as_mut_ptr()
        .cast::<u8>()
        .wrapping_add(1)
        .cast::<i32>();
    assert_eq!(
        unsafe { write_i32(misaligned_i32, 1) },
        CODEC_INVALID_ARGUMENT
    );
}

#[test]
fn typed_output_writes_accept_aligned_pointers() {
    let mut len_out = 0_usize;
    let mut flag_out = 0_i32;

    assert_eq!(unsafe { write_len(&mut len_out, 7) }, CODEC_OK);
    assert_eq!(unsafe { write_i32(&mut flag_out, 1) }, CODEC_OK);
    assert_eq!(len_out, 7);
    assert_eq!(flag_out, 1);
}

/// Copies `value` into the output buffer `(ptr, len)`, returning
/// [`crate::status::CODEC_BUFFER_TOO_SMALL`] if the
/// buffer cannot hold it and [`crate::status::CODEC_OK`] on success.
///
/// # Safety
///
/// When `len != 0`, `ptr` must point to at least `len` bytes of writable,
/// properly aligned memory valid for the duration of the call.
pub(crate) unsafe fn write_fixed(ptr: *mut u8, len: usize, value: &[u8]) -> CodecStatus {
    if validate_write_pair(ptr, len).is_err() {
        return CODEC_INVALID_ARGUMENT;
    }
    if len < value.len() {
        return CODEC_BUFFER_TOO_SMALL;
    }
    if validate_disjoint_ranges(value.as_ptr(), value.len(), ptr, len).is_err() {
        return CODEC_INVALID_ARGUMENT;
    }
    // SAFETY: `validate_write_pair` checked the output pointer/length, and
    // `validate_disjoint_ranges` checked that the immutable input slice does
    // not alias the writable output range for the duration of the copy.
    let Ok(out) = (unsafe { write_slice(ptr, len) }) else {
        return CODEC_INVALID_ARGUMENT;
    };
    out[..value.len()].copy_from_slice(value);
    crate::status::CODEC_OK
}

/// Validates that two byte output buffers are individually valid and disjoint.
pub(crate) fn validate_disjoint_output_pair(
    first_ptr: *mut u8,
    first_len: usize,
    second_ptr: *mut u8,
    second_len: usize,
) -> CodecStatus {
    match validate_disjoint_ranges(first_ptr.cast_const(), first_len, second_ptr, second_len) {
        Ok(()) => crate::status::CODEC_OK,
        Err(status) => status,
    }
}

/// Validates that two produced-length output pointers are valid and disjoint.
pub(crate) fn validate_disjoint_len_outputs(first: *mut usize, second: *mut usize) -> CodecStatus {
    let first_status = validate_len_output(first);
    if first_status != crate::status::CODEC_OK {
        return first_status;
    }
    let second_status = validate_len_output(second);
    if second_status != crate::status::CODEC_OK {
        return second_status;
    }
    match validate_disjoint_ranges(
        first.cast::<u8>().cast_const(),
        core::mem::size_of::<usize>(),
        second.cast::<u8>(),
        core::mem::size_of::<usize>(),
    ) {
        Ok(()) => crate::status::CODEC_OK,
        Err(status) => status,
    }
}
