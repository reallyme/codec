// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::error::{error, CborError, ErrorReason};
use super::limits::{Budget, Limits};
use super::profile::LengthPolicy;
use super::receipt::{check_input, Frame};
use super::scan::{add, scan, tail};

/// Framing cursor for a bounded sequence. Limits accumulate across items.
/// After a parsing error it is fused; no malformed suffix can be skipped.
/// Empty sequences are allowed, while `frame_exact` requires one item.
pub struct Cursor<'a> {
    input: &'a [u8],
    offset: usize,
    consumed: &'a [u8],
    remaining: &'a [u8],
    budget: Budget,
    lengths: LengthPolicy,
    failed: bool,
}

impl<'a> Cursor<'a> {
    /// Create a cursor with explicit transport and indefinite-length limits.
    pub fn new(input: &'a [u8], limits: Limits, lengths: LengthPolicy) -> Result<Self, CborError> {
        check_input(input, limits)?;
        Ok(Self {
            input,
            offset: 0,
            consumed: input.get(..0).ok_or(error(ErrorReason::Truncated))?,
            remaining: input,
            budget: Budget::new(limits),
            lengths,
            failed: false,
        })
    }
    /// Exact successfully consumed received span.
    #[must_use]
    pub fn consumed_bytes(&self) -> &'a [u8] {
        self.consumed
    }
    /// Remaining received span, including an item that failed to frame.
    #[must_use]
    pub fn remaining_bytes(&self) -> &'a [u8] {
        self.remaining
    }
}

impl<'a> Iterator for Cursor<'a> {
    type Item = Result<Frame<'a>, CborError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.failed || self.offset == self.input.len() {
            return None;
        }
        let result = (|| {
            let frame = scan(
                tail(self.input, self.offset)?,
                &mut self.budget,
                0,
                None,
                true,
                self.lengths,
            )?;
            let offset = add(self.offset, frame.received_bytes().len())?;
            let consumed = self
                .input
                .get(..offset)
                .ok_or(error(ErrorReason::Truncated))?;
            let remaining = tail(self.input, offset)?;
            self.offset = offset;
            self.consumed = consumed;
            self.remaining = remaining;
            Ok(frame)
        })();
        if result.is_err() {
            self.failed = true;
        }
        Some(result)
    }
}

impl core::iter::FusedIterator for Cursor<'_> {}
