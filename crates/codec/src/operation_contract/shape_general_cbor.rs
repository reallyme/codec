// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::execute_general_cbor::map_error;
use codec_cbor::general::{ArgumentWidth, CborError, ErrorReason, Frame, Header, Limits, Profile};
use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::CodecWireError;

fn overflow() -> CodecWireError {
    map_error(CborError::new(ErrorReason::ArithmeticOverflow))
}
pub(super) fn size(value: usize) -> Result<u64, CodecWireError> {
    u64::try_from(value).map_err(|_| overflow())
}

pub(super) fn span(source: &[u8], bytes: &[u8]) -> Result<proto::CodecByteSpan, CodecWireError> {
    let offset = bytes
        .as_ptr()
        .addr()
        .checked_sub(source.as_ptr().addr())
        .ok_or_else(overflow)?;
    let end = offset.checked_add(bytes.len()).ok_or_else(overflow)?;
    if end > source.len() {
        return Err(overflow());
    }
    Ok(proto::CodecByteSpan {
        offset: size(offset)?,
        length: size(bytes.len())?,
    })
}

pub(super) fn body<'a>(frame: &Frame<'a>) -> Result<&'a [u8], CodecWireError> {
    let start = frame.header().received_bytes().len();
    let end = if frame.header().argument_width() == ArgumentWidth::Indefinite {
        frame
            .received_bytes()
            .len()
            .checked_sub(1)
            .ok_or_else(overflow)?
    } else {
        frame.received_bytes().len()
    };
    frame.received_bytes().get(start..end).ok_or_else(overflow)
}

pub(super) fn header(value: &Header<'_>) -> Result<proto::CodecGeneralCborHeader, CodecWireError> {
    let mut result = proto::CodecGeneralCborHeader::default();
    result.major_type = value.major_type().into();
    result.width = value.argument_width().into();
    result.argument = value.argument();
    result.encoded_length = size(value.received_bytes().len())?;
    result.preferred_argument = value.has_preferred_argument();
    result.preferred_float = value.has_preferred_float().map_err(map_error)?;
    if let Some(float) = value.float().map_err(map_error)? {
        result.float_value = Some(proto::CodecGeneralCborFloat {
            width: float.width().into(),
            bits: float.bits(),
        })
        .into();
    }
    Ok(result)
}

pub(super) fn options(
    profile: Profile,
    limits: Limits,
) -> Result<proto::CodecGeneralCborOptions, CodecWireError> {
    Ok(proto::CodecGeneralCborOptions {
        serialization: profile.serialization().into(),
        lengths: profile.lengths().into(),
        floats: profile.floats().into(),
        ordering: profile.map_ordering().into(),
        input_bytes: Some(size(limits.input_bytes())?),
        output_bytes: Some(size(limits.output_bytes())?),
        depth: Some(size(limits.depth())?),
        nodes: Some(size(limits.nodes())?),
        entries: Some(size(limits.entries())?),
        byte_string_bytes: Some(size(limits.byte_string_bytes())?),
        text_bytes: Some(size(limits.text_bytes())?),
        work: Some(size(limits.work())?),
    })
}

pub(super) fn frame(
    source: &[u8],
    value: &Frame<'_>,
    validated: bool,
    profile: Profile,
    limits: Limits,
) -> Result<proto::CodecGeneralCborFrame, CodecWireError> {
    Ok(proto::CodecGeneralCborFrame {
        received: Some(span(source, value.received_bytes())?).into(),
        header_bytes: Some(span(source, value.header().received_bytes())?).into(),
        body: Some(span(source, body(value)?)?).into(),
        header: Some(header(value.header())?).into(),
        validated,
        options: Some(options(profile, limits)?).into(),
    })
}

// A map exposes alternating key/value receipts. Apply that same finite
// collection ceiling to wire receipt arrays before copying scalar metadata.
const RECEIPTS_PER_ENTRY: usize = 2;

pub(super) fn receipt_capacity<T>(
    values: impl Iterator<Item = Result<T, CborError>>,
    limits: Limits,
) -> Result<usize, CodecWireError> {
    let limit = limits
        .entries()
        .checked_mul(RECEIPTS_PER_ENTRY)
        .ok_or_else(overflow)?;
    let mut count = 0usize;
    for value in values {
        let _value = value.map_err(map_error)?;
        count = count.checked_add(1).ok_or_else(overflow)?;
        if count > limit {
            return Err(map_error(CborError::new(ErrorReason::EntryLimit)));
        }
    }
    Ok(count)
}

// The counted ceiling is established before any scalar-bearing receipt enters
// the collection, preserving fallible allocation without secret reallocation.
pub(super) fn reserve_receipts<T: zeroize::Zeroize>(
    capacity: usize,
) -> Result<buffa::sensitive::List<T>, CodecWireError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(capacity)
        .map_err(|_| super::internal_wire_error())?;
    Ok(values.into())
}
