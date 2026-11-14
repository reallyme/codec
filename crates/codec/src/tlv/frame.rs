// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::error::{overflow, size};
use super::read_header::span;
use super::{read_header, ErrorReason, Frame, Options, TlvError};

/// Require one complete outer value and return its exact unconsumed suffix.
/// No length-driven allocation or recursive decoding is performed.
pub fn frame_prefix<'a>(
    input: &'a [u8],
    options: &Options,
) -> Result<(Frame<'a>, &'a [u8]), TlvError> {
    let header = read_header(input, options)?;
    // Compare in the wire's uint64 domain first. In particular, a declared
    // 0xffffffff with no value is TruncatedValue on wasm32 as well as native;
    // it must never become an overflow or an invented out-of-bounds span.
    if header.declared_value_length() > size(header.supplied_remainder().len())? {
        return Err(TlvError::new(ErrorReason::TruncatedValue));
    }
    let length = usize::try_from(header.declared_value_length()).map_err(|_| overflow())?;
    let start = header.consumed_bytes();
    let end = start.checked_add(length).ok_or_else(overflow)?;
    let received = input.get(..end).ok_or_else(overflow)?;
    let contents = input.get(start..end).ok_or_else(overflow)?;
    let remainder = input.get(end..).ok_or_else(overflow)?;
    Ok((
        Frame {
            header,
            received,
            contents,
            received_span: span(0, end)?,
            contents_span: span(start, length)?,
            consumed: size(end)?,
        },
        remainder,
    ))
}

/// Require exactly one complete outer value and reject every trailing byte.
pub fn frame_exact<'a>(input: &'a [u8], options: &Options) -> Result<Frame<'a>, TlvError> {
    let (frame, remainder) = frame_prefix(input, options)?;
    if !remainder.is_empty() {
        return Err(TlvError::new(ErrorReason::TrailingData));
    }
    Ok(frame)
}
