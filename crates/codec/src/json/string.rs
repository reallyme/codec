// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use codec_proto::guarded::SecretString;
use zeroize::Zeroizing;

use super::{error::overflow, limits::Budget, ErrorReason as R, JsonError};

const ESCAPE_HEX_DIGITS: usize = 4;
const HIGH_SURROGATE_FIRST: u32 = 0xd800;
const HIGH_SURROGATE_LAST: u32 = 0xdbff;
const LOW_SURROGATE_FIRST: u32 = 0xdc00;
const LOW_SURROGATE_LAST: u32 = 0xdfff;
const SURROGATE_BLOCK: u32 = 0x400;
const SUPPLEMENTARY_FIRST: u32 = 0x1_0000;

fn unicode_error() -> JsonError {
    JsonError::new(R::CODEC_ERROR_REASON_CANONICAL_JSON_INVALID_UNICODE)
}

fn advance(offset: &mut usize, count: usize) -> Result<(), JsonError> {
    *offset = offset.checked_add(count).ok_or_else(overflow)?;
    Ok(())
}

fn hex(bytes: &[u8], offset: &mut usize) -> Result<Zeroizing<u32>, JsonError> {
    let end = offset.checked_add(ESCAPE_HEX_DIGITS).ok_or_else(overflow)?;
    let digits = bytes.get(*offset..end).ok_or_else(unicode_error)?;
    let mut value = Zeroizing::new(0u32);
    for byte in digits {
        let digit = char::from(*byte).to_digit(16).ok_or_else(unicode_error)?;
        *value = value
            .checked_mul(16)
            .and_then(|v| v.checked_add(digit))
            .ok_or_else(overflow)?;
    }
    *offset = end;
    Ok(value)
}

fn scalar(bytes: &[u8], offset: &mut usize) -> Result<Zeroizing<char>, JsonError> {
    let mut value = hex(bytes, offset)?;
    if (HIGH_SURROGATE_FIRST..=HIGH_SURROGATE_LAST).contains(&value) {
        let end = offset.checked_add(2).ok_or_else(overflow)?;
        if bytes.get(*offset..end) != Some(b"\\u") {
            return Err(unicode_error());
        }
        *offset = end;
        let low = hex(bytes, offset)?;
        if !(LOW_SURROGATE_FIRST..=LOW_SURROGATE_LAST).contains(&low) {
            return Err(unicode_error());
        }
        *value = value
            .checked_sub(HIGH_SURROGATE_FIRST)
            .and_then(|v| v.checked_mul(SURROGATE_BLOCK))
            .and_then(|v| {
                low.checked_sub(LOW_SURROGATE_FIRST)
                    .and_then(|low| v.checked_add(low))
            })
            .and_then(|v| v.checked_add(SUPPLEMENTARY_FIRST))
            .ok_or_else(overflow)?;
    } else if (LOW_SURROGATE_FIRST..=LOW_SURROGATE_LAST).contains(&value) {
        return Err(unicode_error());
    }
    char::from_u32(*value)
        .map(Zeroizing::new)
        .ok_or_else(unicode_error)
}

/// Decode only an already framed string token. Reserve its bounded capacity
/// before writing: growth must never retire an unwiped copy of a prefix.
pub(super) fn decode(raw: &str, budget: &mut Budget) -> Result<SecretString, JsonError> {
    let content = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or_else(unicode_error)?;
    let mut output = Zeroizing::new(Vec::new());
    output
        .try_reserve_exact(budget.string_capacity(content.len())?)
        .map_err(|_| JsonError::new(R::CODEC_ERROR_REASON_CANONICAL_JSON_ALLOCATION_FAILURE))?;
    let bytes = content.as_bytes();
    let mut offset = 0usize;
    while let Some(byte) = bytes.get(offset) {
        let mut encoded = Zeroizing::new([0u8; 4]);
        let length;
        advance(&mut offset, 1)?;
        if *byte == b'\\' {
            let escape = *bytes.get(offset).ok_or_else(unicode_error)?;
            advance(&mut offset, 1)?;
            if escape == b'u' {
                let value = scalar(bytes, &mut offset)?;
                length = value.encode_utf8(&mut *encoded).len();
            } else {
                encoded[0] = match escape {
                    b'"' => b'"',
                    b'\\' => b'\\',
                    b'/' => b'/',
                    b'b' => 8,
                    b'f' => 12,
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    _ => return Err(unicode_error()),
                };
                length = 1;
            }
        } else {
            if *byte < 0x20 || *byte == b'"' {
                return Err(unicode_error());
            }
            encoded[0] = *byte;
            length = 1;
        }
        budget.text(output.len(), length)?;
        output.extend_from_slice(encoded.get(..length).ok_or_else(overflow)?);
    }
    match String::from_utf8(core::mem::take(&mut *output)) {
        Ok(text) => Ok(text.into()),
        Err(error) => {
            let _rejected = Zeroizing::new(error.into_bytes());
            Err(unicode_error())
        }
    }
}

pub(super) fn number_text(raw: &str, budget: &mut Budget) -> Result<SecretString, JsonError> {
    budget.text(0, raw.len())?;
    let mut output = Zeroizing::new(String::new());
    output
        .try_reserve_exact(raw.len())
        .map_err(|_| JsonError::new(R::CODEC_ERROR_REASON_CANONICAL_JSON_ALLOCATION_FAILURE))?;
    output.push_str(raw);
    Ok(core::mem::take(&mut *output).into())
}
