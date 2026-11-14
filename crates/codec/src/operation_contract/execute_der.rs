// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::malformed_request_wire_error;
use crate::der::{self, DerError, DerLimits};
use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::CodecWireError;
use proto::codec_api_operation_request::Operation;
use proto::codec_api_operation_result::Result as OperationResult;

pub(super) fn execute(operation: &mut Operation) -> Result<OperationResult, CodecWireError> {
    match operation {
        Operation::DerDecode(request) => {
            let limits = limits(request.limits.as_option())?;
            let schema = request
                .schema
                .as_option()
                .ok_or_else(malformed_request_wire_error)?;
            let mut nodes = 0usize;
            validate_schema(schema, 0, &mut nodes)?;
            let mut result =
                der::decode_contract(&request.encoded, limits, schema, true).map_err(map_error)?;
            let value = result.value.take().ok_or_else(super::internal_wire_error)?;
            Ok(OperationResult::DerDecode(Box::new(value)))
        }
        Operation::DerRead(request) => {
            let limits = limits(request.limits.as_option())?;
            // Read is a prefix operation for primitive types. Container schemas
            // belong to DerDecode, where exact child consumption is explicit.
            let schema = proto::CodecDerSchema {
                kind: request.kind,
                children: Vec::new().into(),
            };
            let mut nodes = 0usize;
            validate_schema(&schema, 0, &mut nodes)?;
            if matches!(
                request.kind.as_known(),
                Some(
                    proto::CodecDerType::CODEC_DER_TYPE_SEQUENCE
                        | proto::CodecDerType::CODEC_DER_TYPE_SET_OF
                )
            ) {
                return Err(malformed_request_wire_error());
            }
            Ok(OperationResult::DerRead(Box::new(
                der::decode_contract(&request.encoded, limits, &schema, false)
                    .map_err(map_error)?,
            )))
        }
        Operation::DerDecodeUnsignedIntegerPair(request) => {
            let limits = limits(request.limits.as_option())?;
            let pair =
                der::decode_unsigned_integer_pair(&request.encoded, limits).map_err(map_error)?;
            Ok(OperationResult::DerDecodeUnsignedIntegerPair(Box::new(
                proto::CodecDerUnsignedIntegerPair {
                    first: super::try_copy_result_bytes(pair.first())?.into(),
                    second: super::try_copy_result_bytes(pair.second())?.into(),
                },
            )))
        }
        _ => Err(malformed_request_wire_error()),
    }
}

fn limits(value: Option<&proto::CodecDerLimits>) -> Result<DerLimits, CodecWireError> {
    let Some(value) = value else {
        return Ok(DerLimits::default());
    };

    DerLimits::try_from(value).map_err(map_error)
}

fn validate_schema(
    value: &proto::CodecDerSchema,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), CodecWireError> {
    *nodes = nodes
        .checked_add(1)
        .ok_or_else(malformed_request_wire_error)?;
    if depth > der::MAX_DER_NESTING_DEPTH || *nodes > der::MAX_DER_ELEMENTS {
        return Err(malformed_request_wire_error());
    }
    let kind = value
        .kind
        .as_known()
        .ok_or_else(malformed_request_wire_error)?;
    match kind {
        proto::CodecDerType::CODEC_DER_TYPE_UNSPECIFIED => {
            return Err(malformed_request_wire_error())
        }
        proto::CodecDerType::CODEC_DER_TYPE_SET_OF if value.children.len() != 1 => {
            return Err(malformed_request_wire_error())
        }
        proto::CodecDerType::CODEC_DER_TYPE_SEQUENCE
        | proto::CodecDerType::CODEC_DER_TYPE_SET_OF => {}
        _ if !value.children.is_empty() => return Err(malformed_request_wire_error()),
        _ => {}
    }
    let next = depth
        .checked_add(1)
        .ok_or_else(malformed_request_wire_error)?;
    for child in &value.children {
        validate_schema(child, next, nodes)?;
    }
    Ok(())
}

pub(super) fn map_error(error: DerError) -> CodecWireError {
    CodecWireError::from_der_reason(error.reason())
}
