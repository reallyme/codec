// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{CborError, CborValue, MAX_DAG_CBOR_INPUT_LEN, MAX_NESTING_DEPTH};
use std::cmp::Ordering;
use std::str;
use zeroize::Zeroizing;

const MT_UINT: u8 = 0;
const MT_NEGINT: u8 = 1;
const MT_BYTES: u8 = 2;
const MT_STRING: u8 = 3;
const MT_ARRAY: u8 = 4;
const MT_MAP: u8 = 5;
const CONTAINER_INITIAL_RESERVE: usize = 8;

/// Smallest possible encoding of one array element or map key/value: a
/// single header byte (e.g. a small integer, or an empty string/array).
/// Used to reject a declared container length that the remaining input
/// could never satisfy, before any capacity is reserved.
const MIN_ELEMENT_ENCODED_LEN: usize = 1;

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
    let (value, offset) = decode_value(bytes, 0, 0)?;
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
    mut offset: usize,
    depth: usize,
) -> Result<(CborValue, usize), CborError> {
    if offset >= bytes.len() {
        return Err(CborError::UnexpectedEnd);
    }

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

    let (arg, new_offset) = read_argument(bytes, offset, ai)?;
    offset = new_offset;

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
            let capacity = initial_container_capacity(item_count);
            let mut items = Zeroizing::new(Vec::with_capacity(capacity));
            let mut off = offset;
            for _ in 0..item_count {
                let (v, next) = decode_value(bytes, off, child_depth)?;
                items.push(v);
                off = next;
            }
            Ok((CborValue::Array(core::mem::take(&mut *items)), off))
        }

        MT_MAP => {
            let child_depth = descend(depth)?;
            let entry_count = usize::try_from(arg).map_err(|_| CborError::LengthTooLarge)?;
            // Each entry is a key plus a value, so it needs at least two
            // bytes; bound the reservation against that before allocating.
            let entry_min = MIN_ELEMENT_ENCODED_LEN
                .checked_mul(2)
                .ok_or(CborError::OffsetOverflow)?;
            bounded_capacity_with_min(entry_count, bytes.len(), offset, entry_min)?;
            let capacity = initial_container_capacity(entry_count);
            let mut entries = Zeroizing::new(Vec::with_capacity(capacity));
            let mut off = offset;
            let mut last_key_bytes: Option<Zeroizing<Vec<u8>>> = None;

            for _ in 0..entry_count {
                let (key_val, key_off) = decode_value(bytes, off, child_depth)?;
                off = key_off;

                let mut key_val = Zeroizing::new(key_val);
                let mut key = Zeroizing::new(match &mut *key_val {
                    CborValue::String(s) => core::mem::take(s),
                    _ => return Err(CborError::MapKeyMustBeString),
                });

                let key_bytes = Zeroizing::new(key.as_bytes().to_vec());
                if let Some(prev) = &last_key_bytes {
                    match compare_bytes(prev, &key_bytes) {
                        Ordering::Less => {}
                        Ordering::Equal => return Err(CborError::DuplicateMapKey),
                        Ordering::Greater => return Err(CborError::MapKeysOutOfOrder),
                    }
                }
                last_key_bytes = Some(key_bytes);

                let (val, val_off) = decode_value(bytes, off, child_depth)?;
                off = val_off;

                entries.push((core::mem::take(&mut *key), val));
            }

            Ok((CborValue::Map(core::mem::take(&mut *entries)), off))
        }

        7 => match arg {
            20 => Ok((CborValue::Bool(false), offset)),
            21 => Ok((CborValue::Bool(true), offset)),
            22 => Ok((CborValue::Null, offset)),
            _ => Err(CborError::DisallowedSimpleValue { value: arg }),
        },

        _ => Err(CborError::DisallowedMajorType { major }),
    }
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
    let len = usize::try_from(len).map_err(|_| CborError::LengthTooLarge)?;
    let end = checked_end(offset, len)?;
    if end > bytes.len() {
        return Err(CborError::TruncatedBytes);
    }
    Ok((bytes[offset..end].to_vec(), end))
}

fn extract_string(bytes: &[u8], offset: usize, len: u64) -> Result<(String, usize), CborError> {
    let len = usize::try_from(len).map_err(|_| CborError::LengthTooLarge)?;
    let end = checked_end(offset, len)?;
    let raw = bytes.get(offset..end).ok_or(CborError::TruncatedBytes)?;
    let s = str::from_utf8(raw).map_err(|_| CborError::InvalidUtf8)?;
    Ok((s.to_string(), end))
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

fn initial_container_capacity(count: usize) -> usize {
    // The declared element count is only a promise by untrusted input. Reserving
    // it eagerly lets each nested ancestor retain a large allocation before the
    // decoder discovers the promised siblings are absent. Start small and let
    // Vec grow only for elements that have actually been decoded.
    count.min(CONTAINER_INITIAL_RESERVE)
}

fn compare_bytes(a: &[u8], b: &[u8]) -> Ordering {
    match a.len().cmp(&b.len()) {
        Ordering::Equal => a.cmp(b),
        ordering => ordering,
    }
}
