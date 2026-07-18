// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#[test]
fn deterministic_cbor_encode_request_returns_canonical_bytes() {
    let request = CodecOperationRequest {
        operation: Some(
            CodecDeterministicCborEncodeRequest {
                value: buffa::MessageField::some(deterministic_cbor_map_fixture()),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };

    let result = result_payload(&process_operation_response(&request.encode_to_vec()));
    let encoded =
        decode_protobuf::<CodecDeterministicCborEncodeResult>(result.bytes()).unwrap();
    assert_eq!(
        encoded.encoded,
        vec![0xa3, 0x00, 0x64, b'z', b'e', b'r', b'o', 0x18, 0x18, 0xf6, 0x61, b'a', 0x01,]
    );

    let json = serde_json::to_vec(&request).unwrap();
    assert_eq!(
        process_operation_response(&request.encode_to_vec()),
        process_operation_response_json(&json)
    );
}

#[test]
fn deterministic_cbor_decode_request_returns_generated_value() {
    let request = CodecOperationRequest {
        operation: Some(
            CodecDeterministicCborDecodeRequest {
                encoded: vec![0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };

    let result = result_payload(&process_operation_response(&request.encode_to_vec()));
    let mut decoded =
        decode_protobuf::<CodecDeterministicCborDecodeResult>(result.bytes()).unwrap();
    let reencoded_request = CodecOperationRequest {
        operation: Some(
            CodecDeterministicCborEncodeRequest {
                value: core::mem::take(&mut decoded.value),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };
    let reencoded = result_payload(&process_operation_response(
        &reencoded_request.encode_to_vec(),
    ));
    let reencoded =
        decode_protobuf::<CodecDeterministicCborEncodeResult>(reencoded.bytes()).unwrap();
    assert_eq!(
        reencoded.encoded,
        vec![0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
    );
}

#[test]
fn deterministic_cbor_semantic_maximum_is_reachable_through_protobuf_lanes() {
    const CBOR_BYTE_STRING_U32_HEADER_LEN: usize = 5;

    let payload_len = MAX_DETERMINISTIC_CBOR_OUTPUT_LEN
        .checked_sub(CBOR_BYTE_STRING_U32_HEADER_LEN)
        .unwrap();
    let value = CodecDeterministicCborValue {
        value: Some(
            CodecDeterministicCborBytes {
                value: vec![0_u8; payload_len],
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };
    let encode_request = CodecOperationRequest {
        operation: Some(
            CodecDeterministicCborEncodeRequest {
                value: buffa::MessageField::some(value),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };

    let binary_request = encode_request.encode_to_vec();
    assert!(binary_request.len() <= codec_proto::MAX_CODEC_PROTO_MESSAGE_BYTES);
    let binary_envelope = process_operation_response(&binary_request);
    let encoded_result = result_payload(&binary_envelope);
    let encoded =
        decode_protobuf::<CodecDeterministicCborEncodeResult>(encoded_result.bytes()).unwrap();
    assert_eq!(encoded.encoded.len(), MAX_DETERMINISTIC_CBOR_OUTPUT_LEN);

    let json_request = serde_json::to_vec(&encode_request).unwrap();
    assert!(json_request.len() <= codec_proto::MAX_CODEC_PROTO_JSON_BYTES);
    assert_eq!(
        binary_envelope.as_slice(),
        process_operation_response_json(&json_request).as_slice()
    );

    let decode_request = CodecOperationRequest {
        operation: Some(
            CodecDeterministicCborDecodeRequest {
                // The generated encode result zeroizes its owned bytes on drop, so Rust
                // deliberately forbids moving the field out. Keep the test copy explicit;
                // the generated decode request owns and zeroizes this copy in turn.
                encoded: encoded.encoded.clone(),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };
    let decode_binary = decode_request.encode_to_vec();
    let decode_envelope = process_operation_response(&decode_binary);
    let decoded_result = result_payload(&decode_envelope);
    let decoded =
        decode_protobuf::<CodecDeterministicCborDecodeResult>(decoded_result.bytes()).unwrap();
    assert!(decoded.value.as_option().is_some());

    let decode_json = serde_json::to_vec(&decode_request).unwrap();
    assert_eq!(
        decode_envelope.as_slice(),
        process_operation_response_json(&decode_json).as_slice()
    );
}

#[test]
fn deterministic_cbor_allocation_failure_is_a_provider_failure() {
    let error = deterministic_cbor_wire_error(DeterministicCborError::AllocationFailure);
    assert_eq!(error.branch(), CodecWireErrorBranch::Backend);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_BACKEND_INTERNAL
    );
}

#[test]
fn deterministic_cbor_maximum_depth_matches_binary_and_proto_json_lanes() {
    let encode_request = CodecOperationRequest {
        operation: Some(
            CodecDeterministicCborEncodeRequest {
                value: buffa::MessageField::some(deterministic_cbor_nested_map_fixture(
                    MAX_DETERMINISTIC_CBOR_NESTING_DEPTH,
                )),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };

    let binary = process_operation_response(&encode_request.encode_to_vec());
    let json = serde_json::to_vec(&encode_request).unwrap();
    let from_json = process_operation_response_json(&json);

    let _ = result_payload(&binary);
    assert_eq!(binary.as_slice(), from_json.as_slice());

    // Exercise the inverse direction as well. A maximum-depth decoded
    // tree adds recursive generated response layers, so request-only
    // coverage cannot prove that SDKs can receive every semantically
    // valid value through the same transport limit.
    let encoded_result = result_payload(&binary);
    let mut encoded =
        decode_protobuf::<CodecDeterministicCborEncodeResult>(encoded_result.bytes()).unwrap();
    let decode_request = CodecOperationRequest {
        operation: Some(
            CodecDeterministicCborDecodeRequest {
                encoded: core::mem::take(&mut encoded.encoded),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };
    let decoded_binary = process_operation_response(&decode_request.encode_to_vec());
    let decoded_json = serde_json::to_vec(&decode_request).unwrap();
    let decoded_from_json = process_operation_response_json(&decoded_json);

    assert_eq!(decoded_binary.as_slice(), decoded_from_json.as_slice());
    let decoded_result = result_payload(&decoded_binary);
    let decoded =
        decode_protobuf::<CodecDeterministicCborDecodeResult>(decoded_result.bytes()).unwrap();
    assert!(decoded.value.as_option().is_some());
}

#[test]
fn deterministic_cbor_rejects_malformed_generated_value() {
    let request = CodecOperationRequest {
        operation: Some(
            CodecDeterministicCborEncodeRequest {
                value: buffa::MessageField::some(CodecDeterministicCborValue {
                    value: Some(
                        CodecDeterministicCborInteger {
                            value: Some(
                                CodecDeterministicCborNegativeInteger {
                                    value: 0,
                                    __buffa_unknown_fields: Default::default(),
                                }
                                .into(),
                            ),
                            __buffa_unknown_fields: Default::default(),
                        }
                        .into(),
                    ),
                    __buffa_unknown_fields: Default::default(),
                }),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };

    let error = codec_error_payload(&process_operation_response(&request.encode_to_vec()));
    assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_PROTOBUF
    );
}

#[test]
fn deterministic_cbor_rejects_nested_unknown_fields() {
    let mut value = CodecDeterministicCborValue {
        value: Some(
            CodecDeterministicCborText {
                value: "identity-value".to_owned(),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };
    value.__buffa_unknown_fields.push(UnknownField {
        number: 99,
        data: UnknownFieldData::Varint(7),
    });
    let request = CodecOperationRequest {
        operation: Some(
            CodecDeterministicCborEncodeRequest {
                value: buffa::MessageField::some(value),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };

    let error = codec_error_payload(&process_operation_response(&request.encode_to_vec()));
    assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_PROTOBUF
    );
}

struct TestOperationPayload {
    bytes: Zeroizing<Vec<u8>>,
}

impl TestOperationPayload {
    fn bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }
}
