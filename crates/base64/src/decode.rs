// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use base64::{engine::general_purpose::STANDARD, Engine as _};
use zeroize::Zeroizing;

use crate::{Base64DecodeLimits, Base64Error, MAX_BASE64_INPUT_LEN};

const BASE64_QUANTUM: usize = 4;
const DECODED_QUANTUM: usize = 3;
/// Default decoded-byte budget paired with the historical encoded-input cap.
pub const DEFAULT_BASE64_DECODED_OUTPUT_BYTES: usize = 1_572_864;

/// Decode standard padded Base64 with the historical 2 MiB input limit.
/// The returned bytes retain the existing caller-owned cleanup contract.
pub fn base64_to_bytes(input: &str) -> Result<Vec<u8>, Base64Error> {
    let mut output = base64_to_bytes_with_limits(input, &Base64DecodeLimits::default())?;
    Ok(core::mem::take(&mut *output))
}

/// Decode strict padded-standard Base64 into a wiping owner.
///
/// Both schema-owned limits are checked before output allocation. Defaults are
/// independent; raising the input budget does not raise the decoded-byte budget.
/// A data-URL prefix must be removed by the caller before invoking this codec.
pub fn base64_to_bytes_with_limits(
    input: &str,
    limits: &Base64DecodeLimits,
) -> Result<Zeroizing<Vec<u8>>, Base64Error> {
    let input_limit = limit(limits.encoded_input_bytes, MAX_BASE64_INPUT_LEN)?;
    let output_limit = limit(
        limits.decoded_output_bytes,
        DEFAULT_BASE64_DECODED_OUTPUT_BYTES,
    )?;
    if input.len() > input_limit {
        return Err(Base64Error::InputTooLarge);
    }
    if !input.len().is_multiple_of(BASE64_QUANTUM) {
        return Err(Base64Error::Invalid);
    }
    let padding = if input.ends_with("==") {
        2
    } else if input.ends_with('=') {
        1
    } else {
        0
    };
    let decoded = input
        .len()
        .checked_div(BASE64_QUANTUM)
        .and_then(|groups| groups.checked_mul(DECODED_QUANTUM))
        .and_then(|bytes| bytes.checked_sub(padding))
        .ok_or(Base64Error::LengthOverflow)?;
    if decoded > output_limit {
        return Err(Base64Error::OutputTooLarge);
    }
    // Reserve from the validated actual input shape, never from caller limits.
    // Exact padded length also avoids allocating two surplus decoded bytes at
    // an exact output boundary. Any late decode error drops this wiping owner.
    let mut output = Zeroizing::new(Vec::new());
    output
        .try_reserve_exact(decoded)
        .map_err(|_| Base64Error::AllocationFailure)?;
    output.resize(decoded, 0);
    let length = STANDARD
        .decode_slice(input.as_bytes(), output.as_mut_slice())
        .map_err(|_| Base64Error::Invalid)?;
    if length != decoded {
        return Err(Base64Error::Invalid);
    }
    Ok(output)
}

fn limit(value: Option<u64>, default: usize) -> Result<usize, Base64Error> {
    let Some(value) = value else {
        return Ok(default);
    };
    // A slice or allocation cannot exceed isize::MAX even on a 64-bit host.
    // Validate before converting so oversized wire limits fail consistently.
    isize::try_from(value).map_err(|_| Base64Error::InvalidLimits)?;
    usize::try_from(value).map_err(|_| Base64Error::InvalidLimits)
}
