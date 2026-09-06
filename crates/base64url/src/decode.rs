// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use base64::{decoded_len_estimate, engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use zeroize::Zeroizing;

use crate::Base64UrlError;

/// Decode RFC 4648 section 5 Base64URL without padding.
pub fn base64url_to_bytes(s: &str) -> Result<Vec<u8>, Base64UrlError> {
    base64url_bytes_to_bytes(s.as_bytes())
}

/// Decode RFC 4648 section 5 Base64URL bytes without padding.
///
/// This is useful for compact JWT/JWS/JWE segments that have already been
/// split as bytes. It avoids requiring callers to first prove UTF-8 just to
/// decode an ASCII Base64URL alphabet.
pub fn base64url_bytes_to_bytes(bytes: &[u8]) -> Result<Vec<u8>, Base64UrlError> {
    // Retain the partially decoded allocation until validation completes so
    // malformed input cannot leave decoded material in a freed heap block.
    let mut output = Zeroizing::new(vec![0_u8; decoded_len_estimate(bytes.len())]);
    let length = URL_SAFE_NO_PAD
        .decode_slice(bytes, output.as_mut_slice())
        .map_err(|_| Base64UrlError::Invalid)?;
    output.truncate(length);
    Ok(core::mem::take(&mut *output))
}
