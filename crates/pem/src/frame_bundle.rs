// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Borrowed armor boundaries; no identifying bytes are copied while framing.

use crate::PemError;
const BEGIN: &[u8] = b"-----BEGIN ";
const END: &[u8] = b"-----END ";
const SUFFIX: &[u8] = b"-----";

pub(super) struct Frame<'a> {
    pub(super) label: &'a str,
    pub(super) body: &'a str,
    pub(super) remaining: &'a [u8],
}

fn split_marker<'a>(
    input: &'a [u8],
    marker: &[u8],
    compatible: bool,
) -> Option<(&'a [u8], &'a [u8])> {
    let offset = if compatible {
        // Preserve the established parser's non-overlapping marker admission,
        // including incomplete armor with surplus hyphens. Restarting at every
        // byte would admit blocks that the compatibility profile must ignore.
        let mut matched = 0_usize;
        let mut offset = None;
        for (index, byte) in input.iter().enumerate() {
            matched = if marker.get(matched) == Some(byte) {
                matched.checked_add(1)?
            } else {
                0
            };
            if matched == marker.len() {
                offset = Some(index.checked_add(1)?.checked_sub(matched)?);
                break;
            }
        }
        offset?
    } else {
        input
            .windows(marker.len())
            .position(|part| part == marker)?
    };
    let end = offset.checked_add(marker.len())?;
    Some((input.get(..offset)?, input.get(end..)?))
}

pub(super) fn next(input: &[u8], compatible: bool) -> Result<Option<Frame<'_>>, PemError> {
    if input.iter().all(u8::is_ascii_whitespace) {
        return Ok(None);
    }
    let Some((prefix, tail)) = split_marker(input, BEGIN, compatible) else {
        return if compatible {
            Ok(None)
        } else {
            Err(PemError::MissingBegin)
        };
    };
    if !compatible
        && (!prefix
            .iter()
            .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
            || prefix
                .last()
                .is_some_and(|byte| !matches!(byte, b'\r' | b'\n')))
    {
        return Err(PemError::InvalidBoundary);
    }
    let Some((label, tail)) = split_marker(tail, SUFFIX, compatible) else {
        return if compatible {
            Ok(None)
        } else {
            Err(PemError::InvalidBoundary)
        };
    };
    let Some((body, tail)) = split_marker(tail, END, compatible) else {
        return if compatible {
            Ok(None)
        } else {
            Err(PemError::MissingEnd)
        };
    };
    let Some((end_label, remaining)) = split_marker(tail, SUFFIX, compatible) else {
        return if compatible {
            Ok(None)
        } else {
            Err(PemError::InvalidBoundary)
        };
    };
    let label = core::str::from_utf8(label).map_err(|_| PemError::InvalidBoundary)?;
    if label.is_empty() || end_label.is_empty() {
        return Err(PemError::InvalidBoundary);
    }
    if label.as_bytes() != end_label {
        return Err(PemError::LabelMismatch);
    }
    let mut body = core::str::from_utf8(body).map_err(|_| PemError::InvalidBody)?;
    if compatible {
        // Compatibility accepts prose, inline armor and the historical header
        // block. Headers are syntax-checked while still borrowed, before DER
        // decoding, so malformed headers never abandon a decoded allocation.
        body = body.trim_start_matches([' ', '\t', '\r', '\n']);
        let header = split_marker(body.as_bytes(), b"\n\n", true)
            .or_else(|| split_marker(body.as_bytes(), b"\r\n\r\n", true));
        if let Some((headers, payload)) = header {
            let headers = core::str::from_utf8(headers).map_err(|_| PemError::InvalidBody)?;
            if headers.lines().any(|line| !line.contains(':')) {
                return Err(PemError::InvalidBody);
            }
            body = core::str::from_utf8(payload).map_err(|_| PemError::InvalidBody)?;
        }
    } else {
        if !valid_strict_label(label) {
            return Err(PemError::InvalidBoundary);
        }
        if !(body.starts_with('\n') || body.starts_with('\r'))
            || !(body.ends_with('\n') || body.ends_with('\r'))
            || remaining
                .first()
                .is_some_and(|byte| !matches!(byte, b'\r' | b'\n'))
        {
            return Err(PemError::InvalidBoundary);
        }
        if body.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric() && !matches!(byte, b'+' | b'/' | b'=' | b'\r' | b'\n')
        }) {
            return Err(PemError::InvalidBody);
        }
    }
    Ok(Some(Frame {
        label,
        body,
        remaining,
    }))
}

// RFC 7468 section 2 forbids leading, trailing and consecutive separators.
// The uppercase profile keeps exact label spelling; it never repairs case.
fn valid_strict_label(label: &str) -> bool {
    let mut separator = true;
    for byte in label.bytes() {
        if matches!(byte, b' ' | b'-') {
            if separator {
                return false;
            }
            separator = true;
        } else {
            if !(b'!'..=b'~').contains(&byte) || byte.is_ascii_lowercase() {
                return false;
            }
            separator = false;
        }
    }
    !separator
}
