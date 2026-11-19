// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Bounded PEM admission into generated guarded block owners.

use crate::{
    frame_bundle,
    policy::{DEFAULT_MAX_DER_LEN, DEFAULT_MAX_PEM_INPUT_LEN},
    CodecPemBundleBlock, CodecPemBundleDecodeOptions, CodecPemBundleDecodeResult,
    CodecPemDecodeScope, CodecPemSyntaxProfile, PemError,
};
use base64::{decoded_len_estimate, engine::general_purpose::STANDARD, Engine as _};
use zeroize::Zeroizing;
const DEFAULT_MAX_BLOCKS: usize = 64;

fn limit(configured: u32, default: usize) -> Result<usize, PemError> {
    if configured == 0 {
        Ok(default)
    } else {
        usize::try_from(configured).map_err(|_| PemError::InvalidOptions)
    }
}

/// Decode bounded PEM blocks into schema-generated owners that clear labels
/// and DER on success/drop and on every partially completed failure path.
///
/// The default STRICT/ALL profile requires standalone, newline-delimited
/// armor. COMPATIBLE retains prose, inline boundaries, Unicode body whitespace,
/// colon-delimited legacy headers, empty bodies and ignored incomplete tails.
/// FIRST stops after the first complete block. No DER interpretation or label
/// allowlist is applied; certificate selection and trust stay with the caller.
pub fn decode_pem_bundle(
    input: &[u8],
    options: &CodecPemBundleDecodeOptions,
) -> Result<CodecPemBundleDecodeResult, PemError> {
    let max_input = limit(options.max_input_len, DEFAULT_MAX_PEM_INPUT_LEN)?;
    let max_der = limit(options.max_der_len, DEFAULT_MAX_DER_LEN)?;
    let max_blocks = limit(options.max_blocks, DEFAULT_MAX_BLOCKS)?;
    if input.len() > max_input {
        return Err(PemError::InputTooLarge);
    }
    let compatible = match options.syntax_profile.as_known() {
        Some(
            CodecPemSyntaxProfile::CODEC_PEM_SYNTAX_PROFILE_UNSPECIFIED
            | CodecPemSyntaxProfile::CODEC_PEM_SYNTAX_PROFILE_STRICT,
        ) => false,
        Some(CodecPemSyntaxProfile::CODEC_PEM_SYNTAX_PROFILE_COMPATIBLE) => true,
        _ => return Err(PemError::InvalidOptions),
    };
    let first = match options.scope.as_known() {
        Some(
            CodecPemDecodeScope::CODEC_PEM_DECODE_SCOPE_UNSPECIFIED
            | CodecPemDecodeScope::CODEC_PEM_DECODE_SCOPE_ALL,
        ) => false,
        Some(CodecPemDecodeScope::CODEC_PEM_DECODE_SCOPE_FIRST) => true,
        _ => return Err(PemError::InvalidOptions),
    };
    let mut result = CodecPemBundleDecodeResult::default();
    let mut remaining = input;
    while let Some(frame) = frame_bundle::next(remaining, compatible)? {
        if result.blocks.len() >= max_blocks {
            return Err(PemError::ResourceLimitExceeded);
        }
        let encoded_limit = max_der
            .checked_add(2)
            .and_then(|value| value.checked_div(3))
            .and_then(|value| value.checked_mul(4))
            .ok_or(PemError::InvalidOptions)?;
        // Count before allocating. Never grow a buffer that contains armor or
        // decoded identifying bytes, including during malformed-base64 failure.
        let count = frame
            .body
            .chars()
            .filter(|character| !character.is_whitespace())
            .try_fold(0_usize, |count, _| {
                count.checked_add(1).ok_or(PemError::ResourceLimitExceeded)
            })?;
        if count > encoded_limit {
            return Err(PemError::DerTooLarge);
        }
        let mut body = Zeroizing::new(Vec::new());
        body.try_reserve_exact(count)
            .map_err(|_| PemError::ResourceLimitExceeded)?;
        for character in frame
            .body
            .chars()
            .filter(|character| !character.is_whitespace())
        {
            if !character.is_ascii() {
                return Err(PemError::InvalidBase64);
            }
            let byte = u8::try_from(u32::from(character)).map_err(|_| PemError::InvalidBase64)?;
            body.push(byte);
        }
        let capacity = decoded_len_estimate(body.len());
        let mut der = Zeroizing::new(Vec::new());
        der.try_reserve_exact(capacity)
            .map_err(|_| PemError::ResourceLimitExceeded)?;
        der.resize(capacity, 0);
        let length = STANDARD
            .decode_slice(&body, der.as_mut_slice())
            .map_err(|_| PemError::InvalidBase64)?;
        if length > max_der {
            return Err(PemError::DerTooLarge);
        }
        if length == 0 && !compatible {
            return Err(PemError::InvalidBody);
        }
        der.truncate(length);
        let block = CodecPemBundleBlock::default()
            .with_der(core::mem::take(&mut *der))
            .with_label(frame.label.to_owned());
        result.blocks.push(block);
        remaining = frame.remaining;
        if first {
            break;
        }
    }
    if result.blocks.is_empty() && !compatible {
        return Err(PemError::MissingBegin);
    }
    Ok(result)
}
