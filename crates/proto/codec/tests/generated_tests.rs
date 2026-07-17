// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Tests for generated codec protobuf bindings.

#![cfg(feature = "generated")]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use buffa::{EnumValue, Enumeration, Message, MessageView};
use reallyme_codec_proto::generated::{
    proto::reallyme::codec::v1::{
        __buffa::oneof::codec_error, CodecBackendError, CodecBaseEncodingError,
        CodecCanonicalizationError, CodecDagCborVerifyCidRequest, CodecDagCborVerifyCidResult,
        CodecError, CodecErrorReason, CodecKeyMaterialKind, CodecMulticodecLookupPrefixRequest,
        CodecMulticodecPrefixForNameRequest, CodecMulticodecSpec, CodecMulticodecTableRequest,
        CodecMultiformatError, CodecMultikeyParseRequest, CodecMultikeyParseResult,
        CodecOperationRequest, CodecOperationRequestView, CodecPemDecodeRequest,
        CodecPemDecodeResult, CodecPemDecodeResultView, CodecPemError, CodecProtoResultEnvelope,
        CodecProtoResultStatus, CodecTag,
    },
    CODEC_PROTO_PACKAGE,
};
use reallyme_codec_proto::{
    decode_codec_error_payload, decode_json, decode_protobuf, encode_protobuf, CodecWireError,
    CodecWireErrorBranch, CodecWireErrorConstructionError, MAX_CODEC_PROTO_ERROR_ENVELOPE_BYTES,
    MAX_CODEC_PROTO_JSON_BYTES, MAX_CODEC_PROTO_MESSAGE_BYTES,
};

#[test]
fn proto_package_name_is_stable() {
    assert_eq!(CODEC_PROTO_PACKAGE, "reallyme.codec.v1");
}

#[test]
fn error_reason_enum_value_is_stable() {
    assert_eq!(
        CodecErrorReason::CODEC_ERROR_REASON_PEM_INVALID_BOUNDARY.to_i32(),
        200
    );
    assert_eq!(
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_PROTOBUF.to_i32(),
        600
    );
}

#[test]
fn codec_error_envelope_round_trips_with_buffa() {
    let error = CodecError {
        error: CodecPemError {
            reason: EnumValue::from(CodecErrorReason::CODEC_ERROR_REASON_PEM_INVALID_BOUNDARY),
            __buffa_unknown_fields: Default::default(),
        }
        .into(),
        __buffa_unknown_fields: Default::default(),
    };

    let encoded = error.encode_to_vec();
    let decoded = CodecError::decode(&mut encoded.as_slice()).unwrap();

    assert_eq!(decoded, error);
}

#[test]
fn codec_result_envelope_round_trips_with_buffa() {
    let result = CodecPemDecodeResult {
        label: "PUBLIC KEY".to_owned(),
        der: vec![0x30, 0x03, 0x02, 0x01, 0x01],
        __buffa_unknown_fields: Default::default(),
    };

    let encoded = result.encode_to_vec();
    let decoded = CodecPemDecodeResult::decode(&mut encoded.as_slice()).unwrap();

    assert_eq!(decoded, result);
}

#[test]
fn pem_decode_result_debug_redacts_der() {
    let result = CodecPemDecodeResult {
        label: "PRIVATE KEY".to_owned(),
        der: vec![0x30, 0x82, 0x01, 0x0a],
        __buffa_unknown_fields: Default::default(),
    };

    let debug = format!("{result:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("130"));

    let encoded = result.encode_to_vec();
    let view = CodecPemDecodeResultView::decode_view(&encoded).unwrap();
    let view_debug = format!("{view:?}");
    assert!(view_debug.contains("<redacted>"));
    assert!(!view_debug.contains("130"));
}

#[test]
fn generated_proto_json_rejects_unknown_fields() {
    let envelope = serde_json::from_str::<CodecProtoResultEnvelope>(
        r#"{"status":"CODEC_PROTO_RESULT_STATUS_RESULT","payload":"","statuz":"RESULT"}"#,
    );
    assert!(envelope.is_err());

    let request = serde_json::from_str::<CodecPemDecodeRequest>(
        r#"{"pem":"","options":{},"privateKeyTypo":""}"#,
    );
    assert!(request.is_err());

    // CodecOperationRequest uses Buffa's flattened-oneof visitor rather than
    // the simple derived Wire helper above. Keep a direct regression test so a
    // generator change cannot restore IgnoredAny on that separate path.
    let operation = serde_json::from_str::<CodecOperationRequest>(
        r#"{"multicodecTable":{},"unexpectedOperation":{}}"#,
    );
    assert!(operation.is_err());
}

#[test]
fn generated_sensitive_fields_redact_debug_output() {
    let envelope = CodecProtoResultEnvelope {
        status: EnumValue::from(CodecProtoResultStatus::CODEC_PROTO_RESULT_STATUS_RESULT),
        payload: vec![241, 242, 243, 244],
        __buffa_unknown_fields: Default::default(),
    };
    assert_redacts_field(format!("{envelope:?}"), "payload");

    let lookup = CodecMulticodecLookupPrefixRequest {
        value: vec![241, 242, 243, 244],
        __buffa_unknown_fields: Default::default(),
    };
    assert_redacts_field(format!("{lookup:?}"), "value");

    let dag_cbor = CodecDagCborVerifyCidRequest {
        cid: "bafy".to_owned(),
        payload: vec![241, 242, 243, 244],
        __buffa_unknown_fields: Default::default(),
    };
    assert_redacts_field(format!("{dag_cbor:?}"), "payload");

    let pem_request = CodecPemDecodeRequest {
        pem: b"-----BEGIN PRIVATE KEY-----".to_vec(),
        options: buffa::MessageField::none(),
        __buffa_unknown_fields: Default::default(),
    };
    let pem_debug = format!("{pem_request:?}");
    assert!(pem_debug.contains("pem"));
    assert!(pem_debug.contains("<redacted>"));
    assert!(!pem_debug.contains("PRIVATE KEY"));

    let multikey = CodecMultikeyParseResult {
        codec_name: "ed25519-pub".to_owned(),
        algorithm_name: "Ed25519".to_owned(),
        public_key: vec![241, 242, 243, 244],
        expected_public_key_length: 32,
        variable_public_key_length: false,
        __buffa_unknown_fields: Default::default(),
    };
    assert_redacts_field(format!("{multikey:?}"), "public_key");

    let operation = CodecOperationRequest {
        operation: Some(pem_request.into()),
        __buffa_unknown_fields: Default::default(),
    };
    let operation_debug = format!("{operation:?}");
    assert!(operation_debug.contains("<redacted>"));
    assert!(!operation_debug.contains("PRIVATE KEY"));

    let encoded = operation.encode_to_vec();
    let operation_view = CodecOperationRequestView::decode_view(&encoded).unwrap();
    let operation_view_debug = format!("{operation_view:?}");
    assert!(operation_view_debug.contains("<redacted>"));
    assert!(!operation_view_debug.contains("PRIVATE KEY"));
}

#[test]
fn operation_request_wire_tags_use_sparse_family_bands() {
    let cases = [
        (
            CodecOperationRequest {
                operation: Some(
                    CodecMulticodecPrefixForNameRequest {
                        name: String::new(),
                        __buffa_unknown_fields: Default::default(),
                    }
                    .into(),
                ),
                __buffa_unknown_fields: Default::default(),
            },
            &[0xc2, 0x3e, 0x00][..],
        ),
        (
            CodecOperationRequest {
                operation: Some(
                    CodecMulticodecLookupPrefixRequest {
                        value: Vec::new(),
                        __buffa_unknown_fields: Default::default(),
                    }
                    .into(),
                ),
                __buffa_unknown_fields: Default::default(),
            },
            &[0xca, 0x3e, 0x00][..],
        ),
        (
            CodecOperationRequest {
                operation: Some(
                    CodecMulticodecTableRequest {
                        __buffa_unknown_fields: Default::default(),
                    }
                    .into(),
                ),
                __buffa_unknown_fields: Default::default(),
            },
            &[0xd2, 0x3e, 0x00][..],
        ),
        (
            CodecOperationRequest {
                operation: Some(
                    CodecMultikeyParseRequest {
                        multikey: String::new(),
                        __buffa_unknown_fields: Default::default(),
                    }
                    .into(),
                ),
                __buffa_unknown_fields: Default::default(),
            },
            &[0x82, 0x7d, 0x00][..],
        ),
        (
            CodecOperationRequest {
                operation: Some(
                    CodecDagCborVerifyCidRequest {
                        cid: String::new(),
                        payload: Vec::new(),
                        __buffa_unknown_fields: Default::default(),
                    }
                    .into(),
                ),
                __buffa_unknown_fields: Default::default(),
            },
            &[0xc2, 0xbb, 0x01, 0x00][..],
        ),
        (
            CodecOperationRequest {
                operation: Some(
                    CodecPemDecodeRequest {
                        pem: Vec::new(),
                        options: buffa::MessageField::none(),
                        __buffa_unknown_fields: Default::default(),
                    }
                    .into(),
                ),
                __buffa_unknown_fields: Default::default(),
            },
            &[0x82, 0xfa, 0x01, 0x00][..],
        ),
    ];

    for (request, expected) in cases {
        assert_eq!(request.encode_to_vec(), expected);
    }
}

#[test]
fn operation_request_rejects_old_compact_operation_tags() {
    let error = decode_protobuf::<CodecOperationRequest>(&[0x0a, 0x00]).unwrap_err();
    assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_PROTOBUF,
    );
}

#[test]
fn codec_error_oneof_wire_bytes_are_stable() {
    let cases = [
        (
            CodecError {
                error: Some(codec_error::Error::BaseEncoding(Box::new(
                    CodecBaseEncodingError {
                        reason: EnumValue::from(
                            CodecErrorReason::CODEC_ERROR_REASON_BASE_INVALID_HEX,
                        ),
                        __buffa_unknown_fields: Default::default(),
                    },
                ))),
                __buffa_unknown_fields: Default::default(),
            },
            &[0x0a, 0x02, 0x08, 0x78][..],
        ),
        (
            CodecError {
                error: Some(codec_error::Error::Pem(Box::new(CodecPemError {
                    reason: EnumValue::from(
                        CodecErrorReason::CODEC_ERROR_REASON_PEM_UNSUPPORTED_LABEL,
                    ),
                    __buffa_unknown_fields: Default::default(),
                }))),
                __buffa_unknown_fields: Default::default(),
            },
            &[0x12, 0x03, 0x08, 0xca, 0x01][..],
        ),
        (
            CodecError {
                error: Some(codec_error::Error::Multiformat(Box::new(
                    CodecMultiformatError {
                        reason: EnumValue::from(
                            CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC,
                        ),
                        __buffa_unknown_fields: Default::default(),
                    },
                ))),
                __buffa_unknown_fields: Default::default(),
            },
            &[0x1a, 0x03, 0x08, 0xae, 0x02][..],
        ),
        (
            CodecError {
                error: Some(codec_error::Error::Canonicalization(Box::new(
                    CodecCanonicalizationError {
                        reason: EnumValue::from(
                            CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_INTERNAL,
                        ),
                        __buffa_unknown_fields: Default::default(),
                    },
                ))),
                __buffa_unknown_fields: Default::default(),
            },
            &[0x22, 0x03, 0x08, 0x94, 0x03][..],
        ),
        (
            CodecError {
                error: Some(codec_error::Error::Backend(Box::new(CodecBackendError {
                    reason: EnumValue::from(CodecErrorReason::CODEC_ERROR_REASON_BACKEND_INTERNAL),
                    __buffa_unknown_fields: Default::default(),
                }))),
                __buffa_unknown_fields: Default::default(),
            },
            &[0x2a, 0x03, 0x08, 0xf4, 0x03][..],
        ),
    ];

    for (message, expected) in cases {
        assert_eq!(message.encode_to_vec(), expected);
    }
}

#[test]
fn codec_wire_error_constructor_rejects_wrong_branch_reasons() {
    assert_eq!(
        CodecWireError::try_new(
            CodecWireErrorBranch::Pem,
            CodecErrorReason::CODEC_ERROR_REASON_BASE_INVALID_HEX,
        ),
        Err(CodecWireErrorConstructionError::BranchReasonMismatch),
    );
    assert_eq!(
        CodecWireError::try_new(
            CodecWireErrorBranch::Backend,
            CodecErrorReason::CODEC_ERROR_REASON_UNSPECIFIED,
        ),
        Err(CodecWireErrorConstructionError::BranchReasonMismatch),
    );
    let error = CodecWireError::try_new(
        CodecWireErrorBranch::Backend,
        CodecErrorReason::CODEC_ERROR_REASON_BACKEND_INTERNAL,
    )
    .unwrap();
    assert_eq!(error.branch(), CodecWireErrorBranch::Backend);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_BACKEND_INTERNAL
    );
}

#[test]
fn malformed_codec_error_payloads_decode_as_boundary_malformed_protobuf() {
    for payload in [
        zeroize::Zeroizing::new(vec![0xff]),
        encode_protobuf(&CodecError {
            error: None,
            __buffa_unknown_fields: Default::default(),
        }),
        encode_protobuf(&CodecError {
            error: Some(codec_error::Error::BaseEncoding(Box::new(
                CodecBaseEncodingError {
                    reason: EnumValue::from(999),
                    __buffa_unknown_fields: Default::default(),
                },
            ))),
            __buffa_unknown_fields: Default::default(),
        }),
        encode_protobuf(&CodecError {
            error: Some(codec_error::Error::BaseEncoding(Box::new(
                CodecBaseEncodingError {
                    reason: EnumValue::from(CodecErrorReason::CODEC_ERROR_REASON_UNSPECIFIED),
                    __buffa_unknown_fields: Default::default(),
                },
            ))),
            __buffa_unknown_fields: Default::default(),
        }),
        encode_protobuf(&CodecError {
            error: Some(codec_error::Error::BaseEncoding(Box::new(
                CodecBaseEncodingError {
                    reason: EnumValue::from(
                        CodecErrorReason::CODEC_ERROR_REASON_PEM_UNSUPPORTED_LABEL,
                    ),
                    __buffa_unknown_fields: Default::default(),
                },
            ))),
            __buffa_unknown_fields: Default::default(),
        }),
    ] {
        let error = decode_codec_error_payload(&payload).unwrap_err();
        assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
        assert_eq!(
            error.reason(),
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_PROTOBUF,
        );
    }
}

#[test]
fn codec_error_payload_decode_accepts_valid_branch_reason_pairs() {
    let payload = encode_protobuf(&CodecError {
        error: Some(codec_error::Error::Backend(Box::new(CodecBackendError {
            reason: EnumValue::from(CodecErrorReason::CODEC_ERROR_REASON_BACKEND_INTERNAL),
            __buffa_unknown_fields: Default::default(),
        }))),
        __buffa_unknown_fields: Default::default(),
    });

    let error = decode_codec_error_payload(&payload).unwrap();
    assert_eq!(error.branch(), CodecWireErrorBranch::Backend);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_BACKEND_INTERNAL,
    );
}

#[test]
fn bounded_protobuf_decode_rejects_oversized_messages() {
    let payload = vec![0_u8; MAX_CODEC_PROTO_MESSAGE_BYTES + 1];
    let error = decode_protobuf::<CodecPemDecodeResult>(&payload).unwrap_err();
    assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
    );
}

#[test]
fn codec_error_payload_decode_rejects_oversized_envelopes() {
    let payload = vec![0_u8; MAX_CODEC_PROTO_ERROR_ENVELOPE_BYTES + 1];
    let error = decode_codec_error_payload(&payload).unwrap_err();
    assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
    );
}

#[test]
fn json_decode_rejects_inputs_that_expand_past_binary_cap() {
    let message = CodecPemDecodeResult {
        label: "PUBLIC KEY".to_owned(),
        der: vec![7_u8; MAX_CODEC_PROTO_MESSAGE_BYTES],
        __buffa_unknown_fields: Default::default(),
    };
    let json = serde_json::to_vec(&message).unwrap();
    assert!(json.len() < MAX_CODEC_PROTO_JSON_BYTES);

    let error = decode_json::<CodecPemDecodeResult>(&json).unwrap_err();
    assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
    );
}

#[test]
fn codec_result_wire_bytes_are_stable() {
    let pem = CodecPemDecodeResult {
        label: "PUBLIC KEY".to_owned(),
        der: vec![0x30, 0x03, 0x02, 0x01, 0x01],
        __buffa_unknown_fields: Default::default(),
    };
    assert_eq!(
        pem.encode_to_vec(),
        &[
            0x0a, 0x0a, b'P', b'U', b'B', b'L', b'I', b'C', b' ', b'K', b'E', b'Y', 0x12, 0x05,
            0x30, 0x03, 0x02, 0x01, 0x01,
        ]
    );

    let cid = CodecDagCborVerifyCidResult {
        valid: true,
        expected_cid: "bafy".to_owned(),
        actual_cid: "bafy".to_owned(),
        __buffa_unknown_fields: Default::default(),
    };
    assert_eq!(
        cid.encode_to_vec(),
        &[0x08, 0x01, 0x12, 0x04, b'b', b'a', b'f', b'y', 0x1a, 0x04, b'b', b'a', b'f', b'y']
    );

    let multikey = CodecMultikeyParseResult {
        codec_name: "ed25519-pub".to_owned(),
        algorithm_name: "Ed25519".to_owned(),
        public_key: vec![0x01, 0x02, 0x03],
        expected_public_key_length: 32,
        variable_public_key_length: false,
        __buffa_unknown_fields: Default::default(),
    };
    assert_eq!(
        multikey.encode_to_vec(),
        &[
            0x0a, 0x0b, b'e', b'd', b'2', b'5', b'5', b'1', b'9', b'-', b'p', b'u', b'b', 0x12,
            0x07, b'E', b'd', b'2', b'5', b'5', b'1', b'9', 0x1a, 0x03, 0x01, 0x02, 0x03, 0x20,
            0x20,
        ]
    );
}

#[test]
fn multicodec_spec_wire_bytes_pin_duplicate_code_and_prefix_fields() {
    let spec = CodecMulticodecSpec {
        name: "ed25519-pub".to_owned(),
        code: vec![0xed, 0x01],
        prefix: vec![0xed, 0x01],
        tag: EnumValue::from(CodecTag::CODEC_TAG_KEY),
        key_material_kind: EnumValue::from(
            CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_PUBLIC_KEY,
        ),
        fixed_length: 32,
        variable_length: false,
        algorithm_name: "Ed25519".to_owned(),
        __buffa_unknown_fields: Default::default(),
    };

    assert_eq!(
        spec.encode_to_vec(),
        &[
            0x0a, 0x0b, b'e', b'd', b'2', b'5', b'5', b'1', b'9', b'-', b'p', b'u', b'b', 0x12,
            0x02, 0xed, 0x01, 0x1a, 0x02, 0xed, 0x01, 0x20, 0x02, 0x28, 0x01, 0x30, 0x20, 0x42,
            0x07, b'E', b'd', b'2', b'5', b'5', b'1', b'9',
        ]
    );
}

fn assert_redacts_field(debug: String, field_name: &str) {
    assert!(debug.contains(field_name));
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("241"));
    assert!(!debug.contains("242"));
    assert!(!debug.contains("243"));
    assert!(!debug.contains("244"));
}
