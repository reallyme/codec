// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Tests for generated codec protobuf bindings.

#![cfg(feature = "generated")]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

#[path = "generated_tests/error_wire.rs"]
mod error_wire;
#[path = "generated_tests/operation_wire.rs"]
mod operation_wire;

use buffa::{EnumValue, Enumeration, Message, MessageView};
use reallyme_codec_proto::generated::{
    proto::reallyme::codec::v1::{
        CodecDagCborVerifyCidRequest, CodecDeterministicCborArray, CodecDeterministicCborBool,
        CodecDeterministicCborBytes, CodecDeterministicCborDecodeRequest,
        CodecDeterministicCborDecodeResult, CodecDeterministicCborEncodeRequest,
        CodecDeterministicCborEncodeResult, CodecDeterministicCborInteger,
        CodecDeterministicCborMap, CodecDeterministicCborMapEntry, CodecDeterministicCborMapKey,
        CodecDeterministicCborNegativeInteger, CodecDeterministicCborText,
        CodecDeterministicCborUnsignedInteger, CodecDeterministicCborValue, CodecError,
        CodecErrorOrigin, CodecErrorReason, CodecMulticodecLookupPrefixRequest,
        CodecMulticodecPrefixForNameRequest, CodecMultikeyParseResult, CodecOperationRequest,
        CodecOperationRequestView, CodecOperationResponse, CodecPemDecodeRequest,
        CodecPemDecodeResult, CodecPemDecodeResultView, CodecPemError,
    },
    CODEC_PROTO_PACKAGE,
};
use reallyme_codec_proto::{
    decode_json, CodecWireErrorBranch, MAX_CODEC_PROTO_JSON_BYTES, MAX_CODEC_PROTO_MESSAGE_BYTES,
};

#[test]
fn proto_package_name_is_stable() {
    assert_eq!(CODEC_PROTO_PACKAGE, "reallyme.codec.v1");
}

#[test]
fn recursive_transport_caps_are_stable() {
    assert_eq!(MAX_CODEC_PROTO_MESSAGE_BYTES, 10_489_856);
    assert_eq!(MAX_CODEC_PROTO_JSON_BYTES, 16_082_264);
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
        origin: EnumValue::from(CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER),
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
fn sensitive_bytes_proto_json_is_canonical_for_owned_and_borrowed_messages() {
    let cases: &[(&[u8], &str)] = &[
        (&[], r#"{"label":"PRIVATE KEY"}"#),
        (&[0xff], r#"{"label":"PRIVATE KEY","der":"/w=="}"#),
        (&[0xff, 0xee], r#"{"label":"PRIVATE KEY","der":"/+4="}"#),
        (
            &[0xff, 0xee, 0xdd],
            r#"{"label":"PRIVATE KEY","der":"/+7d"}"#,
        ),
        (
            &[0xff, 0xee, 0xdd, 0xcc],
            r#"{"label":"PRIVATE KEY","der":"/+7dzA=="}"#,
        ),
    ];

    for (der, expected_json) in cases {
        let message = CodecPemDecodeResult {
            label: "PRIVATE KEY".to_owned(),
            der: der.to_vec(),
            __buffa_unknown_fields: Default::default(),
        };
        let owned_json = serde_json::to_string(&message).unwrap();
        assert_eq!(owned_json, *expected_json);

        let decoded = serde_json::from_str::<CodecPemDecodeResult>(&owned_json).unwrap();
        assert_eq!(decoded.label, message.label);
        assert_eq!(decoded.der, message.der);

        let encoded = message.encode_to_vec();
        let view = CodecPemDecodeResultView::decode_view(&encoded).unwrap();
        let borrowed_json = serde_json::to_string(&view).unwrap();
        assert_eq!(borrowed_json, owned_json);
    }

    for (json, expected_der) in [
        (r#"{"der":"_-7d"}"#, &[0xff, 0xee, 0xdd][..]),
        (r#"{"der":"/+4"}"#, &[0xff, 0xee][..]),
        (r#"{"der":"/x=="}"#, &[0xff][..]),
        (r#"{"der":null}"#, &[][..]),
    ] {
        let decoded = serde_json::from_str::<CodecPemDecodeResult>(json).unwrap();
        assert_eq!(decoded.der, expected_der);
    }

    assert!(serde_json::from_str::<CodecPemDecodeResult>(r#"{"der":"%%%%"}"#).is_err());
}

#[test]
fn generated_proto_json_rejects_unknown_fields() {
    let response = serde_json::from_str::<CodecOperationResponse>(
        r#"{"result":{"multicodecTable":{"entries":[]}},"unexpected":true}"#,
    );
    assert!(response.is_err());

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
fn generated_proto_json_applies_bounded_structural_nesting() {
    let mut excessive_nesting = vec![b'['; 300];
    excessive_nesting.push(b'0');
    excessive_nesting.extend((0..300).map(|_| b']'));
    let error = decode_json::<CodecOperationRequest>(&excessive_nesting).unwrap_err();
    assert_eq!(error.branch(), CodecWireErrorBranch::Boundary);
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
    );

    let request = decode_json::<CodecMulticodecPrefixForNameRequest>(
        br#"{"name":"braces inside strings do not count: [[{{"}"#,
    )
    .unwrap();
    assert_eq!(request.name, "braces inside strings do not count: [[{{");
}

#[test]
fn generated_sensitive_fields_redact_debug_output() {
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

    let encode_request = CodecDeterministicCborEncodeRequest {
        value: buffa::MessageField::some(deterministic_cbor_text_value("P123456789")),
        __buffa_unknown_fields: Default::default(),
    };
    assert_redacts_sensitive_debug(format!("{encode_request:?}"));

    let encode_result = CodecDeterministicCborEncodeResult {
        encoded: vec![241, 242, 243, 244],
        __buffa_unknown_fields: Default::default(),
    };
    assert_redacts_field(format!("{encode_result:?}"), "encoded");

    let decode_request = CodecDeterministicCborDecodeRequest {
        encoded: vec![241, 242, 243, 244],
        __buffa_unknown_fields: Default::default(),
    };
    assert_redacts_field(format!("{decode_request:?}"), "encoded");

    let decode_result = CodecDeterministicCborDecodeResult {
        value: buffa::MessageField::some(deterministic_cbor_text_value("P123456789")),
        __buffa_unknown_fields: Default::default(),
    };
    assert_redacts_sensitive_debug(format!("{decode_result:?}"));
}

fn assert_redacts_field(debug: String, field_name: &str) {
    assert!(debug.contains(field_name));
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("241"));
    assert!(!debug.contains("242"));
    assert!(!debug.contains("243"));
    assert!(!debug.contains("244"));
}

fn assert_redacts_sensitive_debug(debug: String) {
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("P123456789"));
    assert!(!debug.contains("passport-number"));
    assert!(!debug.contains("987654321"));
    assert!(!debug.contains("241"));
    assert!(!debug.contains("242"));
    assert!(!debug.contains("243"));
    assert!(!debug.contains("244"));
}

fn deterministic_cbor_text_value(value: &str) -> CodecDeterministicCborValue {
    CodecDeterministicCborValue {
        value: Some(
            CodecDeterministicCborText {
                value: value.to_owned(),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    }
}

fn deterministic_cbor_unsigned_value(value: u64) -> CodecDeterministicCborValue {
    CodecDeterministicCborValue {
        value: Some(
            CodecDeterministicCborInteger {
                value: Some(
                    CodecDeterministicCborUnsignedInteger {
                        value,
                        __buffa_unknown_fields: Default::default(),
                    }
                    .into(),
                ),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    }
}

fn deterministic_cbor_negative_value(value: i64) -> CodecDeterministicCborValue {
    CodecDeterministicCborValue {
        value: Some(
            CodecDeterministicCborInteger {
                value: Some(
                    CodecDeterministicCborNegativeInteger {
                        value,
                        __buffa_unknown_fields: Default::default(),
                    }
                    .into(),
                ),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    }
}

fn deterministic_cbor_map_value(secret: &str) -> CodecDeterministicCborValue {
    let map = CodecDeterministicCborMap {
        entries: vec![
            CodecDeterministicCborMapEntry {
                key: buffa::MessageField::some(CodecDeterministicCborMapKey {
                    key: Some(
                        CodecDeterministicCborText {
                            value: "passport-number".to_owned(),
                            __buffa_unknown_fields: Default::default(),
                        }
                        .into(),
                    ),
                    __buffa_unknown_fields: Default::default(),
                }),
                value: buffa::MessageField::some(deterministic_cbor_text_value(secret)),
                __buffa_unknown_fields: Default::default(),
            },
            CodecDeterministicCborMapEntry {
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
                value: buffa::MessageField::some(CodecDeterministicCborValue {
                    value: Some(
                        CodecDeterministicCborBytes {
                            value: vec![241, 242, 243, 244],
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
                            value: "array".to_owned(),
                            __buffa_unknown_fields: Default::default(),
                        }
                        .into(),
                    ),
                    __buffa_unknown_fields: Default::default(),
                }),
                value: buffa::MessageField::some(CodecDeterministicCborValue {
                    value: Some(
                        CodecDeterministicCborArray {
                            values: vec![
                                deterministic_cbor_unsigned_value(987654321),
                                deterministic_cbor_negative_value(-987654321),
                                CodecDeterministicCborValue {
                                    value: Some(
                                        CodecDeterministicCborBool {
                                            value: true,
                                            __buffa_unknown_fields: Default::default(),
                                        }
                                        .into(),
                                    ),
                                    __buffa_unknown_fields: Default::default(),
                                },
                            ],
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
    };

    CodecDeterministicCborValue {
        value: Some(map.into()),
        __buffa_unknown_fields: Default::default(),
    }
}
