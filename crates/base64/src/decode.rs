// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use base64::{decoded_len_estimate, engine::general_purpose::STANDARD, Engine as _};
use zeroize::Zeroizing;

use crate::error::Base64Error;

/// Decode standard padded Base64 from RFC 4648.
pub fn base64_to_bytes(input: &str) -> Result<Vec<u8>, Base64Error> {
    // A late alphabet or padding error may follow successfully decoded secret
    // bytes. Keep ownership here so the partial output is wiped on failure.
    let mut output = Zeroizing::new(vec![0_u8; decoded_len_estimate(input.len())]);
    let length = STANDARD
        .decode_slice(input.as_bytes(), output.as_mut_slice())
        .map_err(|_| Base64Error::Invalid)?;
    output.truncate(length);
    Ok(core::mem::take(&mut *output))
}
