// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::compare::{equivalent, ordered};
use super::error::{error, CborError, ErrorReason};
use super::limits::Budget;
use super::profile::{LengthPolicy, MapOrdering, Profile, Serialization};
use super::read_header::{read_header_prefix, ArgumentWidth, Header, MajorType};
use super::receipt::Frame;

pub(super) fn add(left: usize, right: usize) -> Result<usize, CborError> {
    left.checked_add(right)
        .ok_or(error(ErrorReason::ArithmeticOverflow))
}

pub(super) fn tail(input: &[u8], offset: usize) -> Result<&[u8], CborError> {
    input.get(offset..).ok_or(error(ErrorReason::Truncated))
}

fn argument(header: &Header<'_>) -> Result<usize, CborError> {
    usize::try_from(header.argument().ok_or(error(ErrorReason::WrongType))?)
        .map_err(|_| error(ErrorReason::ArithmeticOverflow))
}

fn check_header(header: &Header<'_>, profile: Option<Profile>) -> Result<(), CborError> {
    if let Some(profile) = profile {
        if let Some(value) = header.float()? {
            value.check(profile.floats)?;
            if profile.serialization == Serialization::Preferred
                && value.width() != value.shortest()
            {
                return Err(error(ErrorReason::NonPreferredFloat));
            }
        } else if profile.serialization == Serialization::Preferred
            && header.has_preferred_argument() == Some(false)
        {
            return Err(error(ErrorReason::NonPreferredArgument));
        }
    }
    Ok(())
}

/// All reparsing for key comparison goes through the same syntax walker, but
/// does not count the same semantic node or string contents twice. Work credits
/// are never reset: nested compound keys cannot hide quadratic comparisons.
pub(super) fn scan<'a>(
    input: &'a [u8],
    budget: &mut Budget,
    depth: usize,
    profile: Option<Profile>,
    account: bool,
    lengths: LengthPolicy,
) -> Result<Frame<'a>, CborError> {
    budget.charge(1)?;
    let header = read_header_prefix(input)?;
    budget.charge(header.received_bytes().len())?;
    if header.argument_width() == ArgumentWidth::Break {
        return Err(error(ErrorReason::UnexpectedBreak));
    }
    if account {
        budget.node()?;
    }
    check_header(&header, profile)?;
    let indefinite = header.argument_width() == ArgumentWidth::Indefinite;
    if indefinite && lengths == LengthPolicy::DefiniteOnly {
        return Err(error(ErrorReason::IndefiniteNotAllowed));
    }
    let start = header.received_bytes().len();
    let mut end = start;
    let mut body_end = start;
    match header.major_type() {
        MajorType::Unsigned | MajorType::Negative | MajorType::Simple => {}
        MajorType::Bytes | MajorType::Text if !indefinite => {
            let length = argument(&header)?;
            end = add(start, length)?;
            let bytes = input.get(start..end).ok_or(error(ErrorReason::Truncated))?;
            if account {
                budget.contents(length, header.major_type() == MajorType::Text)?;
            }
            if profile.is_some() && header.major_type() == MajorType::Text {
                budget.charge(length)?;
                core::str::from_utf8(bytes).map_err(|_| error(ErrorReason::InvalidUtf8))?;
            }
            body_end = end;
        }
        MajorType::Bytes | MajorType::Text => {
            let next_depth = budget.enter(depth)?;
            let mut chunks = 0_usize;
            loop {
                let remaining = tail(input, end)?;
                if remaining.first() == Some(&0xff) {
                    body_end = end;
                    end = add(end, 1)?;
                    break;
                }
                if remaining.is_empty() {
                    return Err(error(ErrorReason::Truncated));
                }
                chunks = add(chunks, 1)?;
                if chunks > budget.limits.entries {
                    return Err(error(ErrorReason::EntryLimit));
                }
                // A chunk may not itself be indefinite, including an empty one.
                let chunk_header = read_header_prefix(remaining)?;
                budget.charge(chunk_header.received_bytes().len())?;
                if chunk_header.major_type() != header.major_type()
                    || chunk_header.argument_width() == ArgumentWidth::Indefinite
                {
                    return Err(error(ErrorReason::InvalidChunk));
                }
                let chunk = scan(
                    remaining,
                    budget,
                    next_depth,
                    profile,
                    account,
                    LengthPolicy::DefiniteOnly,
                )?;
                end = add(end, chunk.received_bytes().len())?;
            }
            budget.charge(1)?;
        }
        MajorType::Tag => {
            let next_depth = budget.enter(depth)?;
            let child = scan(
                tail(input, end)?,
                budget,
                next_depth,
                profile,
                account,
                lengths,
            )?;
            end = add(end, child.received_bytes().len())?;
            body_end = end;
        }
        MajorType::Array | MajorType::Map => {
            let next_depth = budget.enter(depth)?;
            let map = header.major_type() == MajorType::Map;
            let declared = if indefinite {
                None
            } else {
                Some(argument(&header)?)
            };
            if let Some(count) = declared {
                if count > budget.limits.entries {
                    return Err(error(ErrorReason::EntryLimit));
                }
                let children = if map {
                    count
                        .checked_mul(2)
                        .ok_or(error(ErrorReason::ArithmeticOverflow))?
                } else {
                    count
                };
                if account {
                    budget.declared_nodes(children)?;
                }
                if children > tail(input, start)?.len() {
                    return Err(error(ErrorReason::ContainerExceedsInput));
                }
            }
            let mut count = 0_usize;
            let mut previous: Option<Frame<'_>> = None;
            loop {
                if declared == Some(count) {
                    body_end = end;
                    break;
                }
                let remaining = tail(input, end)?;
                if indefinite && remaining.first() == Some(&0xff) {
                    body_end = end;
                    end = add(end, 1)?;
                    budget.charge(1)?;
                    break;
                }
                if remaining.is_empty() {
                    return Err(error(ErrorReason::Truncated));
                }
                count = add(count, 1)?;
                if count > budget.limits.entries {
                    return Err(error(ErrorReason::EntryLimit));
                }
                let key_start = end;
                let child = scan(remaining, budget, next_depth, profile, account, lengths)?;
                end = add(end, child.received_bytes().len())?;
                if map {
                    if let Some(profile) = profile {
                        let mut prior = start;
                        while prior < key_start {
                            let key = scan(
                                tail(input, prior)?,
                                budget,
                                next_depth,
                                None,
                                false,
                                lengths,
                            )?;
                            if equivalent(&key, &child, budget, 0)? {
                                return Err(error(ErrorReason::DuplicateMapKey));
                            }
                            prior = add(prior, key.received_bytes().len())?;
                            let value = scan(
                                tail(input, prior)?,
                                budget,
                                next_depth,
                                None,
                                false,
                                lengths,
                            )?;
                            prior = add(prior, value.received_bytes().len())?;
                        }
                        // Uniqueness and ordering are separate guarantees. A
                        // duplicate remains a duplicate under an ordered profile.
                        if let Some(previous) = &previous {
                            if profile.ordering != MapOrdering::Received
                                && !ordered(previous, &child, profile.ordering, budget)?
                            {
                                return Err(error(ErrorReason::MapKeyOrder));
                            }
                        }
                    }
                    previous = Some(child);
                    let value = scan(
                        tail(input, end)?,
                        budget,
                        next_depth,
                        profile,
                        account,
                        lengths,
                    )?;
                    end = add(end, value.received_bytes().len())?;
                }
            }
        }
    }
    let encoded = input.get(..end).ok_or(error(ErrorReason::Truncated))?;
    let body = input
        .get(start..body_end)
        .ok_or(error(ErrorReason::Truncated))?;
    Ok(Frame {
        encoded,
        body,
        header,
        limits: budget.limits,
        lengths,
    })
}
