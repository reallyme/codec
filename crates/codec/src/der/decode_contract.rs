// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use der_core::{Decode, EncodingRules, Header, Reader, SliceReader};

use super::error::map_backend;
use super::read::Container;
use super::{DerError, DerErrorReason, DerLimits, DerReader};

/// The wire schema selects the same reader methods used by borrowed callers.
/// Returned spans refer to the received owner, never to reconstructed DER.
pub(crate) fn decode_contract(
    input: &[u8],
    limits: DerLimits,
    schema: &proto::CodecDerSchema,
    exact: bool,
) -> Result<proto::CodecDerReadResult, DerError> {
    if input.is_empty() {
        return Err(DerError::new(DerErrorReason::EmptyInput));
    }
    if input.len() > limits.input_len {
        return Err(DerError::new(DerErrorReason::InputTooLarge));
    }
    let mut reader = DerReader::new(input, limits, Container::Document, 0, 0)?;
    let value = read_value(&mut reader, input, schema)?;
    let consumed = reader
        .contract_remaining()?
        .as_ptr()
        .addr()
        .checked_sub(input.as_ptr().addr())
        .ok_or(DerError::new(DerErrorReason::ArithmeticOverflow))?;
    if exact {
        reader.finish()?;
    }
    Ok(proto::CodecDerReadResult {
        value: Some(value).into(),
        consumed_bytes: u64::try_from(consumed)
            .map_err(|_| DerError::new(DerErrorReason::ArithmeticOverflow))?,
    })
}

fn span(source: &[u8], bytes: &[u8]) -> Result<proto::CodecByteSpan, DerError> {
    let overflow = || DerError::new(DerErrorReason::ArithmeticOverflow);
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
        offset: u64::try_from(offset).map_err(|_| overflow())?,
        length: u64::try_from(bytes.len()).map_err(|_| overflow())?,
    })
}

fn read_value(
    reader: &mut DerReader<'_>,
    source: &[u8],
    schema: &proto::CodecDerSchema,
) -> Result<proto::CodecDerValue, DerError> {
    use proto::CodecDerType as Kind;
    let invalid = || DerError::new(DerErrorReason::InvalidValue);
    let kind = schema.kind.as_known().ok_or_else(invalid)?;
    let start = reader.contract_remaining()?;
    let mut header_reader =
        SliceReader::new_with_encoding_rules(start, EncodingRules::Der).map_err(map_backend)?;
    let header = Header::decode(&mut header_reader).map_err(map_backend)?;
    let header_len = usize::try_from(header_reader.position())
        .map_err(|_| DerError::new(DerErrorReason::ArithmeticOverflow))?;
    let mut result = proto::CodecDerValue::default();
    result.kind = kind.into();
    match kind {
        Kind::CODEC_DER_TYPE_UNSIGNED_INTEGER => {
            result.bytes_value = copy_bytes(reader.read_unsigned_integer()?)?.into()
        }
        Kind::CODEC_DER_TYPE_INTEGER => {
            result.bytes_value = copy_bytes(reader.read_integer()?.as_bytes())?.into()
        }
        Kind::CODEC_DER_TYPE_BOOLEAN => result.bool_value = Some(reader.read_boolean()?),
        Kind::CODEC_DER_TYPE_OCTET_STRING => {
            result.bytes_value = copy_bytes(reader.read_octet_string()?)?.into()
        }
        Kind::CODEC_DER_TYPE_BIT_STRING => {
            let bits = reader.read_bit_string()?;
            result.unused_bits = Some(u32::from(bits.unused_bits()));
            result.bytes_value = copy_bytes(bits.as_bytes())?.into();
        }
        Kind::CODEC_DER_TYPE_NULL => reader.read_null()?,
        Kind::CODEC_DER_TYPE_OBJECT_IDENTIFIER => {
            result.bytes_value = copy_bytes(reader.read_object_identifier()?.as_bytes())?.into()
        }
        Kind::CODEC_DER_TYPE_UTF8_STRING => {
            result.bytes_value = copy_bytes(reader.read_utf8_string()?.as_bytes())?.into()
        }
        Kind::CODEC_DER_TYPE_SEQUENCE => {
            result.children =
                reserve_children(schema.children.len().min(reader.contract_element_limit()))?;
            reader.read_sequence(|nested| {
                for child in &schema.children {
                    result.children.push(read_value(nested, source, child)?);
                }
                Ok(())
            })?;
        }
        Kind::CODEC_DER_TYPE_SET_OF => {
            let [child_schema] = &schema.children[..] else {
                return Err(invalid());
            };
            // Every DER value occupies at least a tag and a length octet.
            // Reserve the bounded content-derived ceiling before storing any
            // scalar-bearing children, so growth cannot abandon stale values.
            let content_len = usize::try_from(header.length())
                .map_err(|_| DerError::new(DerErrorReason::ArithmeticOverflow))?;
            let capacity = (content_len / 2).min(reader.contract_element_limit());
            result.children = reserve_children(capacity)?;
            reader.read_set_of(|nested| {
                while !nested.is_finished() {
                    result
                        .children
                        .push(read_value(nested, source, child_schema)?);
                }
                Ok(())
            })?;
        }
        _ => return Err(invalid()),
    }
    let remaining = reader.contract_remaining()?;
    let length = remaining
        .as_ptr()
        .addr()
        .checked_sub(start.as_ptr().addr())
        .ok_or(DerError::new(DerErrorReason::ArithmeticOverflow))?;
    let received = start.get(..length).ok_or_else(invalid)?;
    let contents = received.get(header_len..).ok_or_else(invalid)?;
    result.received = Some(span(source, received)?).into();
    result.contents = Some(span(source, contents)?).into();
    Ok(result)
}

fn copy_bytes(bytes: &[u8]) -> Result<Vec<u8>, DerError> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len())
        .map_err(|_| DerError::new(DerErrorReason::AllocationFailure))?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}

// Preallocate the bounded empty collection before any sensitive child enters it.
fn reserve_children(
    capacity: usize,
) -> Result<buffa::sensitive::List<proto::CodecDerValue>, DerError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(capacity)
        .map_err(|_| DerError::new(DerErrorReason::AllocationFailure))?;
    Ok(values.into())
}
