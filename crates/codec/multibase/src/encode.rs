// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_base64url::bytes_to_base64url;

use crate::{base58btc_encode, Base58Error};

/// Encode bytes using multibase base58btc with the `z` prefix.
pub fn bytes_to_multibase58btc(bytes: &[u8]) -> Result<String, Base58Error> {
    base58btc_encode(bytes).map(|encoded| {
        let mut output = String::with_capacity(encoded.len() + 1);
        output.push('z');
        output.push_str(&encoded);
        output
    })
}

/// Encode bytes using multibase base64url with the `u` prefix.
pub fn bytes_to_multibase_base64url(bytes: &[u8]) -> String {
    format!("u{}", bytes_to_base64url(bytes))
}
