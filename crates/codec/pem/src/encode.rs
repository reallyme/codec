// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use base64::{encoded_len, engine::general_purpose::STANDARD, Engine as _};
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

    let encoded_length = encoded_len(der.len(), true).ok_or(PemError::InvalidOptions)?;
    let mut encoded = Zeroizing::new(vec![0_u8; encoded_length]);
    let written = STANDARD
        .encode_slice(der, encoded.as_mut_slice())
        .map_err(|_| PemError::InvalidOptions)?;
    if written != encoded_length {
        return Err(PemError::InvalidOptions);
    }

    let newline = options.line_ending.as_str();
    let output_length = encoded_pem_length(
        label.as_str().len(),
        encoded_length,
        options.line_width,
        newline.len(),
    )?;
    // Allocate the final secret-bearing buffer once. Growing a String after
    // private-key armor has been written would free prior allocations without
    // wiping them; an exact capacity prevents that remanence path.
    let mut output = Zeroizing::new(String::with_capacity(output_length));
    output.push_str("-----BEGIN ");
    output.push_str(label.as_str());
    output.push_str("-----");
    output.push_str(newline);

    for chunk in encoded.as_slice().chunks(options.line_width) {
        let line = core::str::from_utf8(chunk).map_err(|_| PemError::InvalidBase64)?;
        output.push_str(line);
        output.push_str(newline);
    }

    output.push_str("-----END ");
    output.push_str(label.as_str());
    output.push_str("-----");
    output.push_str(newline);

    if output.len() != output_length {
        return Err(PemError::InvalidOptions);
    }

    Ok(output)
}

fn encoded_pem_length(
    label_length: usize,
    encoded_length: usize,
    line_width: usize,
    newline_length: usize,
) -> Result<usize, PemError> {
    let boundary_length = "-----BEGIN "
        .len()
        .checked_add(label_length)
        .and_then(|length| length.checked_add("-----".len()))
        .and_then(|length| length.checked_add(newline_length))
        .ok_or(PemError::InvalidOptions)?;
    let footer_length = "-----END "
        .len()
        .checked_add(label_length)
        .and_then(|length| length.checked_add("-----".len()))
        .and_then(|length| length.checked_add(newline_length))
        .ok_or(PemError::InvalidOptions)?;
    let line_count = encoded_length
        .checked_add(line_width.checked_sub(1).ok_or(PemError::InvalidOptions)?)
        .ok_or(PemError::InvalidOptions)?
        / line_width;
    let body_newlines = line_count
        .checked_mul(newline_length)
        .ok_or(PemError::InvalidOptions)?;

    boundary_length
        .checked_add(encoded_length)
        .and_then(|length| length.checked_add(body_newlines))
        .and_then(|length| length.checked_add(footer_length))
        .ok_or(PemError::InvalidOptions)
}
