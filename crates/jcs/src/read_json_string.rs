// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::JcsError;
use zeroize::Zeroizing;

const HEX_DIGITS_PER_UNIT: usize = 4;
const HEX_RADIX: u16 = 16;
const HIGH_SURROGATE_START: u16 = 0xd800;
const HIGH_SURROGATE_END: u16 = 0xdbff;
const LOW_SURROGATE_START: u16 = 0xdc00;
const LOW_SURROGATE_END: u16 = 0xdfff;
const SURROGATE_WIDTH: u32 = 0x400;
const SUPPLEMENTARY_START: u32 = 0x10000;

/// Decode directly into the wiping owner. No parser-owned unescape scratch is
/// created, and one reservation prevents reallocating a decoded private prefix.
pub(crate) fn read_json_string(
    input: &str,
    offset: &mut usize,
) -> Result<Zeroizing<String>, JcsError> {
    let bytes = input.as_bytes();
    let mut cursor = *offset;
    if take(bytes, &mut cursor)? != b'"' {
        return Err(JcsError::InvalidJson);
    }
    let start = cursor;
    let end = loop {
        let position = cursor;
        match take(bytes, &mut cursor)? {
            b'"' => break position,
            b'\\' => {
                take(bytes, &mut cursor)?;
            }
            0..=0x1f => return Err(JcsError::InvalidJson),
            _ => {}
        }
    };
    let source = input.get(start..end).ok_or(JcsError::InvalidJson)?;
    let mut output = Zeroizing::new(String::new());
    output
        .try_reserve_exact(source.len())
        .map_err(|_| JcsError::SerializationError)?;
    cursor = start;
    let mut chunk = start;
    while cursor < end {
        let position = cursor;
        if take(bytes, &mut cursor)? == b'\\' {
            output.push_str(input.get(chunk..position).ok_or(JcsError::InvalidJson)?);
            let escaped = match take(bytes, &mut cursor)? {
                b'"' => '"',
                b'\\' => '\\',
                b'/' => '/',
                b'b' => '\u{0008}',
                b'f' => '\u{000c}',
                b'n' => '\n',
                b'r' => '\r',
                b't' => '\t',
                b'u' => unicode_escape(bytes, &mut cursor)?,
                _ => return Err(JcsError::InvalidJson),
            };
            if cursor > end {
                return Err(JcsError::InvalidJson);
            }
            output.push(escaped);
            chunk = cursor;
        }
    }
    output.push_str(input.get(chunk..end).ok_or(JcsError::InvalidJson)?);
    *offset = end.checked_add(1).ok_or(JcsError::InvalidJson)?;
    Ok(output)
}

fn take(input: &[u8], offset: &mut usize) -> Result<u8, JcsError> {
    let byte = *input.get(*offset).ok_or(JcsError::InvalidJson)?;
    *offset = offset.checked_add(1).ok_or(JcsError::InvalidJson)?;
    Ok(byte)
}

fn hex_unit(input: &[u8], offset: &mut usize) -> Result<u16, JcsError> {
    let mut value = 0_u16;
    for _ in 0..HEX_DIGITS_PER_UNIT {
        let byte = take(input, offset)?;
        let digit = match byte {
            b'0'..=b'9' => u16::from(byte - b'0'),
            b'a'..=b'f' => u16::from(byte - b'a') + 10,
            b'A'..=b'F' => u16::from(byte - b'A') + 10,
            _ => return Err(JcsError::InvalidJson),
        };
        value = value
            .checked_mul(HEX_RADIX)
            .and_then(|v| v.checked_add(digit))
            .ok_or(JcsError::InvalidJson)?;
    }
    Ok(value)
}

fn unicode_escape(input: &[u8], offset: &mut usize) -> Result<char, JcsError> {
    let high = hex_unit(input, offset)?;
    let scalar = if (HIGH_SURROGATE_START..=HIGH_SURROGATE_END).contains(&high) {
        if take(input, offset)? != b'\\' || take(input, offset)? != b'u' {
            return Err(JcsError::InvalidJson);
        }
        let low = hex_unit(input, offset)?;
        if !(LOW_SURROGATE_START..=LOW_SURROGATE_END).contains(&low) {
            return Err(JcsError::InvalidJson);
        }
        u32::from(
            high.checked_sub(HIGH_SURROGATE_START)
                .ok_or(JcsError::InvalidJson)?,
        )
        .checked_mul(SURROGATE_WIDTH)
        .and_then(|value| value.checked_add(u32::from(low - LOW_SURROGATE_START)))
        .and_then(|value| value.checked_add(SUPPLEMENTARY_START))
        .ok_or(JcsError::InvalidJson)?
    } else {
        u32::from(high)
    };
    char::from_u32(scalar).ok_or(JcsError::InvalidJson)
}
