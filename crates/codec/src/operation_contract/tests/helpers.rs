// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

fn result_payload(response_bytes: &[u8]) -> TestOperationPayload {
    let mut response = decode_protobuf::<CodecOperationResponse>(response_bytes).unwrap();
    let outcome = response.outcome.take();
    assert!(
        matches!(outcome, Some(codec_operation_response::Outcome::Result(_))),
        "expected generated operation result"
    );
    let Some(codec_operation_response::Outcome::Result(mut result)) = outcome else {
        return TestOperationPayload {
            bytes: Zeroizing::new(Vec::new()),
        };
    };
    let bytes = match result.result.take().unwrap() {
        codec_operation_result::Result::MulticodecPrefixForName(value) => {
            encode_protobuf(value.as_ref())
        }
        codec_operation_result::Result::MulticodecLookupPrefix(value) => {
            encode_protobuf(value.as_ref())
        }
        codec_operation_result::Result::MulticodecTable(value) => {
            encode_protobuf(value.as_ref())
        }
        codec_operation_result::Result::MultikeyParse(value) => encode_protobuf(value.as_ref()),
        codec_operation_result::Result::DagCborVerifyCid(value) => {
            encode_protobuf(value.as_ref())
        }
        codec_operation_result::Result::DagCborEncode(value) => encode_protobuf(value.as_ref()),
        codec_operation_result::Result::DagCborDecode(value) => encode_protobuf(value.as_ref()),
        codec_operation_result::Result::PemDecode(value) => encode_protobuf(value.as_ref()),
        codec_operation_result::Result::PemEncode(value) => encode_protobuf(value.as_ref()),
        codec_operation_result::Result::DeterministicCborEncode(value) => {
            encode_protobuf(value.as_ref())
        }
        codec_operation_result::Result::DeterministicCborDecode(value) => {
            encode_protobuf(value.as_ref())
        }
    };
    TestOperationPayload { bytes }
}

fn codec_error_payload(response_bytes: &[u8]) -> CodecWireError {
    let mut response = decode_protobuf::<CodecOperationResponse>(response_bytes).unwrap();
    let outcome = response.outcome.take();
    assert!(
        matches!(outcome, Some(codec_operation_response::Outcome::Error(_))),
        "expected generated operation error"
    );
    let Some(codec_operation_response::Outcome::Error(mut error)) = outcome else {
        return CodecWireError::malformed_protobuf();
    };
    let (branch, reason) = match error.error.take().unwrap() {
        codec_error::Error::BaseEncoding(value) => {
            (CodecWireErrorBranch::BaseEncoding, value.reason)
        }
        codec_error::Error::Pem(value) => (CodecWireErrorBranch::Pem, value.reason),
        codec_error::Error::Multiformat(value) => {
            (CodecWireErrorBranch::Multiformat, value.reason)
        }
        codec_error::Error::Canonicalization(value) => {
            (CodecWireErrorBranch::Canonicalization, value.reason)
        }
        codec_error::Error::Backend(value) => (CodecWireErrorBranch::Backend, value.reason),
        codec_error::Error::Boundary(value) => (CodecWireErrorBranch::Boundary, value.reason),
    };
    let wire_error = CodecWireError::try_new(branch, reason.as_known().unwrap()).unwrap();
    let expected_origin = match wire_error.origin() {
        CodecWireErrorOrigin::Caller => CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER,
        CodecWireErrorOrigin::Provider => CodecErrorOrigin::CODEC_ERROR_ORIGIN_PROVIDER,
        unexpected => {
            assert_eq!(unexpected, CodecWireErrorOrigin::Caller);
            CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER
        }
    };
    assert_eq!(error.origin.as_known(), Some(expected_origin));
    wire_error
}

fn process_binary_and_proto_json(request: &CodecOperationRequest) -> Zeroizing<Vec<u8>> {
    let binary = process_operation_response(&request.encode_to_vec());
    let json = serde_json::to_vec(request).unwrap();
    let from_json = process_operation_response_json(&json);
    assert_eq!(binary.as_slice(), from_json.as_slice());
    binary
}

fn multikey_parse_request(multikey: &str) -> CodecOperationRequest {
    CodecOperationRequest {
        operation: Some(
            CodecMultikeyParseRequest {
                multikey: multikey.to_owned(),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    }
}

fn dag_cbor_verify_cid_request(cid: &str, payload: &[u8]) -> CodecOperationRequest {
    CodecOperationRequest {
        operation: Some(
            CodecDagCborVerifyCidRequest {
                cid: cid.to_owned(),
                payload: payload.to_vec(),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    }
}

fn pem_decode_request(
    pem: &[u8],
    options: Option<CodecPemDecodeOptions>,
) -> CodecOperationRequest {
    CodecOperationRequest {
        operation: Some(
            CodecPemDecodeRequest {
                pem: pem.to_vec(),
                options: match options {
                    Some(options) => buffa::MessageField::some(options),
                    None => buffa::MessageField::none(),
                },
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    }
}

fn deterministic_cbor_map_fixture() -> CodecDeterministicCborValue {
    CodecDeterministicCborValue {
        value: Some(
            CodecDeterministicCborMap {
                entries: vec![
                    CodecDeterministicCborMapEntry {
                        key: buffa::MessageField::some(CodecDeterministicCborMapKey {
                            key: Some(
                                CodecDeterministicCborText {
                                    value: "a".to_owned(),
                                    __buffa_unknown_fields: Default::default(),
                                }
                                .into(),
                            ),
                            __buffa_unknown_fields: Default::default(),
                        }),
                        value: buffa::MessageField::some(CodecDeterministicCborValue {
                            value: Some(
                                CodecDeterministicCborInteger {
                                    value: Some(
                                        CodecDeterministicCborUnsignedInteger {
                                            value: 1,
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
                    },
                    CodecDeterministicCborMapEntry {
                        key: buffa::MessageField::some(CodecDeterministicCborMapKey {
                            key: Some(
                                CodecDeterministicCborInteger {
                                    value: Some(
                                        CodecDeterministicCborUnsignedInteger {
                                            value: 24,
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
                        value: buffa::MessageField::some(CodecDeterministicCborValue {
                            value: Some(
                                CodecDeterministicCborNull {
                                    __buffa_unknown_fields: Default::default(),
                                }
                                .into(),
                            ),
                            __buffa_unknown_fields: Default::default(),
                        }),
                        __buffa_unknown_fields: Default::default(),
                    },
                    CodecDeterministicCborMapEntry {
                        key: buffa::MessageField::some(CodecDeterministicCborMapKey {
                            key: Some(
                                CodecDeterministicCborInteger {
                                    value: Some(
                                        CodecDeterministicCborUnsignedInteger {
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
                        value: buffa::MessageField::some(CodecDeterministicCborValue {
                            value: Some(
                                CodecDeterministicCborText {
                                    value: "zero".to_owned(),
                                    __buffa_unknown_fields: Default::default(),
                                }
                                .into(),
                            ),
                            __buffa_unknown_fields: Default::default(),
                        }),
                        __buffa_unknown_fields: Default::default(),
                    },
                ],
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    }
}

fn dag_cbor_proto_fixture() -> CodecDeterministicCborValue {
    CodecDeterministicCborValue {
        value: Some(
            CodecDeterministicCborMap {
                entries: vec![
                    CodecDeterministicCborMapEntry {
                        key: buffa::MessageField::some(CodecDeterministicCborMapKey {
                            key: Some(
                                CodecDeterministicCborText {
                                    value: "z".to_owned(),
                                    __buffa_unknown_fields: Default::default(),
                                }
                                .into(),
                            ),
                            __buffa_unknown_fields: Default::default(),
                        }),
                        value: buffa::MessageField::some(CodecDeterministicCborValue {
                            value: Some(
                                CodecDeterministicCborInteger {
                                    value: Some(
                                        CodecDeterministicCborNegativeInteger {
                                            value: -1,
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
                    },
                    CodecDeterministicCborMapEntry {
                        key: buffa::MessageField::some(CodecDeterministicCborMapKey {
                            key: Some(
                                CodecDeterministicCborText {
                                    value: "a".to_owned(),
                                    __buffa_unknown_fields: Default::default(),
                                }
                                .into(),
                            ),
                            __buffa_unknown_fields: Default::default(),
                        }),
                        value: buffa::MessageField::some(CodecDeterministicCborValue {
                            value: Some(
                                CodecDeterministicCborBytes {
                                    value: vec![1, 2, 3],
                                    __buffa_unknown_fields: Default::default(),
                                }
                                .into(),
                            ),
                            __buffa_unknown_fields: Default::default(),
                        }),
                        __buffa_unknown_fields: Default::default(),
                    },
                ],
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    }
}

fn deterministic_cbor_nested_map_fixture(depth: usize) -> CodecDeterministicCborValue {
    let mut value = CodecDeterministicCborValue {
        value: Some(
            CodecDeterministicCborNull {
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };
    for _ in 0..depth {
        value = CodecDeterministicCborValue {
            value: Some(
                CodecDeterministicCborMap {
                    entries: vec![CodecDeterministicCborMapEntry {
                        key: buffa::MessageField::some(CodecDeterministicCborMapKey {
                            key: Some(
                                CodecDeterministicCborInteger {
                                    value: Some(
                                        CodecDeterministicCborUnsignedInteger {
                                            value: 1,
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
                        value: buffa::MessageField::some(value),
                        __buffa_unknown_fields: Default::default(),
                    }],
                    __buffa_unknown_fields: Default::default(),
                }
                .into(),
            ),
            __buffa_unknown_fields: Default::default(),
        };
    }
    value
}
