// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::error::{error, CborError, ErrorReason};
use super::limits::Budget;
use super::profile::LengthPolicy;
use super::read_header::MajorType;
use super::receipt::Frame;
use super::scan::{scan, tail};

/// Compare each key with its predecessors before reading its value. The scan
/// and comparison costs follow the COSE preflight, including skipping an
/// earlier value only after its key has been compared.
pub(super) fn check_previous_keys(
    mut previous: &[u8],
    key: &Frame<'_>,
    budget: &mut Budget,
) -> Result<(), CborError> {
    while !previous.is_empty() {
        let candidate = next(&mut previous, budget)?;
        if equivalent(&candidate, key, budget)? {
            return Err(error(ErrorReason::DuplicateMapKey));
        }
        next(&mut previous, budget)?;
    }
    Ok(())
}

fn next<'a>(remaining: &mut &'a [u8], budget: &mut Budget) -> Result<Frame<'a>, CborError> {
    // The source was already checked during the first traversal. Only the
    // structural scan and work budgets apply to these borrowed re-reads.
    let item = scan(
        remaining,
        budget,
        0,
        None,
        false,
        LengthPolicy::AllowIndefinite,
    )?;
    *remaining = tail(remaining, item.received_bytes().len())?;
    Ok(item)
}

fn equivalent(left: &Frame<'_>, right: &Frame<'_>, budget: &mut Budget) -> Result<bool, CborError> {
    let left_bytes = left.received_bytes();
    let right_bytes = right.received_bytes();
    budget.encoded_key_compare_bytes(left_bytes.len(), right_bytes.len())?;
    if left_bytes == right_bytes {
        return Ok(true);
    }
    let major = left.header.major_type();
    if major != right.header.major_type() {
        return Ok(false);
    }
    let left_argument = left.header.argument();
    let right_argument = right.header.argument();
    if matches!((left_argument, right_argument), (Some(a), Some(b)) if a != b) {
        return Ok(false);
    }
    // Scalar encodings retain signed zeros, NaN signs, and received widths.
    // Compound framing and map order do not participate in equality.
    match major {
        MajorType::Map => {
            if (left_argument.is_none() || right_argument.is_none())
                && map_entries(left, budget)? != map_entries(right, budget)?
            {
                return Ok(false);
            }
            maps_equal(left.body, right.body, budget)
        }
        MajorType::Array | MajorType::Tag => children_equal(left.body, right.body, budget),
        _ => Ok(false),
    }
}

fn children_equal(
    mut left: &[u8],
    mut right: &[u8],
    budget: &mut Budget,
) -> Result<bool, CborError> {
    while !left.is_empty() && !right.is_empty() {
        let first = next(&mut left, budget)?;
        let second = next(&mut right, budget)?;
        if !equivalent(&first, &second, budget)? {
            return Ok(false);
        }
    }
    Ok(left.is_empty() && right.is_empty())
}

fn map_entries(frame: &Frame<'_>, budget: &mut Budget) -> Result<u64, CborError> {
    if let Some(count) = frame.header.argument() {
        return Ok(count);
    }
    let mut remaining = frame.body;
    let mut entries = 0_u64;
    while !remaining.is_empty() {
        next(&mut remaining, budget)?;
        next(&mut remaining, budget)?;
        entries = entries
            .checked_add(1)
            .ok_or(error(ErrorReason::ArithmeticOverflow))?;
    }
    Ok(entries)
}

fn maps_equal(mut left: &[u8], right: &[u8], budget: &mut Budget) -> Result<bool, CborError> {
    while !left.is_empty() {
        let key = next(&mut left, budget)?;
        let value = next(&mut left, budget)?;
        let mut candidates = right;
        let mut found = false;
        while !candidates.is_empty() {
            let other_key = next(&mut candidates, budget)?;
            let other_value = next(&mut candidates, budget)?;
            if equivalent(&key, &other_key, budget)? && equivalent(&value, &other_value, budget)? {
                found = true;
                break;
            }
        }
        if !found {
            return Ok(false);
        }
    }
    Ok(true)
}
