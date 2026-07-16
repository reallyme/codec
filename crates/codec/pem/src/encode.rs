// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use base64::{engine::general_purpose::STANDARD, Engine as _};
use zeroize::Zeroizing;

use crate::{PemEncodeOptions, PemError, PemLabel};

/// Encode DER bytes as PEM text armor.
pub fn encode_pem(
    label: PemLabel,
    der: &[u8],
    options: PemEncodeOptions,
) -> Result<Zeroizing<String>, PemError> {
    if der.is_empty() || der.len() > options.max_der_len {
        return Err(PemError::DerTooLarge);
    }
    if options.line_width == 0 || options.line_width > 76 {
        return Err(PemError::InvalidOptions);
    }

    let newline = options.line_ending.as_str();
    let encoded = Zeroizing::new(STANDARD.encode(der));
    let mut output = Zeroizing::new(String::new());
    output.push_str("-----BEGIN ");
    output.push_str(label.as_str());
    output.push_str("-----");
    output.push_str(newline);

    for chunk in encoded.as_bytes().chunks(options.line_width) {
        let line = core::str::from_utf8(chunk).map_err(|_| PemError::InvalidBase64)?;
        output.push_str(line);
        output.push_str(newline);
    }

    output.push_str("-----END ");
    output.push_str(label.as_str());
    output.push_str("-----");
    output.push_str(newline);

    Ok(output)
}
