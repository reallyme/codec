// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{execute_der, execute_general_cbor, execute_scalars, execute_tlv};
use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::{codec_api_error, CodecWireError};
use proto::codec_api_operation_request::Operation;
use proto::codec_api_operation_result::Result as ApiResult;
use proto::CodecApiOperationRequest as ApiRequest;
use proto::CodecApiOperationResponse as ApiResponse;
use proto::CodecApiOperationResult;

pub(super) fn execute_api_request(
    mut request: ApiRequest,
) -> Result<CodecApiOperationResult, CodecWireError> {
    let mut operation = request.operation.take().ok_or_else(|| {
        super::wire_error(
            codec_proto::CodecWireErrorBranch::Boundary,
            proto::CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MISSING_OPERATION,
        )
    })?;
    let result = match &mut operation {
        Operation::MulticodecPrefixForName(request) => {
            return legacy_request(
                proto::codec_operation_request::Operation::MulticodecPrefixForName(Box::new(
                    core::mem::take(request.as_mut()),
                )),
            );
        }
        Operation::MulticodecLookupPrefix(request) => {
            return legacy_request(
                proto::codec_operation_request::Operation::MulticodecLookupPrefix(Box::new(
                    core::mem::take(request.as_mut()),
                )),
            );
        }
        Operation::MulticodecTable(request) => {
            return legacy_request(proto::codec_operation_request::Operation::MulticodecTable(
                Box::new(core::mem::take(request.as_mut())),
            ));
        }
        Operation::MultikeyParse(request) => {
            return legacy_request(proto::codec_operation_request::Operation::MultikeyParse(
                Box::new(core::mem::take(request.as_mut())),
            ));
        }
        Operation::DagCborVerifyCid(request) => {
            return legacy_request(proto::codec_operation_request::Operation::DagCborVerifyCid(
                Box::new(core::mem::take(request.as_mut())),
            ));
        }
        Operation::DagCborEncode(request) => {
            return legacy_request(proto::codec_operation_request::Operation::DagCborEncode(
                Box::new(core::mem::take(request.as_mut())),
            ));
        }
        Operation::DagCborDecode(request) => {
            return legacy_request(proto::codec_operation_request::Operation::DagCborDecode(
                Box::new(core::mem::take(request.as_mut())),
            ));
        }
        Operation::PemDecode(request) => {
            return legacy_request(proto::codec_operation_request::Operation::PemDecode(
                Box::new(core::mem::take(request.as_mut())),
            ));
        }
        Operation::PemEncode(request) => {
            return legacy_request(proto::codec_operation_request::Operation::PemEncode(
                Box::new(core::mem::take(request.as_mut())),
            ));
        }
        Operation::DeterministicCborEncode(request) => {
            return legacy_request(
                proto::codec_operation_request::Operation::DeterministicCborEncode(Box::new(
                    core::mem::take(request.as_mut()),
                )),
            );
        }
        Operation::DeterministicCborDecode(request) => {
            return legacy_request(
                proto::codec_operation_request::Operation::DeterministicCborDecode(Box::new(
                    core::mem::take(request.as_mut()),
                )),
            );
        }
        operation => match operation {
            Operation::GeneralCborReadHeader(_)
            | Operation::GeneralCborFrameExact(_)
            | Operation::GeneralCborFramePrefix(_)
            | Operation::GeneralCborValidateExact(_)
            | Operation::GeneralCborValidatePrefix(_)
            | Operation::GeneralCborEncode(_)
            | Operation::GeneralCborInspect(_)
            | Operation::GeneralCborCursor(_)
            | Operation::GeneralCborValidateEmbedded(_)
            | Operation::GeneralCborFloatProperties(_)
            | Operation::GeneralCborFloatEquivalent(_) => execute_general_cbor::execute(operation)?,
            Operation::DerDecode(_)
            | Operation::DerRead(_)
            | Operation::DerDecodeUnsignedIntegerPair(_) => execute_der::execute(operation)?,
            Operation::TlvReadHeader(_)
            | Operation::TlvFramePrefix(_)
            | Operation::TlvFrameExact(_)
            | Operation::TlvReadIdentifier(_)
            | Operation::TlvReadLength(_) => execute_tlv::execute(operation)?,
            operation => execute_scalars::execute(operation)?,
        },
    };
    Ok(CodecApiOperationResult {
        result: Some(result),
    })
}

fn legacy_request(
    operation: proto::codec_operation_request::Operation,
) -> Result<CodecApiOperationResult, CodecWireError> {
    let request = proto::CodecOperationRequest {
        operation: Some(operation),
    };
    let mut result = super::process_operation_request(request)?;
    // Move generated payload owners directly: compatibility never re-encodes or
    // clones sensitive results merely to change the envelope's Rust type.
    let result = match result
        .result
        .as_mut()
        .ok_or_else(super::internal_wire_error)?
    {
        proto::codec_operation_result::Result::MulticodecPrefixForName(value) => {
            ApiResult::MulticodecPrefixForName(Box::new(core::mem::take(value.as_mut())))
        }
        proto::codec_operation_result::Result::MulticodecLookupPrefix(value) => {
            ApiResult::MulticodecLookupPrefix(Box::new(core::mem::take(value.as_mut())))
        }
        proto::codec_operation_result::Result::MulticodecTable(value) => {
            ApiResult::MulticodecTable(Box::new(core::mem::take(value.as_mut())))
        }
        proto::codec_operation_result::Result::MultikeyParse(value) => {
            ApiResult::MultikeyParse(Box::new(core::mem::take(value.as_mut())))
        }
        proto::codec_operation_result::Result::DagCborVerifyCid(value) => {
            ApiResult::DagCborVerifyCid(Box::new(core::mem::take(value.as_mut())))
        }
        proto::codec_operation_result::Result::DagCborEncode(value) => {
            ApiResult::DagCborEncode(Box::new(core::mem::take(value.as_mut())))
        }
        proto::codec_operation_result::Result::DagCborDecode(value) => {
            ApiResult::DagCborDecode(Box::new(core::mem::take(value.as_mut())))
        }
        proto::codec_operation_result::Result::PemDecode(value) => {
            ApiResult::PemDecode(Box::new(core::mem::take(value.as_mut())))
        }
        proto::codec_operation_result::Result::PemEncode(value) => {
            ApiResult::PemEncode(Box::new(core::mem::take(value.as_mut())))
        }
        proto::codec_operation_result::Result::DeterministicCborEncode(value) => {
            ApiResult::DeterministicCborEncode(Box::new(core::mem::take(value.as_mut())))
        }
        proto::codec_operation_result::Result::DeterministicCborDecode(value) => {
            ApiResult::DeterministicCborDecode(Box::new(core::mem::take(value.as_mut())))
        }
    };
    Ok(CodecApiOperationResult {
        result: Some(result),
    })
}

pub(super) fn api_response_from_result(
    result: Result<CodecApiOperationResult, CodecWireError>,
) -> ApiResponse {
    let outcome = match result {
        Ok(result) => result.into(),
        Err(error) => codec_api_error(error).into(),
    };
    ApiResponse {
        outcome: Some(outcome),
    }
}

pub(super) fn encode_api_operation_response(response: &ApiResponse) -> zeroize::Zeroizing<Vec<u8>> {
    match codec_proto::encode_protobuf(response) {
        Ok(bytes) => {
            if matches!(
                response.outcome.as_ref(),
                Some(proto::codec_api_operation_response::Outcome::Error(_))
            ) && bytes.len() > codec_proto::MAX_CODEC_PROTO_ERROR_ENVELOPE_BYTES
            {
                return zeroize::Zeroizing::new(super::INTERNAL_ERROR_OPERATION_RESPONSE.to_vec());
            }
            bytes
        }
        Err(_) => zeroize::Zeroizing::new(super::INTERNAL_ERROR_OPERATION_RESPONSE.to_vec()),
    }
}
