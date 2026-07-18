// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use crate::{
    deterministic::try_vec_with_capacity, DeterministicCborError, DeterministicCborInteger,
    DeterministicCborMapEntry, DeterministicCborMapKey, DeterministicCborValue,
    MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES, MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES,
    MAX_DETERMINISTIC_CBOR_NESTING_DEPTH, MAX_DETERMINISTIC_CBOR_NODES,
    MAX_DETERMINISTIC_CBOR_OUTPUT_LEN,
};
use std::cmp::Ordering;
use zeroize::{Zeroize, Zeroizing};

const MT_UINT: u8 = 0;
const MT_NEGINT: u8 = 1;
const MT_BYTES: u8 = 2;
const MT_STRING: u8 = 3;
const MT_ARRAY: u8 = 4;
const MT_MAP: u8 = 5;

/// Encode a value using the deterministic generic-CBOR profile.
///
/// The encoder validates limits and computes the exact output length before
/// allocating the result. Temporary encoded map-key buffers are zeroized after
/// sorting and duplicate detection.
pub fn encode_deterministic_cbor(
    value: &DeterministicCborValue,
) -> Result<Zeroizing<Vec<u8>>, DeterministicCborError> {
    let preflight = preflight_value(value, 0)?;
    if preflight.encoded_len > MAX_DETERMINISTIC_CBOR_OUTPUT_LEN {
        return Err(DeterministicCborError::OutputTooLarge);
    }

    let mut output = Zeroizing::new(try_vec_with_capacity(preflight.encoded_len)?);
    encode_value(value, &mut output, 0)?;
    if output.len() != preflight.encoded_len {
        return Err(DeterministicCborError::PreflightLengthMismatch);
    }
    Ok(output)
}

#[derive(Clone, Copy, Default)]
struct Preflight {
    encoded_len: usize,
    nodes: usize,
    aggregate_text_bytes: usize,
    aggregate_byte_string_bytes: usize,
}

impl Preflight {
    fn node(encoded_len: usize) -> Result<Self, DeterministicCborError> {
        Ok(Self {
            encoded_len,
            nodes: 1,
            aggregate_text_bytes: 0,
            aggregate_byte_string_bytes: 0,
        })
    }

    fn text(encoded_len: usize, byte_len: usize) -> Result<Self, DeterministicCborError> {
        let mut stats = Self::node(encoded_len)?;
        stats.aggregate_text_bytes = byte_len;
        stats.ensure_limits()?;
        Ok(stats)
    }

    fn bytes(encoded_len: usize, byte_len: usize) -> Result<Self, DeterministicCborError> {
        let mut stats = Self::node(encoded_len)?;
        stats.aggregate_byte_string_bytes = byte_len;
        stats.ensure_limits()?;
        Ok(stats)
    }

    fn add(&mut self, other: Self) -> Result<(), DeterministicCborError> {
        self.encoded_len = checked_add(self.encoded_len, other.encoded_len)?;
        self.nodes = checked_add(self.nodes, other.nodes)?;
        self.aggregate_text_bytes =
            checked_add(self.aggregate_text_bytes, other.aggregate_text_bytes)?;
        self.aggregate_byte_string_bytes = checked_add(
            self.aggregate_byte_string_bytes,
            other.aggregate_byte_string_bytes,
        )?;
        self.ensure_limits()
    }

    fn ensure_limits(&self) -> Result<(), DeterministicCborError> {
        if self.encoded_len > MAX_DETERMINISTIC_CBOR_OUTPUT_LEN {
            return Err(DeterministicCborError::OutputTooLarge);
        }
        if self.nodes > MAX_DETERMINISTIC_CBOR_NODES {
            return Err(DeterministicCborError::NodeLimitExceeded);
        }
        if self.aggregate_text_bytes > MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES {
            return Err(DeterministicCborError::AggregateTextBytesExceeded);
        }
        if self.aggregate_byte_string_bytes > MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES {
            return Err(DeterministicCborError::AggregateByteStringBytesExceeded);
        }
        Ok(())
    }
}

fn preflight_value(
    value: &DeterministicCborValue,
    depth: usize,
) -> Result<Preflight, DeterministicCborError> {
    match value {
        DeterministicCborValue::Null | DeterministicCborValue::Bool(_) => Preflight::node(1),
        DeterministicCborValue::Integer(integer) => preflight_integer(integer),
        DeterministicCborValue::Text(text) => {
            let byte_len = text.len();
            Preflight::text(
                checked_add(header_len(len_as_u64(byte_len)?)?, byte_len)?,
                byte_len,
            )
        }
        DeterministicCborValue::Bytes(bytes) => {
            let byte_len = bytes.len();
            Preflight::bytes(
                checked_add(header_len(len_as_u64(byte_len)?)?, byte_len)?,
                byte_len,
            )
        }
        DeterministicCborValue::Array(values) => {
            let child_depth = descend(depth)?;
            ensure_container_entries(values.len())?;
            let mut stats = Preflight::node(header_len(len_as_u64(values.len())?)?)?;
            for child in values {
                stats.add(preflight_value(child, child_depth)?)?;
            }
            Ok(stats)
        }
        DeterministicCborValue::Map(entries) => {
            let child_depth = descend(depth)?;
            ensure_container_entries(entries.len())?;
            let mut stats = Preflight::node(header_len(len_as_u64(entries.len())?)?)?;
            for entry in entries {
                stats.add(preflight_key(entry.key())?)?;
                stats.add(preflight_value(entry.value(), child_depth)?)?;
            }

            // Establish the aggregate tree budgets before copying any map key
            // into a sortable owner. Bounding each key independently is not
            // enough: a caller can otherwise supply many individually valid,
            // large keys and force substantial temporary allocation before
            // the aggregate text/output limit rejects the map.
            let mut decorated = decorated_map_entries(entries)?;
            decorated.sort_by(compare_decorated_keys);
            reject_duplicate_decorated_keys(&decorated)?;
            Ok(stats)
        }
    }
}

fn preflight_key(key: &DeterministicCborMapKey) -> Result<Preflight, DeterministicCborError> {
    match key {
        DeterministicCborMapKey::Integer(integer) => preflight_integer(integer),
        DeterministicCborMapKey::Text(text) => {
            let byte_len = text.len();
            Preflight::text(
                checked_add(header_len(len_as_u64(byte_len)?)?, byte_len)?,
                byte_len,
            )
        }
    }
}

fn preflight_integer(
    integer: &DeterministicCborInteger,
) -> Result<Preflight, DeterministicCborError> {
    match integer {
        DeterministicCborInteger::Unsigned(value) => Preflight::node(header_len(*value)?),
        DeterministicCborInteger::Negative(value) => {
            let encoded = negative_argument(value.value())?;
            Preflight::node(header_len(encoded)?)
        }
    }
}

struct DecoratedMapEntry<'a> {
    encoded_key: Zeroizing<Vec<u8>>,
    entry: &'a DeterministicCborMapEntry,
}

fn decorated_map_entries(
    entries: &[DeterministicCborMapEntry],
) -> Result<Vec<DecoratedMapEntry<'_>>, DeterministicCborError> {
    let mut decorated = try_vec_with_capacity(entries.len())?;
    for entry in entries {
        // Pre-size every sensitive temporary from the same semantic preflight
        // used by the encoder. A growing Vec can leave superseded key bytes in
        // freed allocator blocks that the final Zeroizing owner cannot wipe.
        let key_stats = preflight_key(entry.key())?;
        let mut encoded_key = Zeroizing::new(try_vec_with_capacity(key_stats.encoded_len)?);
        encode_key(entry.key(), &mut encoded_key)?;
        if encoded_key.len() != key_stats.encoded_len {
            return Err(DeterministicCborError::PreflightLengthMismatch);
        }
        decorated.push(DecoratedMapEntry { encoded_key, entry });
    }
    Ok(decorated)
}

fn compare_decorated_keys(left: &DecoratedMapEntry<'_>, right: &DecoratedMapEntry<'_>) -> Ordering {
    compare_encoded_keys(&left.encoded_key, &right.encoded_key)
}

fn compare_encoded_keys(left: &[u8], right: &[u8]) -> Ordering {
    match left.len().cmp(&right.len()) {
        Ordering::Equal => left.cmp(right),
        ordering => ordering,
    }
}

fn reject_duplicate_decorated_keys(
    decorated: &[DecoratedMapEntry<'_>],
) -> Result<(), DeterministicCborError> {
    for pair in decorated.windows(2) {
        if pair[0].encoded_key.as_slice() == pair[1].encoded_key.as_slice() {
            return Err(DeterministicCborError::DuplicateMapKey);
        }
    }
    Ok(())
}

fn encode_value(
    value: &DeterministicCborValue,
    output: &mut Vec<u8>,
    depth: usize,
) -> Result<(), DeterministicCborError> {
    match value {
        DeterministicCborValue::Null => push_byte(output, 0xf6),
        DeterministicCborValue::Bool(false) => push_byte(output, 0xf4),
        DeterministicCborValue::Bool(true) => push_byte(output, 0xf5),
        DeterministicCborValue::Integer(integer) => encode_integer(integer, output),
        DeterministicCborValue::Text(text) => encode_text(text, output),
        DeterministicCborValue::Bytes(bytes) => encode_bytes(bytes, output),
        DeterministicCborValue::Array(values) => {
            let child_depth = descend(depth)?;
            ensure_container_entries(values.len())?;
            write_header(MT_ARRAY, len_as_u64(values.len())?, output)?;
            for child in values {
                encode_value(child, output, child_depth)?;
            }
            Ok(())
        }
        DeterministicCborValue::Map(entries) => {
            let child_depth = descend(depth)?;
            ensure_container_entries(entries.len())?;
            let mut decorated = decorated_map_entries(entries)?;
            decorated.sort_by(compare_decorated_keys);
            reject_duplicate_decorated_keys(&decorated)?;

            write_header(MT_MAP, len_as_u64(decorated.len())?, output)?;
            for decorated_entry in decorated {
                extend_bytes(output, &decorated_entry.encoded_key)?;
                encode_value(decorated_entry.entry.value(), output, child_depth)?;
            }
            Ok(())
        }
    }
}

fn encode_key(
    key: &DeterministicCborMapKey,
    output: &mut Vec<u8>,
) -> Result<(), DeterministicCborError> {
    match key {
        DeterministicCborMapKey::Integer(integer) => encode_integer(integer, output),
        DeterministicCborMapKey::Text(text) => encode_text(text, output),
    }
}

fn encode_integer(
    integer: &DeterministicCborInteger,
    output: &mut Vec<u8>,
) -> Result<(), DeterministicCborError> {
    match integer {
        DeterministicCborInteger::Unsigned(value) => write_header(MT_UINT, *value, output),
        DeterministicCborInteger::Negative(value) => {
            write_header(MT_NEGINT, negative_argument(value.value())?, output)
        }
    }
}

fn encode_text(text: &str, output: &mut Vec<u8>) -> Result<(), DeterministicCborError> {
    let bytes = text.as_bytes();
    write_header(MT_STRING, len_as_u64(bytes.len())?, output)?;
    extend_bytes(output, bytes)
}

fn encode_bytes(bytes: &[u8], output: &mut Vec<u8>) -> Result<(), DeterministicCborError> {
    write_header(MT_BYTES, len_as_u64(bytes.len())?, output)?;
    extend_bytes(output, bytes)
}

fn negative_argument(value: i64) -> Result<u64, DeterministicCborError> {
    let argument = (-1_i128)
        .checked_sub(i128::from(value))
        .ok_or(DeterministicCborError::OffsetOverflow)?;
    u64::try_from(argument).map_err(|_| DeterministicCborError::NegativeIntegerOutOfRange)
}

fn len_as_u64(len: usize) -> Result<u64, DeterministicCborError> {
    u64::try_from(len).map_err(|_| DeterministicCborError::LengthTooLarge)
}

fn header_len(value: u64) -> Result<usize, DeterministicCborError> {
    if value < 24 {
        Ok(1)
    } else if value < 0x100 {
        Ok(2)
    } else if value < 0x1_0000 {
        Ok(3)
    } else if value < 0x1_0000_0000 {
        Ok(5)
    } else {
        Ok(9)
    }
}

fn write_header(
    major_type: u8,
    value: u64,
    output: &mut Vec<u8>,
) -> Result<(), DeterministicCborError> {
    let be = value.to_be_bytes();
    let head = major_type << 5;
    if value < 24 {
        push_byte(output, head | be[7])
    } else if value < 0x100 {
        push_byte(output, head | 24)?;
        extend_bytes(output, &be[7..8])
    } else if value < 0x1_0000 {
        push_byte(output, head | 25)?;
        extend_bytes(output, &be[6..8])
    } else if value < 0x1_0000_0000 {
        push_byte(output, head | 26)?;
        extend_bytes(output, &be[4..8])
    } else {
        push_byte(output, head | 27)?;
        extend_bytes(output, &be)
    }
}

fn push_byte(output: &mut Vec<u8>, byte: u8) -> Result<(), DeterministicCborError> {
    let next_len = checked_add(output.len(), 1)?;
    if next_len > MAX_DETERMINISTIC_CBOR_OUTPUT_LEN {
        return Err(DeterministicCborError::OutputTooLarge);
    }
    output.push(byte);
    Ok(())
}

fn extend_bytes(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), DeterministicCborError> {
    let next_len = checked_add(output.len(), bytes.len())?;
    if next_len > MAX_DETERMINISTIC_CBOR_OUTPUT_LEN {
        return Err(DeterministicCborError::OutputTooLarge);
    }
    output.extend_from_slice(bytes);
    Ok(())
}

fn descend(depth: usize) -> Result<usize, DeterministicCborError> {
    let next = checked_add(depth, 1)?;
    if next > MAX_DETERMINISTIC_CBOR_NESTING_DEPTH {
        return Err(DeterministicCborError::DepthExceeded);
    }
    Ok(next)
}

fn ensure_container_entries(entries: usize) -> Result<(), DeterministicCborError> {
    if entries > MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES {
        return Err(DeterministicCborError::ContainerEntriesExceeded);
    }
    Ok(())
}

fn checked_add(left: usize, right: usize) -> Result<usize, DeterministicCborError> {
    left.checked_add(right)
        .ok_or(DeterministicCborError::OffsetOverflow)
}

impl Drop for DecoratedMapEntry<'_> {
    fn drop(&mut self) {
        self.encoded_key.zeroize();
    }
}
