// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use base64::{decoded_len_estimate, engine::general_purpose::STANDARD, Engine as _};
use zeroize::Zeroizing;

use crate::{PemDecodePolicy, PemDocument, PemError, PemLabel};

const BEGIN_PREFIX: &str = "-----BEGIN ";
const END_PREFIX: &str = "-----END ";
const BOUNDARY_SUFFIX: &str = "-----";

/// Decode PEM text armor into a label and DER body.
pub fn decode_pem(input: &str, policy: PemDecodePolicy<'_>) -> Result<PemDocument, PemError> {
    if input.is_empty() || input.len() > policy.max_input_len {
        return Err(PemError::InputTooLarge);
    }
    if policy.max_der_len == 0 || policy.allowed_labels.is_empty() {
        return Err(PemError::InvalidOptions);
    }

    let normalized = Zeroizing::new(normalize_line_endings(input)?);
    let mut lines = normalized.split('\n');

    let begin_line = next_nonempty_line(&mut lines).ok_or(PemError::MissingBegin)?;
    let begin_label = parse_boundary_label(begin_line, BEGIN_PREFIX)?;
    let label = PemLabel::parse(begin_label)?;
    if !policy.allowed_labels.contains(&label) {
        return Err(PemError::UnsupportedLabel);
    }

    let encoded_limit = encoded_len_limit(policy.max_der_len)?;
    // Every body byte is drawn from `input`, so this upper bound prevents the
    // secret-bearing String from reallocating while it is assembled.
    let body_capacity = input.len().min(encoded_limit);
    let mut body = Zeroizing::new(String::with_capacity(body_capacity));
    let mut found_end = false;

    for line in lines {
        if line.is_empty() {
            continue;
        }
        if found_end {
            return Err(PemError::InvalidBoundary);
        }
        if line.starts_with(END_PREFIX) {
            let end_label = parse_boundary_label(line, END_PREFIX)?;
            if end_label != label.as_str() {
                return Err(PemError::LabelMismatch);
            }
            found_end = true;
            continue;
        }
        if line.starts_with(BEGIN_PREFIX) {
            return Err(PemError::InvalidBoundary);
        }
        if !line.bytes().all(is_base64_body_byte) {
            return Err(PemError::InvalidBody);
        }
        let next_len = body
            .len()
            .checked_add(line.len())
            .ok_or(PemError::InvalidOptions)?;
        if next_len > encoded_limit {
            return Err(PemError::DerTooLarge);
        }
        body.push_str(line);
    }

    if !found_end {
        return Err(PemError::MissingEnd);
    }
    if body.is_empty() {
        return Err(PemError::InvalidBody);
    }

    // Decode into one conservatively sized allocation. `Zeroizing<Vec<_>>`
    // wipes the full capacity, including the unused estimate tail, on drop.
    let mut der = Zeroizing::new(vec![0_u8; decoded_len_estimate(body.len())]);
    let decoded_length = STANDARD
        .decode_slice(body.as_bytes(), der.as_mut_slice())
        .map_err(|_| PemError::InvalidBase64)?;
    der.truncate(decoded_length);
    if der.is_empty() || der.len() > policy.max_der_len {
        return Err(PemError::DerTooLarge);
    }

    Ok(PemDocument { label, der })
}

fn normalize_line_endings(input: &str) -> Result<String, PemError> {
    // The normalized text cannot exceed the original byte length. Reserving
    // that exact upper bound avoids the chained `replace` allocations that
    // previously left private-key armor in freed heap blocks.
    let mut output = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut cursor = 0_usize;
    while cursor < bytes.len() {
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor] != b'\r' {
            cursor = cursor.checked_add(1).ok_or(PemError::InvalidOptions)?;
        }
        output.push_str(&input[start..cursor]);
        if cursor == bytes.len() {
            break;
        }
        output.push('\n');
        cursor = cursor.checked_add(1).ok_or(PemError::InvalidOptions)?;
        if cursor < bytes.len() && bytes[cursor] == b'\n' {
            cursor = cursor.checked_add(1).ok_or(PemError::InvalidOptions)?;
        }
    }
    Ok(output)
}

fn next_nonempty_line<'a>(lines: &mut impl Iterator<Item = &'a str>) -> Option<&'a str> {
    lines.find(|line| !line.is_empty())
}

fn parse_boundary_label<'a>(line: &'a str, prefix: &str) -> Result<&'a str, PemError> {
    let remainder = line.strip_prefix(prefix).ok_or(PemError::InvalidBoundary)?;
    let label = remainder
        .strip_suffix(BOUNDARY_SUFFIX)
        .ok_or(PemError::InvalidBoundary)?;
    if label.is_empty() || label.as_bytes().iter().any(|byte| !is_label_byte(*byte)) {
        return Err(PemError::InvalidBoundary);
    }
    Ok(label)
}

fn encoded_len_limit(max_der_len: usize) -> Result<usize, PemError> {
    let groups = max_der_len.checked_add(2).ok_or(PemError::InvalidOptions)? / 3;
    groups.checked_mul(4).ok_or(PemError::InvalidOptions)
}

fn is_label_byte(byte: u8) -> bool {
    byte == b' ' || byte == b'-' || byte.is_ascii_uppercase() || byte.is_ascii_digit()
}

fn is_base64_body_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=')
}
