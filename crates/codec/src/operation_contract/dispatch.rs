// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

/// Executes a generated binary protobuf request and returns a fully
/// discriminated binary response contract.
#[must_use]
pub fn process_operation_response(request_bytes: &[u8]) -> Zeroizing<Vec<u8>> {
    encode_protobuf(&operation_response_from_result(
        decode_protobuf::<CodecOperationRequest>(request_bytes).and_then(process_operation_request),
    ))
}

/// Executes a generated ProtoJSON request and returns the same fully
/// discriminated binary response contract as [`process_operation_response`].
#[must_use]
pub fn process_operation_response_json(request_json: &[u8]) -> Zeroizing<Vec<u8>> {
    encode_protobuf(&operation_response_from_result(
        decode_json::<CodecOperationRequest>(request_json).and_then(process_operation_request),
    ))
}

fn operation_response_from_result(
    result: Result<CodecOperationResult, CodecWireError>,
) -> CodecOperationResponse {
    let outcome = match result {
        Ok(result) => result.into(),
        Err(error) => codec_error(error).into(),
    };
    CodecOperationResponse {
        outcome: Some(outcome),
        __buffa_unknown_fields: Default::default(),
    }
}

fn process_operation_request(
    mut request: CodecOperationRequest,
) -> Result<CodecOperationResult, CodecWireError> {
    reject_unknown_fields(&request.__buffa_unknown_fields)?;
    let Some(operation) = request.operation.take() else {
        return Err(wire_error(
            CodecWireErrorBranch::Boundary,
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MISSING_OPERATION,
        ));
    };

    match operation {
        CodecOperation::MulticodecPrefixForName(request) => {
            reject_unknown_fields(&request.__buffa_unknown_fields)?;
            Ok(operation_result(process_multicodec_prefix_for_name(
                &request.name,
            )?))
        }
        CodecOperation::MulticodecLookupPrefix(request) => {
            reject_unknown_fields(&request.__buffa_unknown_fields)?;
            Ok(operation_result(process_multicodec_lookup_prefix(
                &request.value,
            )?))
        }
        CodecOperation::MulticodecTable(request) => {
            reject_unknown_fields(&request.__buffa_unknown_fields)?;
            Ok(operation_result(process_multicodec_table()?))
        }
        CodecOperation::MultikeyParse(request) => {
            reject_unknown_fields(&request.__buffa_unknown_fields)?;
            Ok(operation_result(process_multikey_parse(&request.multikey)?))
        }
        CodecOperation::DagCborVerifyCid(request) => {
            reject_unknown_fields(&request.__buffa_unknown_fields)?;
            Ok(operation_result(process_dag_cbor_verify_cid(
                &request.cid,
                &request.payload,
            )?))
        }
        CodecOperation::DagCborEncode(request) => {
            reject_unknown_fields(&request.__buffa_unknown_fields)?;
            Ok(operation_result(process_dag_cbor_encode(&request.value)?))
        }
        CodecOperation::DagCborDecode(request) => {
            reject_unknown_fields(&request.__buffa_unknown_fields)?;
            Ok(operation_result(process_dag_cbor_decode(&request.encoded)?))
        }
        CodecOperation::PemDecode(request) => {
            reject_unknown_fields(&request.__buffa_unknown_fields)?;
            if let Some(options) = request.options.as_option() {
                reject_unknown_fields(&options.__buffa_unknown_fields)?;
            }
            Ok(operation_result(process_pem_decode(
                &request.pem,
                request.options.as_option(),
            )?))
        }
        CodecOperation::PemEncode(request) => {
            reject_unknown_fields(&request.__buffa_unknown_fields)?;
            if let Some(options) = request.options.as_option() {
                reject_unknown_fields(&options.__buffa_unknown_fields)?;
            }
            Ok(operation_result(process_pem_encode(
                request.label.as_known(),
                &request.der,
                request.options.as_option(),
            )?))
        }
        CodecOperation::DeterministicCborEncode(request) => {
            reject_unknown_fields(&request.__buffa_unknown_fields)?;
            Ok(operation_result(process_deterministic_cbor_encode(
                &request.value,
            )?))
        }
        CodecOperation::DeterministicCborDecode(request) => {
            reject_unknown_fields(&request.__buffa_unknown_fields)?;
            Ok(operation_result(process_deterministic_cbor_decode(
                &request.encoded,
            )?))
        }
    }
}

fn operation_result<T>(result: T) -> CodecOperationResult
where
    T: Into<codec_proto::generated::proto::reallyme::codec::v1::codec_operation_result::Result>,
{
    CodecOperationResult {
        result: Some(result.into()),
        __buffa_unknown_fields: Default::default(),
    }
}

fn reject_unknown_fields(fields: &buffa::UnknownFields) -> Result<(), CodecWireError> {
    if fields.is_empty() {
        Ok(())
    } else {
        Err(malformed_request_wire_error())
    }
}
