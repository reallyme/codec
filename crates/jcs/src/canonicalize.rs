// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use std::cmp::Ordering;

use serde_json::Value;
use zeroize::Zeroizing;

use crate::error::JcsError;
use crate::parse_json::{parse_json_text, SensitiveJsonValue};

const MAX_INTEROPERABLE_INTEGER: u64 = 9_007_199_254_740_991;
const MIN_INTEROPERABLE_INTEGER: i64 = -9_007_199_254_740_991;
const MAX_INTEROPERABLE_INTEGER_F64: f64 = 9_007_199_254_740_991.0;
const MIN_INTEROPERABLE_INTEGER_F64: f64 = -9_007_199_254_740_991.0;

/// Canonicalize a trusted, already-materialized JSON value according to RFC
/// 8785 (JSON Canonicalization Scheme).
///
/// Integer and integer-valued binary64 numbers are rejected outside the
/// interoperable `[-(2^53)+1, (2^53)-1]` range. This function cannot determine
/// whether a source document contained duplicate object member names because
/// [`Value`] has already discarded that provenance. It is therefore intended
/// only for values constructed programmatically or produced by a parser that
/// already enforced the same duplicate-member policy. Call
/// [`canonicalize_json_text`] at every untrusted text boundary.
pub fn canonicalize_trusted_json_value(value: &Value) -> Result<String, JcsError> {
    let output_length = canonicalized_len(value, 0)?;
    let mut output = Zeroizing::new(String::with_capacity(output_length));
    write_canonical(value, 0, &mut output)?;
    if output.len() != output_length {
        return Err(JcsError::SerializationError);
    }
    Ok(core::mem::take(&mut *output))
}

/// Parse and canonicalize one untrusted JSON text according to RFC 8785.
///
/// Unlike deserializing directly into [`Value`], this entry point detects and
/// rejects duplicate object member names instead of silently retaining one
/// value. It also rejects trailing data. Integer tokens retained exactly as
/// `i64`, `u64`, or integer-valued binary64 are rejected outside the
/// interoperable range, while non-integer binary64 numbers follow RFC 8785's
/// required ECMAScript rounding behavior.
pub fn canonicalize_json_text(input: &str) -> Result<String, JcsError> {
    let value = parse_json_text(input)?;
    canonicalize_sensitive_json_value(&value)
}

/// `depth` counts the array/object containers currently open, bounded by
/// [`MAX_NESTING_DEPTH`](crate::MAX_NESTING_DEPTH) as defense in depth
/// against a `Value` that was built without a parser depth limit.
fn write_canonical(value: &Value, depth: usize, output: &mut String) -> Result<(), JcsError> {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(false) => output.push_str("false"),
        Value::Bool(true) => output.push_str("true"),
        Value::Number(value) => write_canonical_number(value, output)?,
        Value::String(value) => write_escaped_string(value, output),
        Value::Array(values) => write_canonical_array(values, depth, output)?,
        Value::Object(values) => {
            let child_depth = descend(depth)?;
            let mut keys: Vec<&String> = values.keys().collect();
            // RFC 8785 §3.2.3: sort by UTF-16 code unit, NOT by Unicode
            // scalar value. The two orders agree across the BMP but diverge
            // for supplementary-plane names, whose UTF-16 surrogate code
            // units (0xD800–0xDFFF) sort below BMP code points such as
            // 0xE000–0xFFFF. A plain `str` sort (UTF-8 / code-point order)
            // would place them differently and yield non-canonical output.
            keys.sort_by(|left, right| utf16_cmp(left, right));

            output.push('{');
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_escaped_string(key, output);
                output.push(':');
                write_canonical(&values[*key], child_depth, output)?;
            }
            output.push('}');
        }
    }
    Ok(())
}

/// Enters one nesting level, rejecting input deeper than
/// [`MAX_NESTING_DEPTH`](crate::MAX_NESTING_DEPTH).
fn descend(depth: usize) -> Result<usize, JcsError> {
    let next = depth.checked_add(1).ok_or(JcsError::DepthExceeded)?;
    if next > crate::MAX_NESTING_DEPTH {
        return Err(JcsError::DepthExceeded);
    }
    Ok(next)
}

/// Compares two strings by their UTF-16 code units, as RFC 8785 requires
/// for object member ordering.
fn utf16_cmp(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

fn write_canonical_number(value: &serde_json::Number, output: &mut String) -> Result<(), JcsError> {
    if let Some(unsigned) = value.as_u64() {
        if unsigned > MAX_INTEROPERABLE_INTEGER {
            return Err(JcsError::IntegerOutsideInteroperableRange);
        }
        let mut buffer = itoa::Buffer::new();
        output.push_str(buffer.format(unsigned));
        return Ok(());
    }
    if let Some(signed) = value.as_i64() {
        if signed < MIN_INTEROPERABLE_INTEGER {
            return Err(JcsError::IntegerOutsideInteroperableRange);
        }
        let mut buffer = itoa::Buffer::new();
        output.push_str(buffer.format(signed));
        return Ok(());
    }

    let float = value.as_f64().ok_or(JcsError::SerializationError)?;
    if !float.is_finite() {
        return Err(JcsError::NonFiniteNumber);
    }
    validate_interoperable_float_integer(float)?;

    // RFC 8785 §3.2.2.3 mandates the ECMAScript `Number.prototype.toString`
    // algorithm (exponent thresholds, exponent signs, shortest round-trip
    // digits). `serde_json`/`ryu` do not fully match those ES6 formatting
    // rules, so we use `ryu-js` after applying ReallyMe's stricter
    // interoperable-integer policy.
    let mut buffer = ryu_js::Buffer::new();
    output.push_str(buffer.format_finite(float));
    Ok(())
}

fn write_canonical_array(
    values: &[Value],
    depth: usize,
    output: &mut String,
) -> Result<(), JcsError> {
    let child_depth = descend(depth)?;
    output.push('[');
    for (index, item) in values.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        write_canonical(item, child_depth, output)?;
    }
    output.push(']');
    Ok(())
}

fn canonicalized_len(value: &Value, depth: usize) -> Result<usize, JcsError> {
    match value {
        Value::Null => Ok("null".len()),
        Value::Bool(false) => Ok("false".len()),
        Value::Bool(true) => Ok("true".len()),
        Value::Number(value) => canonical_number_len(value),
        Value::String(value) => escaped_string_len(value),
        Value::Array(values) => canonical_array_len(values, depth),
        Value::Object(values) => canonical_object_len(values, depth),
    }
}

fn canonicalize_sensitive_json_value(value: &SensitiveJsonValue) -> Result<String, JcsError> {
    let output_length = canonicalized_sensitive_len(value, 0)?;
    let mut output = Zeroizing::new(String::with_capacity(output_length));
    write_sensitive_canonical(value, 0, &mut output)?;
    if output.len() != output_length {
        return Err(JcsError::SerializationError);
    }
    Ok(core::mem::take(&mut *output))
}

fn write_sensitive_canonical(
    value: &SensitiveJsonValue,
    depth: usize,
    output: &mut String,
) -> Result<(), JcsError> {
    match value {
        SensitiveJsonValue::Null => output.push_str("null"),
        SensitiveJsonValue::Bool(false) => output.push_str("false"),
        SensitiveJsonValue::Bool(true) => output.push_str("true"),
        SensitiveJsonValue::Number(value) => write_canonical_number(value, output)?,
        SensitiveJsonValue::String(value) => write_escaped_string(value, output),
        SensitiveJsonValue::Array(values) => {
            let child_depth = descend(depth)?;
            output.push('[');
            for (index, item) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_sensitive_canonical(item, child_depth, output)?;
            }
            output.push(']');
        }
        SensitiveJsonValue::Object(values) => {
            let child_depth = descend(depth)?;
            let mut keys: Vec<&String> = values.keys().collect();
            keys.sort_by(|left, right| utf16_cmp(left, right));

            output.push('{');
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_escaped_string(key, output);
                output.push(':');
                let value = values.get(*key).ok_or(JcsError::SerializationError)?;
                write_sensitive_canonical(value, child_depth, output)?;
            }
            output.push('}');
        }
    }
    Ok(())
}

fn canonicalized_sensitive_len(
    value: &SensitiveJsonValue,
    depth: usize,
) -> Result<usize, JcsError> {
    match value {
        SensitiveJsonValue::Null => Ok("null".len()),
        SensitiveJsonValue::Bool(false) => Ok("false".len()),
        SensitiveJsonValue::Bool(true) => Ok("true".len()),
        SensitiveJsonValue::Number(value) => canonical_number_len(value),
        SensitiveJsonValue::String(value) => escaped_string_len(value),
        SensitiveJsonValue::Array(values) => canonical_sensitive_array_len(values, depth),
        SensitiveJsonValue::Object(values) => canonical_sensitive_object_len(values, depth),
    }
}

fn canonical_sensitive_array_len(
    values: &[SensitiveJsonValue],
    depth: usize,
) -> Result<usize, JcsError> {
    let child_depth = descend(depth)?;
    let mut length = "["
        .len()
        .checked_add("]".len())
        .ok_or(JcsError::SerializationError)?;
    for (index, item) in values.iter().enumerate() {
        if index > 0 {
            length = length
                .checked_add(",".len())
                .ok_or(JcsError::SerializationError)?;
        }
        length = length
            .checked_add(canonicalized_sensitive_len(item, child_depth)?)
            .ok_or(JcsError::SerializationError)?;
    }
    Ok(length)
}

fn canonical_sensitive_object_len(
    values: &std::collections::BTreeMap<String, SensitiveJsonValue>,
    depth: usize,
) -> Result<usize, JcsError> {
    let child_depth = descend(depth)?;
    let mut keys: Vec<&String> = values.keys().collect();
    keys.sort_by(|left, right| utf16_cmp(left, right));

    let mut length = "{"
        .len()
        .checked_add("}".len())
        .ok_or(JcsError::SerializationError)?;
    for (index, key) in keys.iter().enumerate() {
        if index > 0 {
            length = length
                .checked_add(",".len())
                .ok_or(JcsError::SerializationError)?;
        }
        let value = values.get(*key).ok_or(JcsError::SerializationError)?;
        let value_length = canonicalized_sensitive_len(value, child_depth)?;
        length = length
            .checked_add(escaped_string_len(key)?)
            .and_then(|value| value.checked_add(":".len()))
            .and_then(|value| value.checked_add(value_length))
            .ok_or(JcsError::SerializationError)?;
    }
    Ok(length)
}

fn canonical_array_len(values: &[Value], depth: usize) -> Result<usize, JcsError> {
    let child_depth = descend(depth)?;
    let mut length = "["
        .len()
        .checked_add("]".len())
        .ok_or(JcsError::SerializationError)?;
    for (index, item) in values.iter().enumerate() {
        if index > 0 {
            length = length
                .checked_add(",".len())
                .ok_or(JcsError::SerializationError)?;
        }
        length = length
            .checked_add(canonicalized_len(item, child_depth)?)
            .ok_or(JcsError::SerializationError)?;
    }
    Ok(length)
}

fn canonical_object_len(
    values: &serde_json::Map<String, Value>,
    depth: usize,
) -> Result<usize, JcsError> {
    let child_depth = descend(depth)?;
    let mut keys: Vec<&String> = values.keys().collect();
    keys.sort_by(|left, right| utf16_cmp(left, right));

    let mut length = "{"
        .len()
        .checked_add("}".len())
        .ok_or(JcsError::SerializationError)?;
    for (index, key) in keys.iter().enumerate() {
        if index > 0 {
            length = length
                .checked_add(",".len())
                .ok_or(JcsError::SerializationError)?;
        }
        let value_length = canonicalized_len(&values[*key], child_depth)?;
        length = length
            .checked_add(escaped_string_len(key)?)
            .and_then(|value| value.checked_add(":".len()))
            .and_then(|value| value.checked_add(value_length))
            .ok_or(JcsError::SerializationError)?;
    }
    Ok(length)
}

fn canonical_number_len(value: &serde_json::Number) -> Result<usize, JcsError> {
    if let Some(unsigned) = value.as_u64() {
        if unsigned > MAX_INTEROPERABLE_INTEGER {
            return Err(JcsError::IntegerOutsideInteroperableRange);
        }
        let mut buffer = itoa::Buffer::new();
        return Ok(buffer.format(unsigned).len());
    }
    if let Some(signed) = value.as_i64() {
        if signed < MIN_INTEROPERABLE_INTEGER {
            return Err(JcsError::IntegerOutsideInteroperableRange);
        }
        let mut buffer = itoa::Buffer::new();
        return Ok(buffer.format(signed).len());
    }

    let float = value.as_f64().ok_or(JcsError::SerializationError)?;
    if !float.is_finite() {
        return Err(JcsError::NonFiniteNumber);
    }
    validate_interoperable_float_integer(float)?;

    let mut buffer = ryu_js::Buffer::new();
    Ok(buffer.format_finite(float).len())
}

fn validate_interoperable_float_integer(value: f64) -> Result<(), JcsError> {
    if value.fract() == 0.0
        && !(MIN_INTEROPERABLE_INTEGER_F64..=MAX_INTEROPERABLE_INTEGER_F64).contains(&value)
    {
        return Err(JcsError::IntegerOutsideInteroperableRange);
    }
    Ok(())
}

fn escaped_string_len(value: &str) -> Result<usize, JcsError> {
    let mut length = "\""
        .len()
        .checked_add("\"".len())
        .ok_or(JcsError::SerializationError)?;
    for character in value.chars() {
        length = length
            .checked_add(escaped_character_len(character))
            .ok_or(JcsError::SerializationError)?;
    }
    Ok(length)
}

fn escaped_character_len(character: char) -> usize {
    match character {
        '"' | '\\' | '\u{08}' | '\t' | '\n' | '\u{0c}' | '\r' => 2,
        '\u{00}'..='\u{1f}' => 6,
        _ => character.len_utf8(),
    }
}

fn write_escaped_string(value: &str, output: &mut String) {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\t' => output.push_str("\\t"),
            '\n' => output.push_str("\\n"),
            '\u{0c}' => output.push_str("\\f"),
            '\r' => output.push_str("\\r"),
            '\u{00}'..='\u{1f}' => {
                let byte = character as u8;
                output.push_str("\\u00");
                output.push(char::from(HEX[usize::from(byte >> 4)]));
                output.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
            _ => output.push(character),
        }
    }
    output.push('"');
}
