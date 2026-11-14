// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{malformed_request_wire_error, shape_general_cbor as shape};
use codec_cbor::general as cbor;
use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::CodecWireError;
use proto::codec_api_operation_request::Operation;
use proto::codec_api_operation_result::Result as OperationResult;

pub(super) fn execute(operation: &mut Operation) -> Result<OperationResult, CodecWireError> {
    match operation {
        Operation::GeneralCborEncode(request) => {
            let (profile, limits) = options(request.options.as_option())?;
            let value = request
                .value
                .as_option()
                .ok_or_else(malformed_request_wire_error)?;
            let encoded = cbor::encode_value(value, profile, limits).map_err(map_error)?;
            Ok(OperationResult::GeneralCborEncode(Box::new(
                proto::CodecBytesResult {
                    value: super::try_copy_result_bytes(encoded.as_bytes())?.into(),
                },
            )))
        }
        Operation::GeneralCborFloatProperties(request) => {
            let value = request
                .value
                .as_option()
                .ok_or_else(malformed_request_wire_error)?;
            Ok(OperationResult::GeneralCborFloatProperties(Box::new(
                cbor::float_properties(value).map_err(map_error)?,
            )))
        }
        Operation::GeneralCborFloatEquivalent(request) => {
            let left = request
                .left
                .as_option()
                .ok_or_else(malformed_request_wire_error)?;
            let right = request
                .right
                .as_option()
                .ok_or_else(malformed_request_wire_error)?;
            Ok(OperationResult::GeneralCborFloatEquivalent(Box::new(
                proto::CodecBoolResult {
                    value: cbor::float_equivalent(left, right).map_err(map_error)?,
                },
            )))
        }
        Operation::GeneralCborInspect(request) => {
            let (profile, limits) = options(request.options.as_option())?;
            Ok(OperationResult::GeneralCborInspect(Box::new(
                super::inspect_general_cbor::inspect(request, profile, limits)?,
            )))
        }
        Operation::GeneralCborValidateEmbedded(request) => {
            let (outer_profile, outer_limits) = options(request.outer_options.as_option())?;
            let (profile, limits) = options(request.embedded_options.as_option())?;
            let outer = cbor::frame_exact(&request.encoded, outer_limits, outer_profile.lengths())
                .map_err(map_error)?;
            let tagged = outer.tagged_bytes().map_err(map_error)?;
            let item = tagged
                .validate_embedded(profile, limits)
                .map_err(map_error)?;
            Ok(OperationResult::GeneralCborValidateEmbedded(Box::new(
                shape::frame(&request.encoded, item.frame(), true, profile, limits)?,
            )))
        }
        Operation::GeneralCborReadHeader(request) => {
            let (_, limits) = source_options(request)?;
            if request.encoded.len() > limits.input_bytes() {
                return Err(map_error(cbor::CborError::new(
                    cbor::ErrorReason::InputLimit,
                )));
            }
            Ok(OperationResult::GeneralCborReadHeader(Box::new(
                shape::header(&cbor::read_header_prefix(&request.encoded).map_err(map_error)?)?,
            )))
        }
        Operation::GeneralCborFrameExact(request) => {
            let (profile, limits) = source_options(request)?;
            let frame = cbor::frame_exact(&request.encoded, limits, profile.lengths())
                .map_err(map_error)?;
            Ok(OperationResult::GeneralCborFrameExact(Box::new(
                shape::frame(&request.encoded, &frame, false, profile, limits)?,
            )))
        }
        Operation::GeneralCborFramePrefix(request) => {
            let (profile, limits) = source_options(request)?;
            let (frame, _) = cbor::frame_prefix(&request.encoded, limits, profile.lengths())
                .map_err(map_error)?;
            Ok(OperationResult::GeneralCborFramePrefix(Box::new(
                shape::frame(&request.encoded, &frame, false, profile, limits)?,
            )))
        }
        Operation::GeneralCborValidateExact(request) => {
            let (profile, limits) = source_options(request)?;
            let item =
                cbor::validate_exact(&request.encoded, profile, limits).map_err(map_error)?;
            Ok(OperationResult::GeneralCborValidateExact(Box::new(
                shape::frame(&request.encoded, item.frame(), true, profile, limits)?,
            )))
        }
        Operation::GeneralCborValidatePrefix(request) => {
            let (profile, limits) = source_options(request)?;
            let (item, _) =
                cbor::validate_prefix(&request.encoded, profile, limits).map_err(map_error)?;
            Ok(OperationResult::GeneralCborValidatePrefix(Box::new(
                shape::frame(&request.encoded, item.frame(), true, profile, limits)?,
            )))
        }
        Operation::GeneralCborCursor(request) => {
            let (profile, limits) = source_options(request)?;
            let capacity = shape::receipt_capacity(
                cbor::Cursor::new(&request.encoded, limits, profile.lengths())
                    .map_err(map_error)?,
                limits,
            )?;
            let mut cursor = cbor::Cursor::new(&request.encoded, limits, profile.lengths())
                .map_err(map_error)?;
            let mut result = proto::CodecGeneralCborFrames::default();
            result.values = shape::reserve_receipts(capacity)?;
            for frame in cursor.by_ref() {
                let frame = frame.map_err(map_error)?;
                result.values.push(shape::frame(
                    &request.encoded,
                    &frame,
                    false,
                    profile,
                    limits,
                )?);
            }
            result.consumed_bytes = shape::size(cursor.consumed_bytes().len())?;
            Ok(OperationResult::GeneralCborCursor(Box::new(result)))
        }
        _ => Err(malformed_request_wire_error()),
    }
}

fn source_options(
    value: &proto::CodecGeneralCborSourceRequest,
) -> Result<(cbor::Profile, cbor::Limits), CodecWireError> {
    options(value.options.as_option())
}

fn options(
    value: Option<&proto::CodecGeneralCborOptions>,
) -> Result<(cbor::Profile, cbor::Limits), CodecWireError> {
    match value {
        Some(value) => Ok((
            cbor::Profile::try_from(value).map_err(map_error)?,
            cbor::Limits::try_from(value).map_err(map_error)?,
        )),
        None => Ok((cbor::Profile::received(), cbor::Limits::default())),
    }
}

pub(super) fn map_error(error: cbor::CborError) -> CodecWireError {
    CodecWireError::from_general_cbor_reason(error.reason())
}
