// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_base64url::base64url_to_bytes;

use crate::{base58btc_decode, MultibaseError};

/// Decode a supported multibase string to raw bytes.
///
/// The multibase prefix is the first Unicode scalar value, which may be
/// more than one byte. We therefore split on a character boundary via the
/// char iterator rather than a byte index: slicing at byte 1 would panic
/// on any input whose first character is multi-byte (e.g. `"é"`), and this
/// function is reached with untrusted input through
/// [`parse_multikey`](../../multikey/index.html).
pub fn multibase_to_bytes(multibase: &str) -> Result<Vec<u8>, MultibaseError> {
    let mut chars = multibase.chars();
    let prefix = chars.next().ok_or(MultibaseError::TooShort)?;
    // The remainder of the string after the prefix character.
    let data = chars.as_str();
    match prefix {
        'z' => Ok(base58btc_decode(data)?),
        'u' => base64url_to_bytes(data).map_err(|_| MultibaseError::Base64Url),
        _ => Err(MultibaseError::UnsupportedPrefix),
    }
}
