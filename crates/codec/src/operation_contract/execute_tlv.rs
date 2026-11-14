// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::tlv;
use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::CodecWireError;
use proto::codec_api_operation_request::Operation;
use proto::codec_api_operation_result::Result as OperationResult;

fn options(request: &proto::CodecTlvSourceRequest) -> Result<&tlv::Options, CodecWireError> {
    request
        .options
        .as_option()
        .ok_or_else(|| CodecWireError::from_tlv_reason(tlv::ErrorReason::InvalidProfile))
}

pub(super) fn execute(operation: &mut Operation) -> Result<OperationResult, CodecWireError> {
    match operation {
        Operation::TlvReadHeader(request) => {
            let value = tlv::read_header(&request.encoded, options(request)?).map_err(map_error)?;
            Ok(OperationResult::TlvReadHeader(Box::new(
                value.into_contract(),
            )))
        }
        Operation::TlvFramePrefix(request) => {
            let (value, _remaining) =
                tlv::frame_prefix(&request.encoded, options(request)?).map_err(map_error)?;
            Ok(OperationResult::TlvFramePrefix(Box::new(
                value.into_contract(),
            )))
        }
        Operation::TlvFrameExact(request) => {
            let value = tlv::frame_exact(&request.encoded, options(request)?).map_err(map_error)?;
            Ok(OperationResult::TlvFrameExact(Box::new(
                value.into_contract(),
            )))
        }
        Operation::TlvReadIdentifier(request) => {
            let value =
                tlv::read_identifier(&request.encoded, options(request)?).map_err(map_error)?;
            Ok(OperationResult::TlvReadIdentifier(Box::new(
                value.into_contract(),
            )))
        }
        Operation::TlvReadLength(request) => {
            let value = tlv::read_length(&request.encoded, options(request)?).map_err(map_error)?;
            Ok(OperationResult::TlvReadLength(Box::new(
                value.into_contract(),
            )))
        }
        _ => Err(super::malformed_request_wire_error()),
    }
}

fn map_error(error: tlv::TlvError) -> CodecWireError {
    CodecWireError::from_tlv_reason(error.reason())
}
