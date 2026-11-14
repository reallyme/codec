// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::execute_general_cbor::map_error;
use super::shape_general_cbor as shape;
use codec_cbor::general as cbor;
use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::CodecWireError;
use proto::codec_general_cbor_inspection_result::Value;
use proto::CodecGeneralCborInspection as Inspection;

pub(super) fn inspect(
    request: &proto::CodecGeneralCborInspectRequest,
    profile: cbor::Profile,
    limits: cbor::Limits,
) -> Result<proto::CodecGeneralCborInspectionResult, CodecWireError> {
    let item;
    let framed;
    let frame = if request.validate {
        item = cbor::validate_exact(&request.encoded, profile, limits).map_err(map_error)?;
        item.frame()
    } else {
        framed =
            cbor::frame_exact(&request.encoded, limits, profile.lengths()).map_err(map_error)?;
        &framed
    };
    let value = match request
        .inspection
        .as_known()
        .ok_or_else(super::malformed_request_wire_error)?
    {
        Inspection::CODEC_GENERAL_CBOR_INSPECTION_RECEIVED_BYTES => bytes(frame.received_bytes())?,
        Inspection::CODEC_GENERAL_CBOR_INSPECTION_BODY_BYTES => bytes(shape::body(frame)?)?,
        Inspection::CODEC_GENERAL_CBOR_INSPECTION_BYTES => {
            bytes(frame.as_bytes().map_err(map_error)?)?
        }
        Inspection::CODEC_GENERAL_CBOR_INSPECTION_TEXT => {
            Value::TextValue(Box::new(proto::CodecTextResult {
                value: super::try_copy_result_string(frame.as_text().map_err(map_error)?)?.into(),
            }))
        }
        Inspection::CODEC_GENERAL_CBOR_INSPECTION_INTEGER => {
            let value = frame.integer().map_err(map_error)?;
            Value::IntegerValue(Box::new(proto::CodecGeneralCborInteger {
                negative: value.is_negative(),
                argument: value.argument(),
            }))
        }
        Inspection::CODEC_GENERAL_CBOR_INSPECTION_FLOAT => {
            let value = frame.float().map_err(map_error)?;
            Value::FloatValue(Box::new(proto::CodecGeneralCborFloat {
                width: value.width().into(),
                bits: value.bits(),
            }))
        }
        Inspection::CODEC_GENERAL_CBOR_INSPECTION_SIMPLE_VALUE => {
            Value::SimpleValue(Box::new(proto::CodecUint32Result {
                value: u32::from(frame.simple_value().map_err(map_error)?),
            }))
        }
        Inspection::CODEC_GENERAL_CBOR_INSPECTION_BOOL => {
            Value::BoolValue(Box::new(proto::CodecBoolResult {
                value: frame.as_bool().map_err(map_error)?,
            }))
        }
        Inspection::CODEC_GENERAL_CBOR_INSPECTION_TAGGED_BYTES => {
            let tagged = frame.tagged_bytes().map_err(map_error)?;
            Value::TaggedBytes(Box::new(proto::CodecGeneralCborTaggedBytes {
                tag: tagged.tag(),
                received: Some(shape::span(&request.encoded, tagged.received_bytes())?).into(),
                payload: Some(shape::span(&request.encoded, tagged.payload_bytes())?).into(),
            }))
        }
        Inspection::CODEC_GENERAL_CBOR_INSPECTION_STRING_CHUNKS => {
            let mut result = proto::CodecGeneralCborChunks::default();
            let capacity =
                shape::receipt_capacity(frame.string_chunks().map_err(map_error)?, limits)?;
            result.values = shape::reserve_receipts(capacity)?;
            for chunk in frame.string_chunks().map_err(map_error)? {
                result
                    .values
                    .push(shape::span(&request.encoded, chunk.map_err(map_error)?)?);
            }
            Value::Chunks(Box::new(result))
        }
        Inspection::CODEC_GENERAL_CBOR_INSPECTION_CHILDREN => {
            let mut result = proto::CodecGeneralCborFrames::default();
            let capacity = shape::receipt_capacity(frame.children().map_err(map_error)?, limits)?;
            result.values = shape::reserve_receipts(capacity)?;
            let mut cursor = frame.children().map_err(map_error)?;
            for child in cursor.by_ref() {
                let child = child.map_err(map_error)?;
                result.values.push(shape::frame(
                    &request.encoded,
                    &child,
                    request.validate,
                    profile,
                    limits,
                )?);
            }
            result.consumed_bytes = shape::size(cursor.consumed_bytes().len())?;
            Value::Children(Box::new(result))
        }
        _ => return Err(super::malformed_request_wire_error()),
    };
    Ok(proto::CodecGeneralCborInspectionResult { value: Some(value) })
}

fn bytes(value: &[u8]) -> Result<Value, CodecWireError> {
    Ok(Value::BytesValue(Box::new(proto::CodecBytesResult {
        value: super::try_copy_result_bytes(value)?.into(),
    })))
}
