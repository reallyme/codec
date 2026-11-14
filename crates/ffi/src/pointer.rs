// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(unsafe_code)]
#![allow(clippy::missing_safety_doc)]

use crate::status::{CodecStatus, CODEC_INVALID_ARGUMENT};

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
    ptr.addr().checked_add(len).ok_or(CODEC_INVALID_ARGUMENT)?;
    Ok(())
}

fn validate_output_ptr<T>(ptr: *mut T) -> Result<(), CodecStatus> {
    if ptr.is_null() {
        return Err(CODEC_INVALID_ARGUMENT);
    }
    if !ptr.is_aligned() {
        return Err(CODEC_INVALID_ARGUMENT);
    }
    ptr.addr()
        .checked_add(core::mem::size_of::<T>())
        .ok_or(CODEC_INVALID_ARGUMENT)?;
    Ok(())
}

/// Rejects input that overlaps scalar output storage before initialization.
///
/// Byte outputs can reuse input storage after an operation finishes reading,
/// but scalar outputs are initialized before parsing and must be disjoint.
pub(crate) fn validate_input_scalar_output<T>(
    input_ptr: *const u8,
    input_len: usize,
    output_ptr: *mut T,
) -> Result<(), CodecStatus> {
    validate_output_ptr(output_ptr)?;
    // A null input cannot overlap valid scalar storage. Leave its syntax
    // validation to read_slice so ordinary invalid inputs still receive the
    // documented initialized failure output.
    if input_ptr.is_null() {
        return Ok(());
    }
    validate_disjoint_ranges(
        input_ptr,
        input_len,
        output_ptr.cast::<u8>(),
        core::mem::size_of::<T>(),
    )
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
pub(crate) unsafe fn read_slice<'a>(ptr: *const u8, len: usize) -> Result<&'a [u8], CodecStatus> {
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
pub(crate) unsafe fn write_slice<'a>(
    ptr: *mut u8,
    len: usize,
) -> Result<&'a mut [u8], CodecStatus> {
    validate_write_pair(ptr, len)?;
    if len == 0 {
        return Ok(&mut []);
    }
    // SAFETY: `validate_write_pair` rejects null and oversized non-empty
    // ranges. The caller's safety contract supplies exclusive writable memory
    // for the returned borrow lifetime.
    Ok(unsafe { core::slice::from_raw_parts_mut(ptr, len) })
}

/// Validates a `*mut usize` output pointer without writing to it.
///
/// Variable-length FFI operations call this before mutating byte outputs so a
/// bad produced-length pointer cannot produce a partial success value.
pub(crate) fn validate_len_output(ptr: *mut usize) -> CodecStatus {
    if validate_output_ptr(ptr).is_err() {
        return CODEC_INVALID_ARGUMENT;
    }
    crate::status::CODEC_OK
}

/// Validates that a byte output buffer and its produced-length pointer are
/// individually valid and do not overlap.
pub(crate) fn validate_output_len_pair(
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

/// Writes `value` through a `*mut usize` output pointer (used for the
/// `*_len_out` produced-length parameters), returning
/// [`CODEC_INVALID_ARGUMENT`] if the pointer is null.
///
/// # Safety
///
/// `ptr`, when non-null, must point to a writable, properly aligned `usize`
/// valid for the duration of the call.
pub(crate) unsafe fn write_len(ptr: *mut usize, value: usize) -> CodecStatus {
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
pub(crate) unsafe fn write_i32(ptr: *mut i32, value: i32) -> CodecStatus {
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
#[path = "pointer_tests.rs"]
mod tests;
