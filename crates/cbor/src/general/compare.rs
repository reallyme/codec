// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::cmp::Ordering;

use super::error::{error, CborError, ErrorReason};
use super::limits::Budget;
use super::profile::{LengthPolicy, MapOrdering};
use super::read_header::MajorType;
use super::receipt::Frame;
use super::scan::{add, scan, tail};

pub(super) fn ordered(
    left: &Frame<'_>,
    right: &Frame<'_>,
    ordering: MapOrdering,
    budget: &mut Budget,
) -> Result<bool, CborError> {
    let left = left.received_bytes();
    let right = right.received_bytes();
    if ordering == MapOrdering::LengthFirst && left.len() != right.len() {
        return Ok(left.len() < right.len());
    }
    budget.charge(left.len().max(right.len()))?;
    Ok(left.cmp(right) == Ordering::Less)
}

pub(super) fn next<'a>(
    bytes: &'a [u8],
    offset: &mut usize,
    budget: &mut Budget,
) -> Result<Frame<'a>, CborError> {
    let item = scan(
        tail(bytes, *offset)?,
        budget,
        0,
        None,
        false,
        LengthPolicy::AllowIndefinite,
    )?;
    *offset = add(*offset, item.received_bytes().len())?;
    Ok(item)
}

// Comparing logical chunks avoids allocating a concatenated copy of a key.
// Each byte comparison and chunk traversal consumes the shared work budget.
fn strings_equal(
    left: &Frame<'_>,
    right: &Frame<'_>,
    budget: &mut Budget,
) -> Result<bool, CborError> {
    let mut left = left.string_chunks()?;
    let mut right = right.string_chunks()?;
    let mut a: &[u8] = &[];
    let mut b: &[u8] = &[];
    loop {
        while a.is_empty() {
            match left.next() {
                Some(chunk) => {
                    budget.charge(1)?;
                    a = chunk?;
                    if !a.is_empty() {
                        break;
                    }
                }
                None => break,
            }
        }
        while b.is_empty() {
            match right.next() {
                Some(chunk) => {
                    budget.charge(1)?;
                    b = chunk?;
                    if !b.is_empty() {
                        break;
                    }
                }
                None => break,
            }
        }
        if a.is_empty() || b.is_empty() {
            return Ok(a.is_empty() && b.is_empty());
        }
        let length = a.len().min(b.len());
        budget.charge(length)?;
        if a.get(..length) != b.get(..length) {
            return Ok(false);
        }
        a = tail(a, length)?;
        b = tail(b, length)?;
    }
}

/// RFC 8949 section 5.6.1 basic-model equality, independent of serialization.
/// Integer/float and byte/text types remain distinct. Maps are unordered pairs;
/// arrays are ordered. NaNs compare by right-zero-extended significand, while
/// float zero signs compare equal. Registered tags retain their tag identity.
pub(super) fn equivalent(
    left: &Frame<'_>,
    right: &Frame<'_>,
    budget: &mut Budget,
    depth: usize,
) -> Result<bool, CborError> {
    budget.charge(1)?;
    if left.header.major_type() != right.header.major_type() {
        return Ok(false);
    }
    match left.header.major_type() {
        MajorType::Unsigned | MajorType::Negative => {
            Ok(left.header.argument() == right.header.argument())
        }
        MajorType::Bytes | MajorType::Text => strings_equal(left, right, budget),
        MajorType::Simple => match (left.header.float()?, right.header.float()?) {
            (Some(a), Some(b)) => Ok(a.equivalent(&b)),
            (None, None) => Ok(left.header.simple_value()? == right.header.simple_value()?),
            _ => Ok(false),
        },
        MajorType::Tag => {
            if left.header.argument() != right.header.argument() {
                return Ok(false);
            }
            let depth = budget.enter(depth)?;
            let a = next(left.body, &mut 0, budget)?;
            let b = next(right.body, &mut 0, budget)?;
            equivalent(&a, &b, budget, depth)
        }
        MajorType::Array => {
            let depth = budget.enter(depth)?;
            let (mut a, mut b) = (0, 0);
            while a < left.body.len() && b < right.body.len() {
                let x = next(left.body, &mut a, budget)?;
                let y = next(right.body, &mut b, budget)?;
                if !equivalent(&x, &y, budget, depth)? {
                    return Ok(false);
                }
            }
            Ok(a == left.body.len() && b == right.body.len())
        }
        MajorType::Map => {
            let depth = budget.enter(depth)?;
            let mut left_count = 0_usize;
            let mut right_count = 0_usize;
            let mut a = 0;
            while a < left.body.len() {
                let key = next(left.body, &mut a, budget)?;
                let value = next(left.body, &mut a, budget)?;
                left_count = add(left_count, 1)?;
                let mut b = 0;
                let mut found = false;
                while b < right.body.len() {
                    let other_key = next(right.body, &mut b, budget)?;
                    let other_value = next(right.body, &mut b, budget)?;
                    if equivalent(&key, &other_key, budget, depth)?
                        && equivalent(&value, &other_value, budget, depth)?
                    {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Ok(false);
                }
            }
            let mut b = 0;
            while b < right.body.len() {
                next(right.body, &mut b, budget)?;
                next(right.body, &mut b, budget)?;
                right_count = add(right_count, 1)?;
            }
            Ok(left_count == right_count)
        }
        _ => Err(error(ErrorReason::WrongType)),
    }
}

pub(super) fn require_type(actual: MajorType, wanted: MajorType) -> Result<(), CborError> {
    if actual == wanted {
        Ok(())
    } else {
        Err(error(ErrorReason::WrongType))
    }
}
