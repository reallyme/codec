// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use crate::status::{CodecStatus, CODEC_BUFFER_TOO_SMALL, CODEC_INVALID_ARGUMENT};

const MAX_FFI_SLICE_LEN: usize = isize::MAX.unsigned_abs();

fn validate_read_pair(ptr: *const u8, len: usize) -> Result<(), CodecStatus> {
    validate_nonzero_len(ptr, len)
}

fn validate_write_pair(ptr: *mut u8, len: usize) -> Result<(), CodecStatus> {
    validate_nonzero_len(ptr, len)
}

fn validate_nonzero_len<T>(ptr: *const T, len: usize) -> Result<(), CodecStatus> {
    if len == 0 {
        return Ok(());
    }
    if ptr.is_null() {
        return Err(CODEC_INVALID_ARGUMENT);
    }
    if len > MAX_FFI_SLICE_LEN {
        return Err(CODEC_INVALID_ARGUMENT);
    }
    Ok(())
}

fn validate_output_ptr<T>(ptr: *mut T) -> Result<(), CodecStatus> {
    if ptr.is_null() {
        return Err(CODEC_INVALID_ARGUMENT);
    }
    if !ptr.is_aligned() {
        return Err(CODEC_INVALID_ARGUMENT);
    }
    Ok(())
}

fn validate_disjoint_ranges(
    read_ptr: *const u8,
    read_len: usize,
    write_ptr: *mut u8,
    write_len: usize,
) -> Result<(), CodecStatus> {
    validate_read_pair(read_ptr, read_len)?;
    validate_write_pair(write_ptr, write_len)?;
    if read_len == 0 || write_len == 0 {
        return Ok(());
    }

    let read_start = read_ptr.addr();
    let write_start = write_ptr.addr();
    let read_end = read_start
        .checked_add(read_len)
        .ok_or(CODEC_INVALID_ARGUMENT)?;
    let write_end = write_start
        .checked_add(write_len)
        .ok_or(CODEC_INVALID_ARGUMENT)?;

    if read_start < write_end && write_start < read_end {
        return Err(CODEC_INVALID_ARGUMENT);
    }
    Ok(())
}

/// Builds a read-only slice from a caller-supplied `(ptr, len)` pair.
///
/// A null pointer with `len == 0` yields an empty slice; a null pointer with a
/// non-zero length returns [`CODEC_INVALID_ARGUMENT`]. Lengths larger than
/// `isize::MAX` are rejected before calling `from_raw_parts` because Rust slice
/// values must describe one allocation whose total size does not exceed that
/// bound.
///
/// # Safety
///
/// When `len != 0`, `ptr` must point to `len` initialized bytes that remain
/// valid and unmutated for the lifetime `'a` of the returned slice.
pub unsafe fn read_slice<'a>(ptr: *const u8, len: usize) -> Result<&'a [u8], CodecStatus> {
    validate_read_pair(ptr, len)?;
    if len == 0 {
        return Ok(&[]);
    }
    // SAFETY: `validate_read_pair` rejects null and oversized non-empty
    // ranges. The caller's safety contract supplies initialized bytes in one
    // allocation that remain immutable for the returned borrow lifetime.
    Ok(unsafe { core::slice::from_raw_parts(ptr, len) })
}

/// Builds a mutable slice over a caller-supplied output buffer `(ptr, len)`.
///
/// A null pointer with `len == 0` yields an empty slice; a null pointer with a
/// non-zero length returns [`CODEC_INVALID_ARGUMENT`]. Lengths larger than
/// `isize::MAX` are rejected before calling `from_raw_parts_mut` because Rust
/// slice values must describe one allocation whose total size does not exceed
/// that bound.
///
/// # Safety
///
/// When `len != 0`, `ptr` must point to `len` bytes of writable, properly
/// aligned memory that stays valid and exclusively borrowed for the lifetime
/// `'a` of the returned slice.
pub unsafe fn write_slice<'a>(ptr: *mut u8, len: usize) -> Result<&'a mut [u8], CodecStatus> {
    validate_write_pair(ptr, len)?;
    if len == 0 {
        return Ok(&mut []);
    }
    // SAFETY: `validate_write_pair` rejects null and oversized non-empty
    // ranges. The caller's safety contract supplies exclusive writable memory
    // for the returned borrow lifetime.
    Ok(unsafe { core::slice::from_raw_parts_mut(ptr, len) })
}

/// Copies `value` into the output buffer `(ptr, len)`, returning
/// [`crate::status::CODEC_BUFFER_TOO_SMALL`] if the
/// buffer cannot hold it and [`crate::status::CODEC_OK`] on success.
///
/// # Safety
///
/// When `len != 0`, `ptr` must point to at least `len` bytes of writable,
/// properly aligned memory valid for the duration of the call.
pub unsafe fn write_fixed(ptr: *mut u8, len: usize, value: &[u8]) -> CodecStatus {
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

/// Validates a `*mut usize` output pointer without writing to it.
///
/// Variable-length FFI operations call this before mutating byte outputs so a
/// bad produced-length pointer cannot produce a partial success value.
pub fn validate_len_output(ptr: *mut usize) -> CodecStatus {
    if validate_output_ptr(ptr).is_err() {
        return CODEC_INVALID_ARGUMENT;
    }
    crate::status::CODEC_OK
}

/// Validates that a byte output buffer and its produced-length pointer are
/// individually valid and do not overlap.
pub fn validate_output_len_pair(
    output_ptr: *mut u8,
    output_len: usize,
    len_out: *mut usize,
) -> CodecStatus {
    let len_status = validate_len_output(len_out);
    if len_status != crate::status::CODEC_OK {
        return len_status;
    }
    if validate_write_pair(output_ptr, output_len).is_err() {
        return CODEC_INVALID_ARGUMENT;
    }
    match validate_disjoint_ranges(
        output_ptr.cast_const(),
        output_len,
        len_out.cast::<u8>(),
        core::mem::size_of::<usize>(),
    ) {
        Ok(()) => crate::status::CODEC_OK,
        Err(status) => status,
    }
}

/// Validates that two byte output buffers are individually valid and disjoint.
pub fn validate_disjoint_output_pair(
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
pub fn validate_disjoint_len_outputs(first: *mut usize, second: *mut usize) -> CodecStatus {
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

/// Writes `value` through a `*mut usize` output pointer (used for the
/// `*_len_out` produced-length parameters), returning
/// [`CODEC_INVALID_ARGUMENT`] if the pointer is null.
///
/// # Safety
///
/// `ptr`, when non-null, must point to a writable, properly aligned `usize`
/// valid for the duration of the call.
pub unsafe fn write_len(ptr: *mut usize, value: usize) -> CodecStatus {
    if validate_output_ptr(ptr).is_err() {
        return CODEC_INVALID_ARGUMENT;
    }
    // SAFETY: `validate_output_ptr` rejects null and misaligned pointers. The
    // caller's safety contract supplies writable `usize` storage owned by the
    // caller for the duration of this call.
    unsafe {
        *ptr = value;
    }
    crate::status::CODEC_OK
}

/// Writes `value` through a `*mut i32` output pointer (used for the
/// `valid_out` / `equal_out` result flags), returning
/// [`CODEC_INVALID_ARGUMENT`] if the pointer is null.
///
/// # Safety
///
/// `ptr`, when non-null, must point to a writable, properly aligned `i32`
/// valid for the duration of the call.
pub unsafe fn write_i32(ptr: *mut i32, value: i32) -> CodecStatus {
    if validate_output_ptr(ptr).is_err() {
        return CODEC_INVALID_ARGUMENT;
    }
    // SAFETY: `validate_output_ptr` rejects null and misaligned pointers. The
    // caller's safety contract supplies writable `i32` storage owned by the
    // caller for the duration of this call.
    unsafe {
        *ptr = value;
    }
    crate::status::CODEC_OK
}

#[cfg(test)]
mod tests {
    use super::{
        read_slice, validate_disjoint_len_outputs, validate_disjoint_output_pair,
        validate_len_output, validate_output_len_pair, write_fixed, write_i32, write_len,
        write_slice,
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
        let input_ptr = storage.as_ptr();
        let output_ptr = unsafe { storage.as_mut_ptr().add(2) };
        let input = unsafe { core::slice::from_raw_parts(input_ptr, 4) };

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
}
