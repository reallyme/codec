// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use crate::{
    deterministic::{try_string_copy, try_vec_with_capacity},
    DeterministicCborError, DeterministicCborInteger, DeterministicCborMapEntry,
    DeterministicCborMapKey, DeterministicCborValue,
    MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES, MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES,
    MAX_DETERMINISTIC_CBOR_INPUT_LEN, MAX_DETERMINISTIC_CBOR_NESTING_DEPTH,
    MAX_DETERMINISTIC_CBOR_NODES,
};
use std::cmp::Ordering;
use std::str;

const MT_UINT: u8 = 0;
const MT_NEGINT: u8 = 1;
const MT_BYTES: u8 = 2;
const MT_STRING: u8 = 3;
const MT_ARRAY: u8 = 4;
const MT_MAP: u8 = 5;
const MT_TAG: u8 = 6;
const MT_SIMPLE: u8 = 7;
const SIMPLE_FALSE: u64 = 20;
const SIMPLE_TRUE: u64 = 21;
const SIMPLE_NULL: u64 = 22;
const MIN_ELEMENT_ENCODED_LEN: usize = 1;

/// Decode deterministic generic-CBOR bytes into the supported semantic value.
///
/// The decoder is strict: non-minimal integer/length encodings, indefinite
/// lengths, unsupported major types, duplicate keys, out-of-order keys, invalid
/// UTF-8, trailing bytes, and limit violations all fail closed.
pub fn decode_deterministic_cbor(
    bytes: &[u8],
) -> Result<DeterministicCborValue, DeterministicCborError> {
    if bytes.len() > MAX_DETERMINISTIC_CBOR_INPUT_LEN {
        return Err(DeterministicCborError::InputTooLarge);
    }
    let mut limits = DecodeLimits::default();
    let (value, offset) = decode_value(bytes, 0, 0, &mut limits)?;
    if offset != bytes.len() {
        return Err(DeterministicCborError::TrailingBytes);
    }
    Ok(value)
}

#[derive(Default)]
struct DecodeLimits {
    nodes: usize,
    aggregate_text_bytes: usize,
    aggregate_byte_string_bytes: usize,
}

impl DecodeLimits {
    fn add_node(&mut self) -> Result<(), DeterministicCborError> {
        self.nodes = checked_add(self.nodes, 1)?;
        if self.nodes > MAX_DETERMINISTIC_CBOR_NODES {
            return Err(DeterministicCborError::NodeLimitExceeded);
        }
        Ok(())
    }

    fn add_text_bytes(&mut self, len: usize) -> Result<(), DeterministicCborError> {
        self.aggregate_text_bytes = checked_add(self.aggregate_text_bytes, len)?;
        if self.aggregate_text_bytes > MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES {
            return Err(DeterministicCborError::AggregateTextBytesExceeded);
        }
        Ok(())
    }

    fn add_byte_string_bytes(&mut self, len: usize) -> Result<(), DeterministicCborError> {
        self.aggregate_byte_string_bytes = checked_add(self.aggregate_byte_string_bytes, len)?;
        if self.aggregate_byte_string_bytes > MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES {
            return Err(DeterministicCborError::AggregateByteStringBytesExceeded);
        }
        Ok(())
    }
}

fn decode_value(
    bytes: &[u8],
    offset: usize,
    depth: usize,
    limits: &mut DecodeLimits,
) -> Result<(DeterministicCborValue, usize), DeterministicCborError> {
    limits.add_node()?;
    let (major, argument, mut offset) = read_head(bytes, offset)?;

    match major {
        MT_UINT => Ok((
            DeterministicCborValue::Integer(DeterministicCborInteger::unsigned(argument)),
            offset,
        )),
        MT_NEGINT => Ok((
            DeterministicCborValue::Integer(DeterministicCborInteger::negative(negative_value(
                argument,
            )?)?),
            offset,
        )),
        MT_BYTES => {
            let (value, next) = extract_bytes(bytes, offset, argument, limits)?;
            Ok((DeterministicCborValue::Bytes(value), next))
        }
        MT_STRING => {
            let (value, next) = extract_string(bytes, offset, argument, limits)?;
            Ok((DeterministicCborValue::Text(value), next))
        }
        MT_ARRAY => {
            let child_depth = descend(depth)?;
            let item_count = container_count(argument)?;
            bounded_capacity(item_count, bytes.len(), offset, MIN_ELEMENT_ENCODED_LEN)?;
            // The declared count has already been bounded both semantically
            // and against the remaining input. Exact allocation prevents Vec
            // growth from abandoning earlier identity-bearing owners in
            // allocator blocks that the final owner cannot wipe.
            let mut values = try_vec_with_capacity(item_count)?;
            for _ in 0..item_count {
                let (value, next) = decode_value(bytes, offset, child_depth, limits)?;
                values.push(value);
                offset = next;
            }
            Ok((DeterministicCborValue::Array(values), offset))
        }
        MT_MAP => {
            let child_depth = descend(depth)?;
            let entry_count = container_count(argument)?;
            let min_entry_len = checked_mul(MIN_ELEMENT_ENCODED_LEN, 2)?;
            bounded_capacity(entry_count, bytes.len(), offset, min_entry_len)?;
            let mut entries = try_vec_with_capacity(entry_count)?;
            let mut previous_key_range: Option<(usize, usize)> = None;

            for _ in 0..entry_count {
                let key_start = offset;
                let (key, key_end) = decode_key(bytes, offset, limits)?;
                if let Some((previous_start, previous_end)) = previous_key_range {
                    let previous_key = checked_slice(bytes, previous_start, previous_end)?;
                    let current_key = checked_slice(bytes, key_start, key_end)?;
                    match compare_encoded_keys(previous_key, current_key) {
                        Ordering::Less => {}
                        Ordering::Equal => return Err(DeterministicCborError::DuplicateMapKey),
                        Ordering::Greater => {
                            return Err(DeterministicCborError::MapKeysOutOfOrder);
                        }
                    }
                }
                previous_key_range = Some((key_start, key_end));
                offset = key_end;

                let (value, next) = decode_value(bytes, offset, child_depth, limits)?;
                entries.push(DeterministicCborMapEntry::new(key, value));
                offset = next;
            }
            Ok((DeterministicCborValue::Map(entries), offset))
        }
        MT_SIMPLE => match argument {
            SIMPLE_FALSE => Ok((DeterministicCborValue::Bool(false), offset)),
            SIMPLE_TRUE => Ok((DeterministicCborValue::Bool(true), offset)),
            SIMPLE_NULL => Ok((DeterministicCborValue::Null, offset)),
            _ => Err(DeterministicCborError::UnsupportedSimpleValue),
        },
        _ => Err(DeterministicCborError::UnsupportedMajorType),
    }
}

fn decode_key(
    bytes: &[u8],
    offset: usize,
    limits: &mut DecodeLimits,
) -> Result<(DeterministicCborMapKey, usize), DeterministicCborError> {
    limits.add_node()?;
    let (major, argument, offset) = read_head(bytes, offset)?;
    match major {
        MT_UINT => Ok((
            DeterministicCborMapKey::Integer(DeterministicCborInteger::unsigned(argument)),
            offset,
        )),
        MT_NEGINT => Ok((
            DeterministicCborMapKey::Integer(DeterministicCborInteger::negative(negative_value(
                argument,
            )?)?),
            offset,
        )),
        MT_STRING => {
            let (value, next) = extract_string(bytes, offset, argument, limits)?;
            Ok((DeterministicCborMapKey::text(value), next))
        }
        _ => Err(DeterministicCborError::UnsupportedMapKeyType),
    }
}

fn read_head(bytes: &[u8], offset: usize) -> Result<(u8, u64, usize), DeterministicCborError> {
    if offset >= bytes.len() {
        return Err(DeterministicCborError::UnexpectedEnd);
    }
    let first = *bytes
        .get(offset)
        .ok_or(DeterministicCborError::UnexpectedEnd)?;
    let next_offset = checked_add(offset, 1)?;
    let major = first >> 5;
    let additional = first & 0x1f;
    // Tags and floating-point/simple extensions are outside this closed
    // profile. Reject them from the initial byte alone so float payload bits
    // are never misinterpreted as an integer argument and misclassified as a
    // canonical-integer failure.
    if major == MT_TAG {
        return Err(DeterministicCborError::UnsupportedMajorType);
    }
    if major == MT_SIMPLE && additional >= 24 {
        return match additional {
            24..=27 => Err(DeterministicCborError::UnsupportedSimpleValue),
            _ => Err(DeterministicCborError::UnsupportedAdditionalInfo),
        };
    }
    let (argument, after_argument) = read_argument(bytes, next_offset, additional)?;
    Ok((major, argument, after_argument))
}

fn read_argument(
    bytes: &[u8],
    offset: usize,
    additional: u8,
) -> Result<(u64, usize), DeterministicCborError> {
    match additional {
        value @ 0..=23 => Ok((u64::from(value), offset)),
        24 => {
            let end = checked_end(offset, 1)?;
            let value = u64::from(
                *bytes
                    .get(offset)
                    .ok_or(DeterministicCborError::TruncatedArgument)?,
            );
            if value < 24 {
                return Err(DeterministicCborError::NonCanonicalInteger);
            }
            Ok((value, end))
        }
        25 => {
            let end = checked_end(offset, 2)?;
            let encoded = checked_argument_slice(bytes, offset, end)?;
            let value = u16::from_be_bytes(
                <[u8; 2]>::try_from(encoded)
                    .map_err(|_| DeterministicCborError::TruncatedArgument)?,
            );
            if value < 0x100 {
                return Err(DeterministicCborError::NonCanonicalInteger);
            }
            Ok((u64::from(value), end))
        }
        26 => {
            let end = checked_end(offset, 4)?;
            let encoded = checked_argument_slice(bytes, offset, end)?;
            let value = u32::from_be_bytes(
                <[u8; 4]>::try_from(encoded)
                    .map_err(|_| DeterministicCborError::TruncatedArgument)?,
            );
            if value < 0x1_0000 {
                return Err(DeterministicCborError::NonCanonicalInteger);
            }
            Ok((u64::from(value), end))
        }
        27 => {
            let end = checked_end(offset, 8)?;
            let encoded = checked_argument_slice(bytes, offset, end)?;
            let value = u64::from_be_bytes(
                <[u8; 8]>::try_from(encoded)
                    .map_err(|_| DeterministicCborError::TruncatedArgument)?,
            );
            if value < 0x1_0000_0000 {
                return Err(DeterministicCborError::NonCanonicalInteger);
            }
            Ok((value, end))
        }
        _ => Err(DeterministicCborError::UnsupportedAdditionalInfo),
    }
}

fn extract_bytes(
    bytes: &[u8],
    offset: usize,
    len: u64,
    limits: &mut DecodeLimits,
) -> Result<(Vec<u8>, usize), DeterministicCborError> {
    let len = usize::try_from(len).map_err(|_| DeterministicCborError::LengthTooLarge)?;
    limits.add_byte_string_bytes(len)?;
    let end = checked_end(offset, len)?;
    let encoded =
        checked_slice(bytes, offset, end).map_err(|_| DeterministicCborError::TruncatedBytes)?;
    let mut value = try_vec_with_capacity(len)?;
    value.extend_from_slice(encoded);
    Ok((value, end))
}

fn extract_string(
    bytes: &[u8],
    offset: usize,
    len: u64,
    limits: &mut DecodeLimits,
) -> Result<(String, usize), DeterministicCborError> {
    let len = usize::try_from(len).map_err(|_| DeterministicCborError::LengthTooLarge)?;
    limits.add_text_bytes(len)?;
    let end = checked_end(offset, len)?;
    let encoded =
        checked_slice(bytes, offset, end).map_err(|_| DeterministicCborError::TruncatedBytes)?;
    let text =
        try_string_copy(str::from_utf8(encoded).map_err(|_| DeterministicCborError::InvalidUtf8)?)?;
    Ok((text, end))
}

fn negative_value(argument: u64) -> Result<i64, DeterministicCborError> {
    let value = (-1_i128)
        .checked_sub(i128::from(argument))
        .ok_or(DeterministicCborError::NegativeIntegerOutOfRange)?;
    i64::try_from(value).map_err(|_| DeterministicCborError::NegativeIntegerOutOfRange)
}

fn container_count(argument: u64) -> Result<usize, DeterministicCborError> {
    let count = usize::try_from(argument).map_err(|_| DeterministicCborError::LengthTooLarge)?;
    if count > MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES {
        return Err(DeterministicCborError::ContainerEntriesExceeded);
    }
    Ok(count)
}

fn bounded_capacity(
    count: usize,
    total_len: usize,
    offset: usize,
    min_element_len: usize,
) -> Result<(), DeterministicCborError> {
    let remaining = checked_sub(total_len, offset)?;
    let max_possible = remaining / min_element_len.max(1);
    if count > max_possible {
        return Err(DeterministicCborError::ContainerLengthExceedsInput);
    }
    Ok(())
}

fn compare_encoded_keys(left: &[u8], right: &[u8]) -> Ordering {
    match left.len().cmp(&right.len()) {
        Ordering::Equal => left.cmp(right),
        ordering => ordering,
    }
}

fn descend(depth: usize) -> Result<usize, DeterministicCborError> {
    let next = checked_add(depth, 1)?;
    if next > MAX_DETERMINISTIC_CBOR_NESTING_DEPTH {
        return Err(DeterministicCborError::DepthExceeded);
    }
    Ok(next)
}

fn checked_end(offset: usize, len: usize) -> Result<usize, DeterministicCborError> {
    checked_add(offset, len)
}

fn checked_argument_slice(
    bytes: &[u8],
    start: usize,
    end: usize,
) -> Result<&[u8], DeterministicCborError> {
    checked_slice(bytes, start, end).map_err(|_| DeterministicCborError::TruncatedArgument)
}

fn checked_slice(bytes: &[u8], start: usize, end: usize) -> Result<&[u8], DeterministicCborError> {
    bytes
        .get(start..end)
        .ok_or(DeterministicCborError::OffsetOverflow)
}

fn checked_add(left: usize, right: usize) -> Result<usize, DeterministicCborError> {
    left.checked_add(right)
        .ok_or(DeterministicCborError::OffsetOverflow)
}

fn checked_sub(left: usize, right: usize) -> Result<usize, DeterministicCborError> {
    left.checked_sub(right)
        .ok_or(DeterministicCborError::OffsetOverflow)
}

fn checked_mul(left: usize, right: usize) -> Result<usize, DeterministicCborError> {
    left.checked_mul(right)
        .ok_or(DeterministicCborError::OffsetOverflow)
}
