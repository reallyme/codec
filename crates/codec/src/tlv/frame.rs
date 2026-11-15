// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::error::{overflow, size};
use super::read_header::span;
use super::{read_header, ErrorReason, Frame, Header, Options, TlvError};

/// Require one complete outer value and return its exact unconsumed suffix.
/// No length-driven allocation or recursive decoding is performed.
pub fn frame_prefix<'a>(
    input: &'a [u8],
    options: &Options,
) -> Result<(Frame<'a>, &'a [u8]), TlvError> {
    let header = read_header(input, options)?;
    frame_from_header(input, header)
}

pub(super) fn value_end(start: usize, available: usize, declared: u64) -> Result<usize, TlvError> {
    // Compare in the wire's uint64 domain first. In particular, a declared
    // 0xffffffff with no value is TruncatedValue on wasm32 as well as native;
    // it must never become an overflow or an invented out-of-bounds span.
    if declared > size(available)? {
        return Err(TlvError::new(ErrorReason::TruncatedValue));
    }
    let length = usize::try_from(declared).map_err(|_| overflow())?;
    start.checked_add(length).ok_or_else(overflow)
}

pub(super) fn frame_from_header<'a>(
    input: &'a [u8],
    header: Header<'a>,
) -> Result<(Frame<'a>, &'a [u8]), TlvError> {
    let start = header.consumed_bytes();
    let end = value_end(
        start,
        header.supplied_remainder().len(),
        header.declared_value_length(),
    )?;
    let received = input.get(..end).ok_or_else(overflow)?;
    let contents = input.get(start..end).ok_or_else(overflow)?;
    let remainder = input.get(end..).ok_or_else(overflow)?;
    Ok((
        Frame {
            header,
            received,
            contents,
            received_span: span(0, end)?,
            contents_span: span(start, contents.len())?,
            consumed: size(end)?,
        },
        remainder,
    ))
}

/// Require exactly one complete outer value and reject every trailing byte.
pub fn frame_exact<'a>(input: &'a [u8], options: &Options) -> Result<Frame<'a>, TlvError> {
    require_exact_frame(frame_prefix(input, options)?)
}

pub(super) fn require_exact_frame<'a>(
    (frame, remainder): (Frame<'a>, &'a [u8]),
) -> Result<Frame<'a>, TlvError> {
    if !remainder.is_empty() {
        return Err(TlvError::new(ErrorReason::TrailingData));
    }
    Ok(frame)
}
