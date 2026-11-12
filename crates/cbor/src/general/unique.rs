// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Map-key uniqueness using a bounded index, independent of received key order.
//!
//! Keys are sorted with a bounded, fallible merge sort under a total preorder
//! whose ties include every pair that [`equivalent`] accepts. Equivalent keys
//! therefore end up adjacent, inside one tie run, and only runs are checked
//! with [`equivalent`]. Keys without descendant maps have an exact preorder and
//! need `O(n log n)` comparisons. Maps tie on entry count, so maps and compound
//! keys containing maps can share coarse ties. Their runs use pairwise semantic
//! comparisons and may exhaust work before the entry ceiling. Every traversal
//! and comparison charges the shared work budget.
//! Only key start offsets are stored; key bytes are never copied.

use core::cmp::Ordering;

use super::compare::{equivalent, next};
use super::error::{error, CborError, ErrorReason};
use super::limits::Budget;
use super::read_header::MajorType;
use super::receipt::Frame;
use super::scan::{add, tail};

/// Maps with at most this many keys are checked pairwise from inline storage,
/// so validating typical small maps performs no heap allocation.
const INLINE_KEYS: usize = 16;

/// Key start offsets for one map: inline for small maps, then a bounded heap
/// list. Offsets are not key material.
pub(super) struct KeyStarts {
    inline: [usize; INLINE_KEYS],
    len: usize,
    heap: Vec<usize>,
}

impl KeyStarts {
    /// Prepare empty storage. Heap storage grows only as keys are actually
    /// read, so a declared count never reserves memory ahead of the input.
    pub(super) const fn new() -> Self {
        Self {
            inline: [0; INLINE_KEYS],
            len: 0,
            heap: Vec::new(),
        }
    }

    pub(super) fn push(&mut self, start: usize) -> Result<(), CborError> {
        if self.heap.is_empty() {
            if let Some(slot) = self.inline.get_mut(self.len) {
                *slot = start;
                self.len = add(self.len, 1)?;
                return Ok(());
            }
            let inline = self
                .inline
                .get(..self.len)
                .ok_or(error(ErrorReason::ArithmeticOverflow))?;
            self.heap
                .try_reserve(INLINE_KEYS.checked_mul(2).ok_or(error(ErrorReason::ArithmeticOverflow))?)
                .map_err(|_| error(ErrorReason::AllocationFailure))?;
            self.heap.extend_from_slice(inline);
        }
        self.heap
            .try_reserve(1)
            .map_err(|_| error(ErrorReason::AllocationFailure))?;
        self.heap.push(start);
        Ok(())
    }

    fn as_slice(&self) -> Result<&[usize], CborError> {
        if self.heap.is_empty() {
            self.inline.get(..self.len).ok_or(error(ErrorReason::ArithmeticOverflow))
        } else {
            Ok(&self.heap)
        }
    }
}

/// Reject a map whose recorded keys contain two equivalent keys.
pub(super) fn check_unique(
    input: &[u8],
    keys: &KeyStarts,
    budget: &mut Budget,
) -> Result<(), CborError> {
    let starts = keys.as_slice()?;
    let len = starts.len();
    if len < 2 {
        return Ok(());
    }
    if len <= INLINE_KEYS {
        let indices: [usize; INLINE_KEYS] = core::array::from_fn(|index| index);
        let run = indices
            .get(..len)
            .ok_or(error(ErrorReason::ArithmeticOverflow))?;
        return check_run(input, starts, run, budget);
    }
    let mut order = try_indices(len)?;
    let mut scratch = try_indices(len)?;
    merge_sort(input, starts, &mut order, &mut scratch, budget)?;

    let mut run_start = 0_usize;
    for index in 1..=len {
        let previous_index = index.checked_sub(1).ok_or(error(ErrorReason::ArithmeticOverflow))?;
        let tied = match (order.get(previous_index), order.get(index)) {
            (Some(&previous), Some(&current)) => {
                compare_at(input, starts, previous, current, budget)? == Ordering::Equal
            }
            _ => false,
        };
        if !tied {
            let run = order
                .get(run_start..index)
                .ok_or(error(ErrorReason::ArithmeticOverflow))?;
            check_run(input, starts, run, budget)?;
            run_start = index;
        }
    }
    Ok(())
}

fn try_indices(len: usize) -> Result<Vec<usize>, CborError> {
    let mut indices = Vec::new();
    indices
        .try_reserve_exact(len)
        .map_err(|_| error(ErrorReason::AllocationFailure))?;
    indices.extend(0..len);
    Ok(indices)
}

fn check_run(
    input: &[u8],
    starts: &[usize],
    run: &[usize],
    budget: &mut Budget,
) -> Result<(), CborError> {
    for (position, &first) in run.iter().enumerate() {
        let rest = run.get(add(position, 1)?..).ok_or(error(ErrorReason::ArithmeticOverflow))?;
        for &second in rest {
            let left = frame_at(input, starts, first, budget)?;
            let right = frame_at(input, starts, second, budget)?;
            if equivalent(&left, &right, budget, 0)? {
                return Err(error(ErrorReason::DuplicateMapKey));
            }
        }
    }
    Ok(())
}

fn merge_sort(
    input: &[u8],
    starts: &[usize],
    order: &mut [usize],
    scratch: &mut [usize],
    budget: &mut Budget,
) -> Result<(), CborError> {
    let len = order.len();
    let mut width = 1_usize;
    while width < len {
        let mut start = 0_usize;
        while start < len {
            let middle = add(start, width)?.min(len);
            let end = add(middle, width)?.min(len);
            let left = order
                .get(start..middle)
                .ok_or(error(ErrorReason::ArithmeticOverflow))?;
            let right = order
                .get(middle..end)
                .ok_or(error(ErrorReason::ArithmeticOverflow))?;
            let output = scratch
                .get_mut(start..end)
                .ok_or(error(ErrorReason::ArithmeticOverflow))?;
            merge(input, starts, left, right, output, budget)?;
            start = end;
        }
        if order.len() != scratch.len() { return Err(error(ErrorReason::ArithmeticOverflow)); }
        order.copy_from_slice(scratch);
        width = width.checked_mul(2).ok_or(error(ErrorReason::ArithmeticOverflow))?;
    }
    Ok(())
}

fn merge(
    input: &[u8],
    starts: &[usize],
    left: &[usize],
    right: &[usize],
    output: &mut [usize],
    budget: &mut Budget,
) -> Result<(), CborError> {
    let mut left = left.iter().copied().peekable();
    let mut right = right.iter().copied().peekable();
    for slot in output.iter_mut() {
        let take_left = match (left.peek(), right.peek()) {
            (Some(&first), Some(&second)) => {
                compare_at(input, starts, first, second, budget)? != Ordering::Greater
            }
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => return Err(error(ErrorReason::ArithmeticOverflow)),
        };
        let next_index = if take_left { left.next() } else { right.next() };
        *slot = next_index.ok_or(error(ErrorReason::ArithmeticOverflow))?;
    }
    Ok(())
}

fn frame_at<'a>(
    input: &'a [u8],
    starts: &[usize],
    index: usize,
    budget: &mut Budget,
) -> Result<Frame<'a>, CborError> {
    let start = *starts
        .get(index)
        .ok_or(error(ErrorReason::ArithmeticOverflow))?;
    let mut offset = 0_usize;
    next(tail(input, start)?, &mut offset, budget)
}

fn compare_at(
    input: &[u8],
    starts: &[usize],
    first: usize,
    second: usize,
    budget: &mut Budget,
) -> Result<Ordering, CborError> {
    let left = frame_at(input, starts, first, budget)?;
    let right = frame_at(input, starts, second, budget)?;
    key_order(&left, &right, budget, 0)
}

const fn rank(major: MajorType) -> u8 {
    match major {
        MajorType::Unsigned => 0,
        MajorType::Negative => 1,
        MajorType::Bytes => 2,
        MajorType::Text => 3,
        MajorType::Array => 4,
        MajorType::Map => 5,
        MajorType::Tag => 6,
        MajorType::Simple => 7,
    }
}

fn key_order(
    left: &Frame<'_>,
    right: &Frame<'_>,
    budget: &mut Budget,
    depth: usize,
) -> Result<Ordering, CborError> {
    budget.charge(1)?;
    let major = left.header.major_type();
    match rank(major).cmp(&rank(right.header.major_type())) {
        Ordering::Equal => {}
        different => return Ok(different),
    }
    match major {
        MajorType::Unsigned | MajorType::Negative => {
            Ok(left.header.argument().cmp(&right.header.argument()))
        }
        MajorType::Bytes | MajorType::Text => strings_order(left, right, budget),
        MajorType::Simple => Ok(simple_key(left)?.cmp(&simple_key(right)?)),
        MajorType::Tag => {
            match left.header.argument().cmp(&right.header.argument()) {
                Ordering::Equal => {}
                different => return Ok(different),
            }
            let depth = budget.enter(depth)?;
            let first = next(left.body, &mut 0, budget)?;
            let second = next(right.body, &mut 0, budget)?;
            key_order(&first, &second, budget, depth)
        }
        MajorType::Array => {
            let depth = budget.enter(depth)?;
            let (mut first, mut second) = (0_usize, 0_usize);
            loop {
                match (first < left.body.len(), second < right.body.len()) {
                    (true, true) => {
                        let x = next(left.body, &mut first, budget)?;
                        let y = next(right.body, &mut second, budget)?;
                        match key_order(&x, &y, budget, depth)? {
                            Ordering::Equal => {}
                            different => return Ok(different),
                        }
                    }
                    (false, false) => return Ok(Ordering::Equal),
                    (false, true) => return Ok(Ordering::Less),
                    (true, false) => return Ok(Ordering::Greater),
                }
            }
        }
        MajorType::Map => Ok(entry_count(left, budget)?.cmp(&entry_count(right, budget)?)),
    }
}

fn simple_key(frame: &Frame<'_>) -> Result<(u8, u8, u8, u64), CborError> {
    match frame.header.float()? {
        Some(value) => {
            let (class, bits) = value.equivalence_key();
            Ok((1, 0, class, bits))
        }
        None => Ok((0, frame.header.simple_value()?.ok_or(error(ErrorReason::WrongType))?, 0, 0)),
    }
}

/// Count entries by walking the body, so definite and indefinite maps with
/// the same content tie.
fn entry_count(frame: &Frame<'_>, budget: &mut Budget) -> Result<usize, CborError> {
    let mut offset = 0_usize;
    let mut count = 0_usize;
    while offset < frame.body.len() {
        next(frame.body, &mut offset, budget)?;
        next(frame.body, &mut offset, budget)?;
        count = count
            .checked_add(1)
            .ok_or(error(ErrorReason::ArithmeticOverflow))?;
    }
    Ok(count)
}

// Lexicographic order over logical string contents, walking chunks so a
// chunked string ties with a definite string of the same contents.
fn strings_order(
    left: &Frame<'_>,
    right: &Frame<'_>,
    budget: &mut Budget,
) -> Result<Ordering, CborError> {
    let mut left = left.string_chunks()?;
    let mut right = right.string_chunks()?;
    let mut first: &[u8] = &[];
    let mut second: &[u8] = &[];
    loop {
        while first.is_empty() {
            match left.next() {
                Some(chunk) => {
                    budget.charge(1)?;
                    first = chunk?;
                }
                None => break,
            }
        }
        while second.is_empty() {
            match right.next() {
                Some(chunk) => {
                    budget.charge(1)?;
                    second = chunk?;
                }
                None => break,
            }
        }
        match (first.is_empty(), second.is_empty()) {
            (true, true) => return Ok(Ordering::Equal),
            (true, false) => return Ok(Ordering::Less),
            (false, true) => return Ok(Ordering::Greater),
            (false, false) => {
                let length = first.len().min(second.len());
                budget.charge(length)?;
                match first.get(..length).cmp(&second.get(..length)) {
                    Ordering::Equal => {}
                    different => return Ok(different),
                }
                first = tail(first, length)?;
                second = tail(second, length)?;
            }
        }
    }
}
