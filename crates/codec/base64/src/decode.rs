// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use base64::{engine::general_purpose::STANDARD, Engine as _};

use crate::error::Base64Error;

/// Decode standard padded Base64 from RFC 4648.
pub fn base64_to_bytes(input: &str) -> Result<Vec<u8>, Base64Error> {
    STANDARD
        .decode(input.as_bytes())
        .map_err(|_| Base64Error::Invalid)
}
