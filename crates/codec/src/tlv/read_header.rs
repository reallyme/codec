// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::error::{overflow, size};
use super::validate_options::ValidatedOptions;
use super::{ErrorReason, Header, Options, TlvError};
use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use zeroize::Zeroizing;

const HIGH_TAG_NUMBER: u8 = 0x1f;
const CONTINUATION: u8 = 0x80;
const LOW_SEVEN_BITS: u8 = 0x7f;
const LENGTH_RADIX: u64 = 256;

pub(super) fn identifier_length(
    input: &[u8],
    options: &ValidatedOptions,
) -> Result<usize, TlvError> {
    let first = *input
        .first()
        .ok_or(TlvError::new(ErrorReason::EmptyInput))?;
    // End-of-contents terminates an indefinite container; it is never an outer
    // definite value. Recovery from padding belongs to the consuming protocol.
    if first == 0 {
        return Err(TlvError::new(ErrorReason::InvalidIdentifier));
    }
    let mut offset = 1usize;
    if first & HIGH_TAG_NUMBER != HIGH_TAG_NUMBER {
        return Ok(offset);
    }
    loop {
        if offset >= options.identifier_octets {
            return Err(TlvError::new(ErrorReason::IdentifierLimit));
        }
        let byte = *input
            .get(offset)
            .ok_or(TlvError::new(ErrorReason::TruncatedIdentifier))?;
        // DER requires a nonzero initial base-128 group and high-tag form only
        // for tag numbers >= 31. ISO 7816 retains its encoded tag identifier;
        // applying ASN.1's numeric minimality would wrongly reject 5F01.
        if offset == 1 && byte & LOW_SEVEN_BITS == 0 {
            return Err(TlvError::new(if options.der {
                ErrorReason::NoncanonicalIdentifier
            } else {
                ErrorReason::InvalidIdentifier
            }));
        }
        offset = offset.checked_add(1).ok_or_else(overflow)?;
        if byte & CONTINUATION == 0 {
            if options.der && offset == 2 && byte < HIGH_TAG_NUMBER {
                return Err(TlvError::new(ErrorReason::NoncanonicalIdentifier));
            }
            return Ok(offset);
        }
    }
}

pub(super) fn span(offset: usize, length: usize) -> Result<proto::CodecByteSpan, TlvError> {
    Ok(proto::CodecByteSpan {
        offset: size(offset)?,
        length: size(length)?,
    })
}

/// Read a complete identifier and definite length, without requiring any value
/// bytes. Options and actual-input limits are validated before header parsing.
pub fn read_header<'a>(input: &'a [u8], options: &Options) -> Result<Header<'a>, TlvError> {
    let options = ValidatedOptions::for_input(input, options)?;
    let identifier_end = identifier_length(input, &options)?;
    let length_source = input.get(identifier_end..).ok_or_else(overflow)?;
    let (length_size, declared) = measure_length(length_source, &options)?;
    let header_end = identifier_end
        .checked_add(length_size)
        .ok_or_else(overflow)?;
    Ok(Header {
        identifier: input.get(..identifier_end).ok_or_else(overflow)?,
        length_octets: input.get(identifier_end..header_end).ok_or_else(overflow)?,
        received: input.get(..header_end).ok_or_else(overflow)?,
        remainder: input.get(header_end..).ok_or_else(overflow)?,
        receipt: proto::CodecTlvHeader {
            identifier: Some(span(0, identifier_end)?).into(),
            length_octets: Some(span(identifier_end, length_size)?).into(),
            header_bytes: Some(span(0, header_end)?).into(),
            declared_value_length: *declared,
            consumed_bytes: size(header_end)?,
        },
    })
}

// Parse the same definite length whether it begins a source or follows a tag.
pub(super) fn measure_length(
    input: &[u8],
    options: &ValidatedOptions,
) -> Result<(usize, Zeroizing<u64>), TlvError> {
    let discriminator = *input
        .first()
        .ok_or(TlvError::new(ErrorReason::TruncatedLength))?;
    let payload_start = 1_usize;
    let mut declared = Zeroizing::new(u64::from(discriminator));
    let length_end = if discriminator & CONTINUATION == 0 {
        payload_start
    } else {
        let count = usize::from(discriminator & LOW_SEVEN_BITS);
        if count == 0 {
            return Err(TlvError::new(ErrorReason::UnsupportedIndefiniteLength));
        }
        if discriminator == u8::MAX {
            return Err(TlvError::new(ErrorReason::InvalidLength));
        }
        if count > options.length_octets {
            return Err(TlvError::new(ErrorReason::LengthLimit));
        }
        let end = payload_start.checked_add(count).ok_or_else(overflow)?;
        let bytes = input
            .get(payload_start..end)
            .ok_or(TlvError::new(ErrorReason::TruncatedLength))?;
        *declared = 0;
        for byte in bytes {
            *declared = declared
                .checked_mul(LENGTH_RADIX)
                .and_then(|value| value.checked_add(u64::from(*byte)))
                .ok_or_else(overflow)?;
        }
        if options.der && (bytes.first() == Some(&0) || *declared < u64::from(CONTINUATION)) {
            return Err(TlvError::new(ErrorReason::NoncanonicalLength));
        }
        end
    };
    Ok((length_end, declared))
}
