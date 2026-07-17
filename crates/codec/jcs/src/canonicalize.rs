// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use std::cmp::Ordering;

use serde_json::Value;

use crate::error::JcsError;
use crate::parse_json::parse_json_text;

const MAX_INTEROPERABLE_INTEGER: u64 = 9_007_199_254_740_991;
const MIN_INTEROPERABLE_INTEGER: i64 = -9_007_199_254_740_991;

/// Canonicalize a trusted, already-materialized JSON value according to RFC
/// 8785 (JSON Canonicalization Scheme).
///
/// Integer values stored exactly as `i64` or `u64` are rejected outside the
/// interoperable `[-(2^53)+1, (2^53)-1]` range. This function cannot determine
/// whether a source document contained duplicate object member names because
/// [`Value`] has already discarded that provenance. It is therefore intended
/// only for values constructed programmatically or produced by a parser that
/// already enforced the same duplicate-member policy. Call
/// [`canonicalize_json_text`] at every untrusted text boundary.
pub fn canonicalize_trusted_json_value(value: &Value) -> Result<String, JcsError> {
    canonicalize(value, 0)
}

/// Deprecated alias for [`canonicalize_trusted_json_value`].
///
/// The old name did not communicate that duplicate-member provenance has
/// already been lost. New code must select the explicitly trusted-value API or
/// the raw-text boundary API.
#[deprecated(
    since = "0.1.22",
    note = "use canonicalize_json_text at text boundaries or canonicalize_trusted_json_value for trusted Values"
)]
pub fn canonicalize_json(value: &Value) -> Result<String, JcsError> {
    canonicalize_trusted_json_value(value)
}

/// Parse and canonicalize one untrusted JSON text according to RFC 8785.
///
/// Unlike deserializing directly into [`Value`], this entry point detects and
/// rejects duplicate object member names instead of silently retaining one
/// value. It also rejects trailing data. Integer tokens retained exactly as
/// `i64` or `u64` are rejected outside the interoperable range, while numbers
/// parsed as binary64 follow RFC 8785's required ECMAScript rounding behavior.
pub fn canonicalize_json_text(input: &str) -> Result<String, JcsError> {
    let value = parse_json_text(input)?;
    canonicalize_trusted_json_value(&value)
}

/// `depth` counts the array/object containers currently open, bounded by
/// [`MAX_NESTING_DEPTH`](crate::MAX_NESTING_DEPTH) as defense in depth
/// against a `Value` that was built without a parser depth limit.
fn canonicalize(value: &Value, depth: usize) -> Result<String, JcsError> {
    match value {
        Value::Null => Ok("null".to_owned()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => canonicalize_number(value),
        Value::String(value) => {
            serde_json::to_string(value).map_err(|_| JcsError::SerializationError)
        }
        Value::Array(values) => canonicalize_array(values, depth),
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

            let mut output = String::from("{");
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(
                    &serde_json::to_string(key).map_err(|_| JcsError::SerializationError)?,
                );
                output.push(':');
                output.push_str(&canonicalize(&values[*key], child_depth)?);
            }
            output.push('}');
            Ok(output)
        }
    }
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

fn canonicalize_number(value: &serde_json::Number) -> Result<String, JcsError> {
    if let Some(unsigned) = value.as_u64() {
        if unsigned > MAX_INTEROPERABLE_INTEGER {
            return Err(JcsError::IntegerOutsideInteroperableRange);
        }
        return Ok(unsigned.to_string());
    }
    if let Some(signed) = value.as_i64() {
        if signed < MIN_INTEROPERABLE_INTEGER {
            return Err(JcsError::IntegerOutsideInteroperableRange);
        }
        return Ok(signed.to_string());
    }

    let float = value.as_f64().ok_or(JcsError::SerializationError)?;
    if !float.is_finite() {
        return Err(JcsError::NonFiniteNumber);
    }

    // RFC 8785 §3.2.2.3 mandates the ECMAScript `Number.prototype.toString`
    // algorithm (exponent thresholds, `+`/`-` exponent sign, shortest
    // round-trip digits). `serde_json`/`ryu` do not follow the ES6
    // exponent rules — e.g. 1e21 must serialize as `1e+21`, not `1e21` —
    // so we use `ryu-js`, whose output is defined to match ES6 exactly.
    let mut buffer = ryu_js::Buffer::new();
    Ok(buffer.format_finite(float).to_owned())
}

fn canonicalize_array(values: &[Value], depth: usize) -> Result<String, JcsError> {
    let child_depth = descend(depth)?;
    let mut output = String::from("[");
    for (index, item) in values.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&canonicalize(item, child_depth)?);
    }
    output.push(']');
    Ok(output)
}
