// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::check_map_key_order::{MapKeyOrder, MapKeyOrderError};
use crate::{
    CborError, CborValue, MAX_DAG_CBOR_CONTAINER_ENTRIES, MAX_DAG_CBOR_INPUT_LEN,
    MAX_DAG_CBOR_NODES, MAX_NESTING_DEPTH,
};
use std::str;
use zeroize::Zeroizing;

const MT_UINT: u8 = 0;
const MT_NEGINT: u8 = 1;
const MT_BYTES: u8 = 2;
const MT_STRING: u8 = 3;
const MT_ARRAY: u8 = 4;
const MT_MAP: u8 = 5;

/// Smallest possible encoding of one array element or map key/value: a
/// single header byte (e.g. a small integer, or an empty string/array).
/// Used to reject a declared container length that the remaining input
/// could never satisfy, before any capacity is reserved.
const MIN_ELEMENT_ENCODED_LEN: usize = 1;

#[derive(Default)]
struct DecodeBudget {
    seen: usize,
    pending: usize,
}

impl DecodeBudget {
    fn enter(&mut self) -> Result<(), CborError> {
        if self.seen != 0 {
            self.pending = self
                .pending
                .checked_sub(1)
                .ok_or(CborError::NodeLimitExceeded)?;
        }
        self.seen = self
            .seen
            .checked_add(1)
            .ok_or(CborError::NodeLimitExceeded)?;
        if self.seen > MAX_DAG_CBOR_NODES {
            return Err(CborError::NodeLimitExceeded);
        }
        Ok(())
    }

    fn reserve_children(&mut self, count: usize) -> Result<(), CborError> {
        let pending = self
            .pending
            .checked_add(count)
            .ok_or(CborError::NodeLimitExceeded)?;
        let total = self
            .seen
            .checked_add(pending)
            .ok_or(CborError::NodeLimitExceeded)?;
        if total > MAX_DAG_CBOR_NODES {
            return Err(CborError::NodeLimitExceeded);
        }
        self.pending = pending;
        Ok(())
    }
}

/// Decode canonical DAG-CBOR bytes into a CborValue.
///
/// This decoder is intentionally strict:
/// - rejects non-canonical encodings
/// - rejects floats, tags, and indefinite-length items
/// - enforces UTF-8 string map keys
/// - enforces canonical map key ordering
///
/// Suitable only for cryptographic / authoritative CBOR.
pub fn decode_dag_cbor(bytes: &[u8]) -> Result<CborValue, CborError> {
    if bytes.len() > MAX_DAG_CBOR_INPUT_LEN {
        return Err(CborError::InputTooLarge);
    }
    let mut budget = DecodeBudget::default();
    let (value, offset) = decode_value(bytes, 0, 0, &mut budget)?;
    let mut value = Zeroizing::new(value);
    if offset != bytes.len() {
        return Err(CborError::TrailingBytes);
    }
    Ok(core::mem::replace(&mut *value, CborValue::Null))
}

/// `depth` is the number of array/map containers currently open. It is
/// checked against [`MAX_NESTING_DEPTH`] before descending so a
/// pathologically nested input cannot overflow the stack.
fn decode_value(
    bytes: &[u8],
    offset: usize,
    depth: usize,
    budget: &mut DecodeBudget,
) -> Result<(CborValue, usize), CborError> {
    decode_value_with(bytes, offset, depth, budget, decode_value)
}

fn decode_value_with(
    bytes: &[u8],
    offset: usize,
    depth: usize,
    budget: &mut DecodeBudget,
    mut decode_child: impl FnMut(
        &[u8],
        usize,
        usize,
        &mut DecodeBudget,
    ) -> Result<(CborValue, usize), CborError>,
) -> Result<(CborValue, usize), CborError> {
    let (major, arg, offset) = read_value_head(bytes, offset, budget)?;
    match major {
        MT_UINT => Ok((
            CborValue::Int(i64::try_from(arg).map_err(|_| CborError::IntegerOutOfRange)?),
            offset,
        )),

        MT_NEGINT => {
            let magnitude = i128::from(arg);
            let value = (-1_i128)
                .checked_sub(magnitude)
                .ok_or(CborError::IntegerOutOfRange)?;
            Ok((
                CborValue::Int(i64::try_from(value).map_err(|_| CborError::IntegerOutOfRange)?),
                offset,
            ))
        }

        MT_BYTES => {
            let (b, off) = extract_bytes(bytes, offset, arg)?;
            Ok((CborValue::Bytes(b), off))
        }

        MT_STRING => {
            let (s, off) = extract_string(bytes, offset, arg)?;
            Ok((CborValue::String(s), off))
        }

        MT_ARRAY => {
            let child_depth = descend(depth)?;
            let item_count = usize::try_from(arg).map_err(|_| CborError::LengthTooLarge)?;
            // Reserve only what the remaining input could actually contain.
            // Each element occupies at least one byte, so a count larger
            // than the bytes left is a malformed length prefix and must be
            // rejected before allocating (prevents OOM from a crafted
            // header such as `9B 7F FF …`).
            bounded_capacity(item_count, bytes.len(), offset)?;
            if item_count > MAX_DAG_CBOR_CONTAINER_ENTRIES {
                return Err(CborError::ContainerEntriesExceeded);
            }
            budget.reserve_children(item_count)?;
            // The global pending-node budget bounds aggregate reservations;
            // exact capacity avoids reallocating secret-bearing elements.
            let mut items = Zeroizing::new(Vec::with_capacity(item_count));
            let mut off = offset;
            for _ in 0..item_count {
                let (v, next) = decode_child(bytes, off, child_depth, budget)?;
                items.push(v);
                off = next;
            }
            Ok((CborValue::Array(core::mem::take(&mut *items)), off))
        }

        MT_MAP => decode_map(bytes, offset, depth, arg, budget, decode_child),

        7 => Ok((decode_simple(arg)?, offset)),

        _ => Err(CborError::DisallowedMajorType { major }),
    }
}

fn decode_simple(argument: u64) -> Result<CborValue, CborError> {
    match argument {
        20 => Ok(CborValue::Bool(false)),
        21 => Ok(CborValue::Bool(true)),
        22 => Ok(CborValue::Null),
        _ => Err(CborError::DisallowedSimpleValue { value: argument }),
    }
}

fn read_value_head(
    bytes: &[u8],
    mut offset: usize,
    budget: &mut DecodeBudget,
) -> Result<(u8, u64, usize), CborError> {
    if offset >= bytes.len() {
        return Err(CborError::UnexpectedEnd);
    }
    budget.enter()?;

    let first = bytes[offset];
    offset = offset.checked_add(1).ok_or(CborError::OffsetOverflow)?;

    let major = first >> 5;
    let ai = first & 0x1f;

    // Floating-point payload bits are caller data, not simple-value codes.
    // Reject the type before reading them so typed errors cannot retain a
    // private number or arbitrary eight-byte payload in their metadata.
    if major == 7 && (25..=27).contains(&ai) {
        return Err(CborError::DisallowedSimpleValue {
            value: u64::from(ai),
        });
    }
    // DAG-CBOR admits only false, true, and null, which always use the
    // one-byte form. Every two-byte simple value is disallowed; RFC 8949
    // section 3.3 additionally makes values below 32 malformed, so this is
    // checked before the shared integer-minimality rule could misreport it.
    if major == 7 && ai == 24 {
        let value = bytes.get(offset).ok_or(CborError::TruncatedArgument)?;
        return Err(CborError::DisallowedSimpleValue {
            value: u64::from(*value),
        });
    }

    let (argument, end) = read_argument(bytes, offset, ai)?;
    Ok((major, argument, end))
}

fn decode_map(
    bytes: &[u8],
    offset: usize,
    depth: usize,
    argument: u64,
    budget: &mut DecodeBudget,
    decode_child: impl FnMut(
        &[u8],
        usize,
        usize,
        &mut DecodeBudget,
    ) -> Result<(CborValue, usize), CborError>,
) -> Result<(CborValue, usize), CborError> {
    let (entry_count, child_depth) = reserve_map(bytes.len(), offset, depth, argument, budget)?;
    let mut entries = Zeroizing::new(Vec::with_capacity(entry_count));
    let end = decode_map_entries(
        bytes,
        offset,
        child_depth,
        entry_count,
        budget,
        &mut entries,
        decode_child,
    )?;
    Ok((CborValue::Map(core::mem::take(&mut *entries)), end))
}

fn reserve_map(
    total_len: usize,
    offset: usize,
    depth: usize,
    argument: u64,
    budget: &mut DecodeBudget,
) -> Result<(usize, usize), CborError> {
    let child_depth = descend(depth)?;
    let entry_count = usize::try_from(argument).map_err(|_| CborError::LengthTooLarge)?;
    // Each entry needs at least two bytes, one each for the key and value.
    let entry_min = MIN_ELEMENT_ENCODED_LEN
        .checked_mul(2)
        .ok_or(CborError::OffsetOverflow)?;
    bounded_capacity_with_min(entry_count, total_len, offset, entry_min)?;
    if entry_count > MAX_DAG_CBOR_CONTAINER_ENTRIES {
        return Err(CborError::ContainerEntriesExceeded);
    }
    let child_count = entry_count
        .checked_mul(2)
        .ok_or(CborError::NodeLimitExceeded)?;
    budget.reserve_children(child_count)?;
    Ok((entry_count, child_depth))
}

fn decode_map_entries(
    bytes: &[u8],
    offset: usize,
    child_depth: usize,
    entry_count: usize,
    budget: &mut DecodeBudget,
    entries: &mut Zeroizing<Vec<(String, CborValue)>>,
    mut decode_child: impl FnMut(
        &[u8],
        usize,
        usize,
        &mut DecodeBudget,
    ) -> Result<(CborValue, usize), CborError>,
) -> Result<usize, CborError> {
    let mut order = MapKeyOrder::default();
    collect_map_entries(entry_count, offset, entries, |start| {
        decode_map_entry(
            bytes,
            start,
            child_depth,
            budget,
            &mut decode_child,
            |span| admit_map_key(&mut order, bytes, span),
        )
    })
}

fn collect_map_entries<T>(
    entry_count: usize,
    mut offset: usize,
    entries: &mut Vec<T>,
    mut decode_entry: impl FnMut(usize) -> Result<(T, usize), CborError>,
) -> Result<usize, CborError> {
    for _ in 0..entry_count {
        let (entry, next) = decode_entry(offset)?;
        entries.push(entry);
        offset = next;
    }
    Ok(offset)
}

fn decode_map_entry(
    bytes: &[u8],
    start: usize,
    depth: usize,
    budget: &mut DecodeBudget,
    mut decode_child: impl FnMut(
        &[u8],
        usize,
        usize,
        &mut DecodeBudget,
    ) -> Result<(CborValue, usize), CborError>,
    admit_key: impl FnOnce(core::ops::Range<usize>) -> Result<(), CborError>,
) -> Result<((String, CborValue), usize), CborError> {
    let (decoded_key, end) = decode_child(bytes, start, depth, budget)?;
    let mut decoded_key = Zeroizing::new(decoded_key);
    let mut key = Zeroizing::new(core::mem::take(require_text_key(&mut decoded_key)?));

    admit_key(start..end)?;
    let (value, next) = decode_child(bytes, end, depth, budget)?;
    Ok(((core::mem::take(&mut *key), value), next))
}

fn require_text_key(value: &mut CborValue) -> Result<&mut String, CborError> {
    match value {
        CborValue::String(text) => Ok(text),
        _ => Err(CborError::MapKeyMustBeString),
    }
}

fn admit_map_key<'a>(
    order: &mut MapKeyOrder<'a>,
    bytes: &'a [u8],
    span: core::ops::Range<usize>,
) -> Result<(), CborError> {
    // Canonical text keys have encoded lengths that increase with payload
    // length. Borrow the full wire span to compare without copying key bytes.
    let encoded = bytes.get(span).ok_or(CborError::OffsetOverflow)?;
    order.admit(encoded).map_err(|error| match error {
        MapKeyOrderError::Duplicate => CborError::DuplicateMapKey,
        MapKeyOrderError::Descending => CborError::MapKeysOutOfOrder,
    })
}

fn read_argument(bytes: &[u8], offset: usize, ai: u8) -> Result<(u64, usize), CborError> {
    match ai {
        n @ 0..=23 => Ok((u64::from(n), offset)),

        24 => {
            let end = checked_end(offset, 1)?;
            if end > bytes.len() {
                return Err(CborError::TruncatedArgument);
            }
            let value = u64::from(bytes[offset]);
            if value < 24 {
                return Err(CborError::NonCanonicalInteger);
            }
            Ok((value, end))
        }

        25 => {
            let end = checked_end(offset, 2)?;
            if end > bytes.len() {
                return Err(CborError::TruncatedArgument);
            }
            let val = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
            if val < 256 {
                return Err(CborError::NonCanonicalInteger);
            }
            Ok((u64::from(val), end))
        }

        26 => {
            let end = checked_end(offset, 4)?;
            if end > bytes.len() {
                return Err(CborError::TruncatedArgument);
            }
            let val = u32::from_be_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            if val < 65536 {
                return Err(CborError::NonCanonicalInteger);
            }
            Ok((u64::from(val), end))
        }

        27 => {
            let end = checked_end(offset, 8)?;
            if end > bytes.len() {
                return Err(CborError::TruncatedArgument);
            }
            let val = u64::from_be_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]);
            if val < 0x1_0000_0000 {
                return Err(CborError::NonCanonicalInteger);
            }
            Ok((val, end))
        }

        _ => Err(CborError::UnsupportedAdditionalInfo),
    }
}

fn extract_bytes(bytes: &[u8], offset: usize, len: u64) -> Result<(Vec<u8>, usize), CborError> {
    let raw = declared_contents(bytes, offset, len)?;
    Ok((raw.to_vec(), checked_end(offset, raw.len())?))
}

fn extract_string(bytes: &[u8], offset: usize, len: u64) -> Result<(String, usize), CborError> {
    let raw = declared_contents(bytes, offset, len)?;
    let s = str::from_utf8(raw).map_err(|_| CborError::InvalidUtf8)?;
    Ok((s.to_string(), checked_end(offset, raw.len())?))
}

/// Borrows the `len` content bytes that follow a string head at `offset`.
///
/// A declared length beyond the remaining input is truncation on every
/// target. Comparing in `u64` before converting keeps 32-bit and WebAssembly
/// builds from reporting the same input as a resource-limit failure.
fn declared_contents(bytes: &[u8], offset: usize, len: u64) -> Result<&[u8], CborError> {
    let remaining = bytes.get(offset..).ok_or(CborError::TruncatedBytes)?;
    let available = u64::try_from(remaining.len()).map_err(|_| CborError::LengthTooLarge)?;
    if len > available {
        return Err(CborError::TruncatedBytes);
    }
    let len = usize::try_from(len).map_err(|_| CborError::LengthTooLarge)?;
    remaining.get(..len).ok_or(CborError::TruncatedBytes)
}

fn checked_end(offset: usize, len: usize) -> Result<usize, CborError> {
    offset.checked_add(len).ok_or(CborError::OffsetOverflow)
}

/// Enters one nesting level, rejecting input that would exceed
/// [`MAX_NESTING_DEPTH`].
fn descend(depth: usize) -> Result<usize, CborError> {
    let next = depth.checked_add(1).ok_or(CborError::OffsetOverflow)?;
    if next > MAX_NESTING_DEPTH {
        return Err(CborError::DepthExceeded);
    }
    Ok(next)
}

/// Capacity to reserve for a container of `count` elements, each at least
/// [`MIN_ELEMENT_ENCODED_LEN`] bytes.
fn bounded_capacity(count: usize, total_len: usize, offset: usize) -> Result<usize, CborError> {
    bounded_capacity_with_min(count, total_len, offset, MIN_ELEMENT_ENCODED_LEN)
}

/// Rejects a declared element `count` that could not fit in the bytes
/// remaining after `offset`, then returns that count as the reservation
/// size. Because every element needs at least `min_element_len` bytes, a
/// count exceeding `remaining / min_element_len` is provably malformed, so
/// this both prevents OOM aborts and reserves an exact, honest capacity.
fn bounded_capacity_with_min(
    count: usize,
    total_len: usize,
    offset: usize,
    min_element_len: usize,
) -> Result<usize, CborError> {
    let remaining = total_len.saturating_sub(offset);
    let max_possible = remaining / min_element_len.max(1);
    if count > max_possible {
        return Err(CborError::ContainerLengthExceedsInput);
    }
    Ok(count)
}

#[cfg(kani)]
#[path = "decode_dag_cbor_proof_tests.rs"]
mod proofs;
