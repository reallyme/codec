// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::read::Container;
use super::{DerError, DerErrorReason, DerLimits, DerReader, DerUnsignedIntegerPair};

/// Decode exactly one DER value using the caller's typed ASN.1 schema.
///
/// The closure cannot make a failed read succeed by ignoring its error. The
/// document and every opened container must be exhausted before success. Input
/// bytes are borrowed; the decoder never assumes ownership or wipes the input.
pub fn decode_der<'a, T>(
    input: &'a [u8],
    limits: DerLimits,
    decode: impl FnOnce(&mut DerReader<'a>) -> Result<T, DerError>,
) -> Result<T, DerError> {
    if input.is_empty() {
        return Err(DerError::new(DerErrorReason::EmptyInput));
    }
    if input.len() > limits.input_len {
        return Err(DerError::new(DerErrorReason::InputTooLarge));
    }
    let mut reader = DerReader::new(input, limits, Container::Document, 0, 0)?;
    let value = decode(&mut reader)?;
    reader.finish()?;
    Ok(value)
}

/// Decode one exact SEQUENCE containing exactly two canonical unsigned INTEGERs.
///
/// Zero is syntactically valid. The consuming crypto API must separately enforce
/// nonzero scalars, curve widths, subgroup orders, and its chosen high-S policy.
pub fn decode_unsigned_integer_pair(
    input: &[u8],
    limits: DerLimits,
) -> Result<DerUnsignedIntegerPair<'_>, DerError> {
    decode_der(input, limits, |reader| {
        reader.read_sequence(|sequence| {
            Ok(DerUnsignedIntegerPair {
                first: sequence.read_unsigned_integer()?,
                second: sequence.read_unsigned_integer()?,
            })
        })
    })
}
