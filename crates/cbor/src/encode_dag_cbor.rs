// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{CborError, CborValue, MAX_DAG_CBOR_INPUT_LEN, MAX_NESTING_DEPTH};
use zeroize::Zeroizing;

const MT_UINT: u8 = 0;
const MT_NEGINT: u8 = 1;
const MT_BYTES: u8 = 2;
const MT_STRING: u8 = 3;
const MT_ARRAY: u8 = 4;
const MT_MAP: u8 = 5;

/// Encode a value using canonical DAG-CBOR.
///
/// This encoding:
/// - uses definite-length, shortest-form (canonical) integer headers only
/// - orders map keys by length-first deterministic rules: shorter encoded
///   key first, then bytewise lexical order among equal lengths
/// - contains no floats, tags, or indefinite-length items
/// - is deterministic and cryptographically stable, so equal values always
///   encode to identical bytes (a prerequisite for stable content IDs)
pub fn encode_dag_cbor(value: &CborValue) -> Result<Vec<u8>, CborError> {
    // Run the same encoder against a counting sink first. This preserves
    // error precedence, then reserves once so no written document buffer is
    // ever discarded by Vec growth.
    let mut length = 0_usize;
    encode_value(value, &mut length, 0)?;
    let mut out = Zeroizing::new(Vec::with_capacity(length));
    encode_value(value, &mut *out, 0)?;
    Ok(core::mem::take(&mut *out))
}

fn encode_value(v: &CborValue, out: &mut impl EncodingSink, depth: usize) -> Result<(), CborError> {
    match v {
        CborValue::Null => push_byte(out, 0xf6)?,
        CborValue::Bool(false) => push_byte(out, 0xf4)?,
        CborValue::Bool(true) => push_byte(out, 0xf5)?,

        CborValue::Int(n) => {
            if *n >= 0 {
                write_header(MT_UINT, n.unsigned_abs(), out)?;
            } else {
                write_header(MT_NEGINT, n.unsigned_abs() - 1, out)?;
            }
        }

        CborValue::Bytes(b) => {
            write_header(MT_BYTES, len_as_u64(b.len())?, out)?;
            extend_bytes(out, b)?;
        }

        CborValue::String(s) => {
            let bytes = s.as_bytes();
            write_header(MT_STRING, len_as_u64(bytes.len())?, out)?;
            extend_bytes(out, bytes)?;
        }

        CborValue::Array(arr) => {
            let child_depth = descend(depth)?;
            ensure_minimum_encoded_len(arr.len(), 1)?;
            write_header(MT_ARRAY, len_as_u64(arr.len())?, out)?;
            for v in arr {
                encode_value(v, out, child_depth)?;
            }
        }

        CborValue::Map(entries) => {
            let child_depth = descend(depth)?;
            ensure_minimum_encoded_len(entries.len(), 2)?;
            // Length-first deterministic ordering sorts text keys by the
            // length of their encoded bytes first, then by bytewise lexical
            // order. did:me vectors rely on this exact order for stable CIDs.
            let mut sorted: Vec<(&String, &CborValue)> =
                entries.iter().map(|(key, value)| (key, value)).collect();
            sorted.sort_by(|(ka, _), (kb, _)| {
                ka.len()
                    .cmp(&kb.len())
                    .then_with(|| ka.as_bytes().cmp(kb.as_bytes()))
            });

            if sorted
                .windows(2)
                .any(|pair| pair[0].0.as_bytes() == pair[1].0.as_bytes())
            {
                return Err(CborError::DuplicateMapKey);
            }

            write_header(MT_MAP, len_as_u64(sorted.len())?, out)?;

            for (k, v) in sorted {
                let kb = k.as_bytes();
                write_header(MT_STRING, len_as_u64(kb.len())?, out)?;
                extend_bytes(out, kb)?;
                encode_value(v, out, child_depth)?;
            }
        }
    }
    Ok(())
}

/// Widens a container length to the `u64` argument width CBOR headers use.
///
/// This is a widening conversion — `usize` is at most 64 bits on every
/// supported target — so it never loses information on supported platforms,
/// while still returning a typed error if that assumption is violated.
fn len_as_u64(len: usize) -> Result<u64, CborError> {
    u64::try_from(len).map_err(|_| CborError::LengthTooLarge)
}

/// Writes a CBOR head byte plus the minimal big-endian argument encoding
/// for `value`, following canonical (shortest-form) integer rules.
///
/// Each branch slices the exact low-order bytes of `value.to_be_bytes()`
/// that its range guarantees are significant, so no narrowing cast or
/// truncation is involved.
fn write_header(mt: u8, value: u64, out: &mut impl EncodingSink) -> Result<(), CborError> {
    let be = value.to_be_bytes();
    let head = mt << 5;
    if value < 24 {
        // The whole argument fits in the low 5 bits of the head byte.
        push_byte(out, head | be[7])?;
    } else if value < 0x100 {
        push_byte(out, head | 24)?;
        extend_bytes(out, &be[7..8])?;
    } else if value < 0x1_0000 {
        push_byte(out, head | 25)?;
        extend_bytes(out, &be[6..8])?;
    } else if value < 0x1_0000_0000 {
        push_byte(out, head | 26)?;
        extend_bytes(out, &be[4..8])?;
    } else {
        push_byte(out, head | 27)?;
        extend_bytes(out, &be)?;
    }
    Ok(())
}

/// A counting sink and a byte sink share every validation and ordering step.
/// Only the byte sink copies caller data into an owned allocation.
trait EncodingSink {
    fn append(&mut self, bytes: &[u8]) -> Result<(), CborError>;
}

impl EncodingSink for usize {
    fn append(&mut self, bytes: &[u8]) -> Result<(), CborError> {
        *self = checked_output_length(*self, bytes.len())?;
        Ok(())
    }
}

impl EncodingSink for Vec<u8> {
    fn append(&mut self, bytes: &[u8]) -> Result<(), CborError> {
        checked_output_length(self.len(), bytes.len())?;
        self.extend_from_slice(bytes);
        Ok(())
    }
}

fn checked_output_length(length: usize, additional: usize) -> Result<usize, CborError> {
    let next = length
        .checked_add(additional)
        .ok_or(CborError::OffsetOverflow)?;
    if next > MAX_DAG_CBOR_INPUT_LEN {
        return Err(CborError::OutputTooLarge);
    }
    Ok(next)
}

fn push_byte(out: &mut impl EncodingSink, byte: u8) -> Result<(), CborError> {
    out.append(&[byte])
}

fn extend_bytes(out: &mut impl EncodingSink, bytes: &[u8]) -> Result<(), CborError> {
    out.append(bytes)
}

fn descend(depth: usize) -> Result<usize, CborError> {
    let next = depth.checked_add(1).ok_or(CborError::OffsetOverflow)?;
    if next > MAX_NESTING_DEPTH {
        return Err(CborError::DepthExceeded);
    }
    Ok(next)
}

fn ensure_minimum_encoded_len(count: usize, min_element_len: usize) -> Result<(), CborError> {
    let minimum = count
        .checked_mul(min_element_len)
        .ok_or(CborError::OffsetOverflow)?;
    if minimum > MAX_DAG_CBOR_INPUT_LEN {
        return Err(CborError::OutputTooLarge);
    }
    Ok(())
}
