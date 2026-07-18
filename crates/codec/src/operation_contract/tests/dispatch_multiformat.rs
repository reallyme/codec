// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#[test]
fn binary_and_proto_json_dispatch_match() {
    let requests = [
        (table_request(), true),
        (
            CodecOperationRequest {
                operation: Some(
                    CodecMulticodecPrefixForNameRequest {
                        name: "ed25519-pub".to_owned(),
                        __buffa_unknown_fields: Default::default(),
                    }
                    .into(),
                ),
                __buffa_unknown_fields: Default::default(),
            },
            true,
        ),
        (
            CodecOperationRequest {
                operation: Some(
                    CodecMulticodecLookupPrefixRequest {
                        value: vec![0xed, 0x01, 0xaa],
                        __buffa_unknown_fields: Default::default(),
                    }
                    .into(),
                ),
                __buffa_unknown_fields: Default::default(),
            },
            true,
        ),
        (
            CodecOperationRequest {
                operation: Some(
                    CodecMulticodecPrefixForNameRequest {
                        name: "not-a-codec".to_owned(),
                        __buffa_unknown_fields: Default::default(),
                    }
                    .into(),
                ),
                __buffa_unknown_fields: Default::default(),
            },
            false,
        ),
        (
            CodecOperationRequest {
                operation: Some(
                    CodecMulticodecLookupPrefixRequest {
                        value: vec![0, 0, 7],
                        __buffa_unknown_fields: Default::default(),
                    }
                    .into(),
                ),
                __buffa_unknown_fields: Default::default(),
            },
            false,
        ),
    ];

    for (request, expects_result) in requests {
        let binary = process_operation_response(&request.encode_to_vec());
        let json = serde_json::to_vec(&request).unwrap();
        let from_json = process_operation_response_json(&json);
        assert_eq!(binary.as_slice(), from_json.as_slice());

        let response = decode_protobuf::<CodecOperationResponse>(&binary).unwrap();
        assert_eq!(
            matches!(
                response.outcome,
                Some(codec_operation_response::Outcome::Result(_))
            ),
            expects_result
        );
    }
}

#[test]
fn discriminated_response_selects_the_exact_generated_result_variant() {
    let request = CodecOperationRequest {
        operation: Some(
            CodecMulticodecPrefixForNameRequest {
                name: "ed25519-pub".to_owned(),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };

    let encoded = process_operation_response(&request.encode_to_vec());
    let response = decode_protobuf::<CodecOperationResponse>(&encoded).unwrap();
    assert!(matches!(
        response.outcome.as_ref(),
        Some(codec_operation_response::Outcome::Result(_))
    ));
    let Some(codec_operation_response::Outcome::Result(result)) = response.outcome.as_ref()
    else {
        return;
    };
    let Some(codec_operation_result::Result::MulticodecPrefixForName(result)) =
        result.result.as_ref()
    else {
        return;
    };
    assert_eq!(result.name, "ed25519-pub");
    assert_eq!(result.prefix, [0xed, 0x01]);
}

#[test]
fn discriminated_response_preserves_typed_error_attribution() {
    let request = CodecOperationRequest {
        operation: Some(
            CodecMulticodecPrefixForNameRequest {
                name: "not-a-codec".to_owned(),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };

    let encoded = process_operation_response(&request.encode_to_vec());
    let response = decode_protobuf::<CodecOperationResponse>(&encoded).unwrap();
    assert!(matches!(
        response.outcome.as_ref(),
        Some(codec_operation_response::Outcome::Error(_))
    ));
    let Some(codec_operation_response::Outcome::Error(_)) = response.outcome.as_ref() else {
        return;
    };
    let decoded = codec_error_payload(&encoded);
    assert_eq!(decoded.branch(), CodecWireErrorBranch::Multiformat);
    assert_eq!(
        decoded.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC
    );
}

#[test]
fn multicodec_prefix_for_name_dispatch_uses_semantic_result() {
    let request = CodecOperationRequest {
        operation: Some(
            CodecMulticodecPrefixForNameRequest {
                name: "ed25519-pub".to_owned(),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };

    let result = result_payload(&process_operation_response(&request.encode_to_vec()));
    let decoded = decode_protobuf::<CodecMulticodecSpec>(result.bytes()).unwrap();

    assert_eq!(decoded.name, "ed25519-pub");
    assert_eq!(decoded.algorithm_name, "Ed25519");
    assert_eq!(decoded.prefix, [0xed, 0x01]);
    assert_eq!(decoded.fixed_length, 32);
    assert!(!decoded.variable_length);
}

#[test]
fn multicodec_lookup_prefix_dispatch_uses_semantic_result() {
    let request = CodecOperationRequest {
        operation: Some(
            CodecMulticodecLookupPrefixRequest {
                value: vec![0xed, 0x01, 0xaa],
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };

    let result = result_payload(&process_operation_response(&request.encode_to_vec()));
    let decoded = decode_protobuf::<CodecMulticodecLookupResult>(result.bytes()).unwrap();
    let metadata = decoded.metadata.as_option().unwrap();

    assert_eq!(decoded.name, "ed25519-pub");
    assert_eq!(decoded.prefix_length, 2);
    assert_eq!(metadata.name, "ed25519-pub");
    assert_eq!(metadata.algorithm_name, "Ed25519");
    assert_eq!(metadata.prefix, [0xed, 0x01]);
    assert_eq!(metadata.fixed_length, 32);
    assert!(!metadata.variable_length);
}

#[test]
fn multicodec_table_proto_preserves_every_semantic_field() {
    let semantic_table = supported_table().unwrap();
    let wire_table = multicodec_table_result_proto(&semantic_table).unwrap();

    assert_eq!(wire_table.entries.len(), semantic_table.entries().len());
    for (semantic, wire) in semantic_table.entries().iter().zip(&wire_table.entries) {
        assert_eq!(wire.name, semantic.name());
        assert_eq!(wire.algorithm_name, semantic.algorithm_name());
        assert_eq!(wire.code, semantic.code());
        assert_eq!(wire.prefix, semantic.prefix());

        let expected_tag = match semantic.tag() {
            CodecTag::Encryption => ProtoCodecTag::CODEC_TAG_ENCRYPTION,
            CodecTag::Hash => ProtoCodecTag::CODEC_TAG_HASH,
            CodecTag::Key => ProtoCodecTag::CODEC_TAG_KEY,
            CodecTag::Multihash => ProtoCodecTag::CODEC_TAG_MULTIHASH,
            CodecTag::Multikey => ProtoCodecTag::CODEC_TAG_MULTIKEY,
        };
        assert_eq!(wire.tag.as_known(), Some(expected_tag));

        let expected_key_material = match semantic.key_material() {
            KeyMaterialKind::NotKey => CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_NOT_KEY,
            KeyMaterialKind::PublicKey => {
                CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_PUBLIC_KEY
            }
            KeyMaterialKind::PrivateKey => {
                CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_PRIVATE_KEY
            }
            KeyMaterialKind::SymmetricKey => {
                CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_SYMMETRIC_KEY
            }
        };
        assert_eq!(
            wire.key_material_kind.as_known(),
            Some(expected_key_material)
        );

        let (expected_fixed_length, expected_variable_length) = match semantic.length() {
            MulticodecLength::Fixed(length) => (u32::try_from(length).unwrap(), false),
            MulticodecLength::Variable => (0, true),
            MulticodecLength::NotApplicable => (0, false),
        };
        assert_eq!(wire.fixed_length, expected_fixed_length);
        assert_eq!(wire.variable_length, expected_variable_length);
    }
}

#[test]
fn multicodec_dispatch_preserves_semantic_errors() {
    let unknown_name = CodecOperationRequest {
        operation: Some(
            CodecMulticodecPrefixForNameRequest {
                name: "not-a-codec".to_owned(),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };
    let unknown_error =
        codec_error_payload(&process_operation_response(&unknown_name.encode_to_vec()));
    assert_eq!(unknown_error.branch(), CodecWireErrorBranch::Multiformat);
    assert_eq!(
        unknown_error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC
    );

    let invalid_prefix = CodecOperationRequest {
        operation: Some(
            CodecMulticodecLookupPrefixRequest {
                value: vec![0, 0, 7],
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };
    let prefix_error =
        codec_error_payload(&process_operation_response(&invalid_prefix.encode_to_vec()));
    assert_eq!(prefix_error.branch(), CodecWireErrorBranch::Multiformat);
    assert_eq!(
        prefix_error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTICODEC_PREFIX
    );
}

#[test]
fn multicodec_registry_invariant_is_a_provider_failure() {
    for semantic_error in [
        MulticodecOperationError::RegistryInvariant,
        MulticodecOperationError::AllocationFailure,
    ] {
        let error = multicodec_boundary_error(semantic_error);

        assert_eq!(error.branch(), CodecWireErrorBranch::Backend);
        assert_eq!(
            error.reason(),
            CodecErrorReason::CODEC_ERROR_REASON_BACKEND_INTERNAL
        );
    }
}

#[test]
fn multicodec_requests_reject_nested_unknown_fields() {
    let mut prefix_for_name = CodecMulticodecPrefixForNameRequest {
        name: "ed25519-pub".to_owned(),
        __buffa_unknown_fields: Default::default(),
    };
    prefix_for_name.__buffa_unknown_fields.push(UnknownField {
        number: 99,
        data: UnknownFieldData::Varint(7),
    });

    let mut lookup_prefix = CodecMulticodecLookupPrefixRequest {
        value: vec![0xed, 0x01],
        __buffa_unknown_fields: Default::default(),
    };
    lookup_prefix.__buffa_unknown_fields.push(UnknownField {
        number: 99,
        data: UnknownFieldData::Varint(7),
    });

    let mut table = CodecMulticodecTableRequest {
        __buffa_unknown_fields: Default::default(),
    };
    table.__buffa_unknown_fields.push(UnknownField {
        number: 99,
        data: UnknownFieldData::Varint(7),
    });

    let requests = [
        CodecOperationRequest {
            operation: Some(prefix_for_name.into()),
            __buffa_unknown_fields: Default::default(),
        },
        CodecOperationRequest {
            operation: Some(lookup_prefix.into()),
            __buffa_unknown_fields: Default::default(),
        },
        CodecOperationRequest {
            operation: Some(CodecOperation::MulticodecTable(Box::new(table))),
            __buffa_unknown_fields: Default::default(),
        },
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
fn missing_operation_is_a_structured_boundary_error() {
    let request = CodecOperationRequest::default();
    let error = codec_error_payload(&process_operation_response(&request.encode_to_vec()));
    assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MISSING_OPERATION
    );
}
