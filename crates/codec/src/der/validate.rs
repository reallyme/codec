// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{DerError, DerErrorReason};

const BASE_128_CONTINUATION: u8 = 0x80;

pub(super) fn validate_oid(bytes: &[u8]) -> Result<(), DerError> {
    if bytes.is_empty() {
        return Err(DerError::new(DerErrorReason::InvalidValue));
    }
    // X.690 8.19 encodes the first two arcs as one subidentifier, which may
    // itself be multibyte. All subidentifiers use minimal unsigned base-128;
    // retaining their octets avoids narrowing valid large arcs to u32/u64.
    let mut start_of_subidentifier = true;
    for byte in bytes {
        if start_of_subidentifier && *byte == BASE_128_CONTINUATION {
            return Err(DerError::new(DerErrorReason::NonCanonicalEncoding));
        }
        start_of_subidentifier = byte & BASE_128_CONTINUATION == 0;
    }
    if !start_of_subidentifier {
        return Err(DerError::new(DerErrorReason::InvalidValue));
    }
    Ok(())
}
