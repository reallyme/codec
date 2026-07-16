// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use crate::{CborError, CborValue, MAX_DAG_CBOR_INPUT_LEN, MAX_NESTING_DEPTH};

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
/// - orders map keys by RFC 8949 core deterministic rules: shorter encoded
///   key first, then bytewise lexical order among equal lengths
/// - contains no floats, tags, or indefinite-length items
/// - is deterministic and cryptographically stable, so equal values always
///   encode to identical bytes (a prerequisite for stable content IDs)
pub fn encode_dag_cbor(value: &CborValue) -> Result<Vec<u8>, CborError> {
    let mut out = Vec::new();
    encode_value(value, &mut out, 0)?;
    Ok(out)
}

fn encode_value(v: &CborValue, out: &mut Vec<u8>, depth: usize) -> Result<(), CborError> {
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
            // RFC 8949 core deterministic ordering sorts text keys by the
            // length of their encoded bytes first, then by bytewise lexical
            // order. did:me vectors rely on this exact order for stable CIDs.
            let mut sorted: Vec<(&String, &CborValue)> =
                entries.iter().map(|(key, value)| (key, value)).collect();
            sorted.sort_by(|(ka, _), (kb, _)| {
                ka.len()
                    .cmp(&kb.len())
                    .then_with(|| ka.as_bytes().cmp(kb.as_bytes()))
            });

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
fn write_header(mt: u8, value: u64, out: &mut Vec<u8>) -> Result<(), CborError> {
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

fn push_byte(out: &mut Vec<u8>, byte: u8) -> Result<(), CborError> {
    let next_len = out.len().checked_add(1).ok_or(CborError::OffsetOverflow)?;
    if next_len > MAX_DAG_CBOR_INPUT_LEN {
        return Err(CborError::OutputTooLarge);
    }
    out.push(byte);
    Ok(())
}

fn extend_bytes(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), CborError> {
    let next_len = out
        .len()
        .checked_add(bytes.len())
        .ok_or(CborError::OffsetOverflow)?;
    if next_len > MAX_DAG_CBOR_INPUT_LEN {
        return Err(CborError::OutputTooLarge);
    }
    out.extend_from_slice(bytes);
    Ok(())
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
