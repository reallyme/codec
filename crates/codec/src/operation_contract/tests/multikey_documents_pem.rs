// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#[test]
fn multikey_parse_dispatch_matches_primitive_parser() {
    let public_key = [11_u8; 32];
    let multikey = codec_multikey::encode_multikey("ed25519-pub", &public_key).unwrap();
    let primitive = codec_multikey::parse_multikey(&multikey).unwrap();
    let request = multikey_parse_request(&multikey);

    let binary = process_binary_and_proto_json(&request);

    let result = result_payload(&binary);
    let parsed = decode_protobuf::<CodecMultikeyParseResult>(result.bytes()).unwrap();
    assert_eq!(parsed.codec_name, primitive.codec_name);
    assert_eq!(parsed.algorithm_name, primitive.alg);
    assert_eq!(parsed.public_key, primitive.public_key.as_slice());
    assert_eq!(
        parsed.expected_public_key_length,
        u32::try_from(primitive.key_length).unwrap()
    );
    assert!(!parsed.variable_public_key_length);
}

#[test]
fn multikey_parse_dispatch_preserves_variable_length_flag() {
    let public_key = [13_u8; 80];
    let multikey = codec_multikey::encode_multikey("rsa-pub", &public_key).unwrap();
    let request = multikey_parse_request(&multikey);
    let envelope = process_binary_and_proto_json(&request);
    let result = result_payload(&envelope);
    let parsed = decode_protobuf::<CodecMultikeyParseResult>(result.bytes()).unwrap();

    assert_eq!(parsed.codec_name, "rsa-pub");
    assert_eq!(parsed.algorithm_name, "RSA");
    assert_eq!(parsed.public_key, public_key.as_slice());
    assert_eq!(parsed.expected_public_key_length, 0);
    assert!(parsed.variable_public_key_length);
}

#[test]
fn multikey_parse_dispatch_rejects_noncanonical_multibase() {
    let request = multikey_parse_request("not-a-key");
    let envelope = process_binary_and_proto_json(&request);
    let error = codec_error_payload(&envelope);

    assert_eq!(error.branch(), CodecWireErrorBranch::Multiformat);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTIKEY
    );
}

#[test]
fn multikey_parse_dispatch_rejects_unknown_prefix() {
    let multikey = codec_multibase::bytes_to_multibase58btc(&[0, 0, 7]).unwrap();
    let request = multikey_parse_request(&multikey);
    let envelope = process_binary_and_proto_json(&request);
    let error = codec_error_payload(&envelope);

    assert_eq!(error.branch(), CodecWireErrorBranch::Multiformat);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC
    );
}

#[test]
fn dag_cbor_verify_cid_dispatch_matches_primitive_verifier() {
    let payload = vec![0xa0];
    let cid = codec_cbor::compute_cid_dag_cbor(&payload);
    let primitive = codec_cbor::verify_dag_cbor_cid(&cid, &payload);
    let request = dag_cbor_verify_cid_request(&cid, &payload);

    let binary = process_binary_and_proto_json(&request);

    let result = result_payload(&binary);
    let verified = decode_protobuf::<CodecDagCborVerifyCidResult>(result.bytes()).unwrap();
    assert_eq!(verified.valid, primitive.0);
    assert_eq!(verified.expected_cid, primitive.1);
    assert_eq!(verified.actual_cid, primitive.2);
}

#[test]
fn dag_cbor_generated_result_takes_semantic_string_ownership() {
    let payload = [0xa0];
    let cid = codec_cbor::compute_cid_dag_cbor(&payload);
    let verification = verify_dag_cbor_cid(&cid, &payload).unwrap();
    let expected_cid_allocation = verification.expected_cid().as_ptr();
    let actual_cid_allocation = verification.actual_cid().as_ptr();

    let result = dag_cbor_verify_cid_result_proto(verification);

    // The adapter must transfer both owned strings into the generated
    // owner. Reallocating here would create avoidable identity-bearing
    // copies and weaken the ownership model at every scalar boundary.
    assert_eq!(result.expected_cid.as_ptr(), expected_cid_allocation);
    assert_eq!(result.actual_cid.as_ptr(), actual_cid_allocation);
}

#[test]
fn dag_cbor_verify_cid_dispatch_preserves_invalid_cid_sanitization() {
    let payload = vec![0xa0];
    let request = dag_cbor_verify_cid_request("not-a-cid", &payload);
    let envelope = process_binary_and_proto_json(&request);
    let result = result_payload(&envelope);
    let verified = decode_protobuf::<CodecDagCborVerifyCidResult>(result.bytes()).unwrap();

    assert!(!verified.valid);
    assert_eq!(
        verified.expected_cid,
        codec_cbor::compute_cid_dag_cbor(&payload)
    );
    assert_eq!(verified.actual_cid, "");
}

#[test]
fn dag_cbor_verify_cid_dispatch_rejects_oversized_payload() {
    let payload = vec![0_u8; codec_cbor::MAX_DAG_CBOR_INPUT_LEN + 1];
    let request = dag_cbor_verify_cid_request("", &payload);
    let envelope = process_binary_and_proto_json(&request);
    let error = codec_error_payload(&envelope);

    assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED
    );
}

#[test]
fn dag_cbor_encode_request_returns_primitive_canonical_bytes() {
    let semantic = CborValue::Map(vec![
        ("z".to_owned(), CborValue::Int(-1)),
        ("a".to_owned(), CborValue::Bytes(vec![1, 2, 3])),
    ]);
    let request = CodecOperationRequest {
        operation: Some(
            codec_proto::generated::proto::reallyme::codec::v1::CodecDagCborEncodeRequest {
                value: buffa::MessageField::some(dag_cbor_proto_fixture()),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };

    let result = result_payload(&process_operation_response(&request.encode_to_vec()));
    let encoded = decode_protobuf::<CodecDagCborEncodeResult>(result.bytes()).unwrap();
    let expected = encode_dag_cbor_value(&semantic).unwrap();
    assert_eq!(encoded.encoded.as_slice(), expected.as_slice());

    let json = serde_json::to_vec(&request).unwrap();
    assert_eq!(
        process_operation_response(&request.encode_to_vec()),
        process_operation_response_json(&json)
    );
}

#[test]
fn dag_cbor_decode_request_returns_typed_generated_value() {
    let encoded = encode_dag_cbor_value(&CborValue::Map(vec![(
        "a".to_owned(),
        CborValue::Array(vec![CborValue::Bool(true), CborValue::Int(-2)]),
    )]))
    .unwrap();
    let request = CodecOperationRequest {
        operation: Some(
            codec_proto::generated::proto::reallyme::codec::v1::CodecDagCborDecodeRequest {
                encoded: encoded.to_vec(),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };

    let result = result_payload(&process_operation_response(&request.encode_to_vec()));
    let mut decoded = decode_protobuf::<CodecDagCborDecodeResult>(result.bytes()).unwrap();
    let reencoded_request = CodecOperationRequest {
        operation: Some(
            codec_proto::generated::proto::reallyme::codec::v1::CodecDagCborEncodeRequest {
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
    let reencoded = decode_protobuf::<CodecDagCborEncodeResult>(reencoded.bytes()).unwrap();
    assert_eq!(reencoded.encoded.as_slice(), encoded.as_slice());
}

#[test]
fn dag_cbor_encode_rejects_integer_map_keys() {
    let request = CodecOperationRequest {
        operation: Some(
            codec_proto::generated::proto::reallyme::codec::v1::CodecDagCborEncodeRequest {
                value: buffa::MessageField::some(deterministic_cbor_map_fixture()),
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
fn dag_cbor_encode_rejects_unsigned_values_outside_i64() {
    let request = CodecOperationRequest {
        operation: Some(
            codec_proto::generated::proto::reallyme::codec::v1::CodecDagCborEncodeRequest {
                value: buffa::MessageField::some(CodecDeterministicCborValue {
                    value: Some(
                        CodecDeterministicCborInteger {
                            value: Some(
                                CodecDeterministicCborUnsignedInteger {
                                    value: i64::MAX.unsigned_abs() + 1,
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
fn pem_decode_dispatch_matches_primitive_decoder() {
    let der = b"not real der";
    let pem = codec_pem::encode_pem(
        PemLabel::PublicKey,
        der,
        codec_pem::PemEncodeOptions::default(),
    )
    .unwrap();
    let policy = PemDecodePolicy {
        allowed_labels: &[PemLabel::PublicKey],
        ..PemDecodePolicy::default()
    };
    let primitive = codec_pem::decode_pem(&pem, policy).unwrap();
    let request = pem_decode_request(
        pem.as_bytes(),
        Some(CodecPemDecodeOptions {
            allowed_labels: vec![EnumValue::from(CodecPemLabel::CODEC_PEM_LABEL_PUBLIC_KEY)],
            max_input_len: 0,
            max_der_len: 0,
            __buffa_unknown_fields: Default::default(),
        }),
    );

    let binary = process_binary_and_proto_json(&request);

    let result = result_payload(&binary);
    let decoded = decode_protobuf::<CodecPemDecodeResult>(result.bytes()).unwrap();
    assert_eq!(decoded.label, primitive.label.as_str());
    assert_eq!(decoded.der, primitive.der.as_slice());
}

#[test]
fn pem_generated_result_takes_semantic_der_ownership() {
    let der = b"private der";
    let pem = codec_pem::encode_pem(
        PemLabel::PrivateKey,
        der,
        codec_pem::PemEncodeOptions::default(),
    )
    .unwrap();
    let decoded = decode_pem(&pem, PemDecodePolicy::default()).unwrap();
    let der_allocation = decoded.der().as_ptr();

    let result = pem_decode_result_proto(decoded).unwrap();

    // This pointer equality is the regression proof that private DER is
    // moved from one zeroizing owner to the next, not copied through an
    // additional secret-bearing allocation.
    assert_eq!(result.der.as_ptr(), der_allocation);
    assert_eq!(result.der, der);
}

#[test]
fn pem_decode_dispatch_preserves_label_mismatch_error() {
    let pem = b"-----BEGIN PUBLIC KEY-----\nAA==\n-----END PRIVATE KEY-----\n";
    let request = pem_decode_request(pem, None);
    let envelope = process_binary_and_proto_json(&request);
    let error = codec_error_payload(&envelope);

    assert_eq!(error.branch(), CodecWireErrorBranch::Pem);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_PEM_LABEL_MISMATCH
    );
}

#[test]
fn pem_decode_dispatch_preserves_unsupported_label_error() {
    let der = b"not real der";
    let pem = codec_pem::encode_pem(
        PemLabel::PublicKey,
        der,
        codec_pem::PemEncodeOptions::default(),
    )
    .unwrap();
    let request = pem_decode_request(
        pem.as_bytes(),
        Some(CodecPemDecodeOptions {
            allowed_labels: vec![EnumValue::from(CodecPemLabel::CODEC_PEM_LABEL_PRIVATE_KEY)],
            max_input_len: 0,
            max_der_len: 0,
            __buffa_unknown_fields: Default::default(),
        }),
    );
    let envelope = process_binary_and_proto_json(&request);
    let error = codec_error_payload(&envelope);

    assert_eq!(error.branch(), CodecWireErrorBranch::Pem);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_PEM_UNSUPPORTED_LABEL
    );
}

#[test]
fn remaining_structured_requests_reject_binary_unknown_fields() {
    let mut multikey = CodecMultikeyParseRequest {
        multikey: "not-a-key".to_owned(),
        __buffa_unknown_fields: Default::default(),
    };
    multikey.__buffa_unknown_fields.push(UnknownField {
        number: 99,
        data: UnknownFieldData::Varint(7),
    });

    let mut dag_cbor = CodecDagCborVerifyCidRequest {
        cid: String::new(),
        payload: vec![0xa0],
        __buffa_unknown_fields: Default::default(),
    };
    dag_cbor.__buffa_unknown_fields.push(UnknownField {
        number: 99,
        data: UnknownFieldData::Varint(7),
    });

    let mut options = CodecPemDecodeOptions {
        allowed_labels: Vec::new(),
        max_input_len: 0,
        max_der_len: 0,
        __buffa_unknown_fields: Default::default(),
    };
    options.__buffa_unknown_fields.push(UnknownField {
        number: 99,
        data: UnknownFieldData::Varint(7),
    });

    let requests = [
        CodecOperationRequest {
            operation: Some(multikey.into()),
            __buffa_unknown_fields: Default::default(),
        },
        CodecOperationRequest {
            operation: Some(dag_cbor.into()),
            __buffa_unknown_fields: Default::default(),
        },
        pem_decode_request(b"", Some(options)),
    ];

    for request in requests {
        let error = codec_error_payload(&process_operation_response(&request.encode_to_vec()));
        assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
        assert_eq!(
            error.reason(),
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_PROTOBUF
        );
    }
}

#[test]
fn remaining_structured_requests_reject_proto_json_unknown_fields() {
    let requests = [
        br#"{"multikeyParse":{"multikey":"not-a-key","unknown":7}}"#.as_slice(),
        br#"{"dagCborVerifyCid":{"cid":"","payload":"oA==","unknown":7}}"#.as_slice(),
        br#"{"pemDecode":{"pem":"","options":{"unknown":7}}}"#.as_slice(),
    ];

    for request in requests {
        let error = codec_error_payload(&process_operation_response_json(request));
        assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
        assert_eq!(
            error.reason(),
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_JSON
        );
    }
}

#[test]
fn malformed_binary_is_a_structured_boundary_error() {
    let error = codec_error_payload(&process_operation_response(&[0xff]));
    assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_PROTOBUF
    );
}
