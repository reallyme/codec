// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Executes a generated binary protobuf request and returns a fully
/// discriminated binary response contract.
#[must_use]
pub fn process_operation_response(request_bytes: &[u8]) -> Zeroizing<Vec<u8>> {
    encode_api_operation_response(&api_response_from_result(
        preflight_binary_request(request_bytes)
            .and_then(|()| decode_protobuf::<ApiRequest>(request_bytes))
            .and_then(execute_api_request),
    ))
}

/// Executes a generated ProtoJSON request and returns the same fully
/// discriminated binary response contract as [`process_operation_response`].
#[must_use]
pub fn process_operation_response_json(request_json: &[u8]) -> Zeroizing<Vec<u8>> {
    encode_api_operation_response(&api_response_from_result(
        decode_json::<ApiRequest>(request_json).and_then(execute_api_request),
    ))
}

/// Return the stable caller-attributed resource-limit envelope when a boundary
/// has rejected an oversized request before copying its bytes.
#[must_use]
pub fn resource_limit_operation_response() -> Zeroizing<Vec<u8>> {
    encode_operation_response(&operation_response_from_result(Err(wire_error(
        CodecWireErrorBranch::Boundary,
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
    ))))
}

// The fixed response remains available if encoding a generated response ever
// exceeds Buffa's size limit. It carries BACKEND_INTERNAL with PROVIDER origin.
// Keeping this envelope independent of Buffa avoids a second encoding failure
// while preserving the discriminated operation response contract.
const INTERNAL_ERROR_OPERATION_RESPONSE: [u8; 10] =
    [0x12, 0x08, 0x2a, 0x03, 0x08, 0xf4, 0x03, 0xa0, 0x06, 0x02];

fn encode_operation_response(response: &CodecOperationResponse) -> Zeroizing<Vec<u8>> {
    match encode_protobuf(response) {
        Ok(bytes) => {
            if matches!(
                response.outcome.as_ref(),
                Some(codec_proto::generated::proto::reallyme::codec::v1::codec_operation_response::Outcome::Error(_))
            ) && bytes.len() > MAX_CODEC_PROTO_ERROR_ENVELOPE_BYTES
            {
                return Zeroizing::new(INTERNAL_ERROR_OPERATION_RESPONSE.to_vec());
            }
            bytes
        }
        Err(_) => Zeroizing::new(INTERNAL_ERROR_OPERATION_RESPONSE.to_vec()),
    }
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

    }
}

fn process_operation_request(
    mut request: CodecOperationRequest,
) -> Result<CodecOperationResult, CodecWireError> {

    let Some(mut operation) = request.operation.take() else {
        return Err(wire_error(
            CodecWireErrorBranch::Boundary,
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MISSING_OPERATION,
        ));
    };

    match &mut operation {
        CodecOperation::MulticodecPrefixForName(request) => {

            Ok(operation_result(process_multicodec_prefix_for_name(
                &request.name,
            )?))
        }
        CodecOperation::MulticodecLookupPrefix(request) => {

            Ok(operation_result(process_multicodec_lookup_prefix(
                &request.value,
            )?))
        }
        CodecOperation::MulticodecTable(_) => {

            Ok(operation_result(process_multicodec_table()?))
        }
        CodecOperation::MultikeyParse(request) => {

            Ok(operation_result(process_multikey_parse(&request.multikey)?))
        }
        CodecOperation::DagCborVerifyCid(request) => {

            Ok(operation_result(process_dag_cbor_verify_cid(
                &request.cid,
                &request.payload,
            )?))
        }
        CodecOperation::DagCborEncode(request) => {

            Ok(operation_result(process_dag_cbor_encode(&request.value)?))
        }
        CodecOperation::DagCborDecode(request) => {

            Ok(operation_result(process_dag_cbor_decode(&request.encoded)?))
        }
        CodecOperation::PemDecode(request) => {

                Ok(operation_result(process_pem_decode(
                &request.pem,
                request.options.as_option(),
            )?))
        }
        CodecOperation::PemEncode(request) => {

                Ok(operation_result(process_pem_encode(
                request.label,
                &request.der,
                request.options.as_option(),
            )?))
        }
        CodecOperation::DeterministicCborEncode(request) => {

            Ok(operation_result(process_deterministic_cbor_encode(
                &request.value,
            )?))
        }
        CodecOperation::DeterministicCborDecode(request) => {

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

    }
}
