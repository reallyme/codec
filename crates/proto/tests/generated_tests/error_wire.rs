// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use buffa::{EnumValue, Message};
use reallyme_codec_proto::generated::proto::reallyme::codec::v1::{
    __buffa::oneof::codec_error, CodecBackendError, CodecBaseEncodingError,
    CodecCanonicalizationError, CodecDagCborVerifyCidResult, CodecError, CodecErrorOrigin,
    CodecErrorReason, CodecKeyMaterialKind, CodecMulticodecSpec, CodecMultiformatError,
    CodecMultikeyParseResult, CodecPemDecodeResult, CodecPemError, CodecTag,
};
use reallyme_codec_proto::{
    codec_error, decode_json, decode_protobuf, CodecWireError, CodecWireErrorBranch,
    CodecWireErrorConstructionError, MAX_CODEC_PROTO_JSON_BYTES, MAX_CODEC_PROTO_MESSAGE_BYTES,
};

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
                origin: EnumValue::from(CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER),
                __buffa_unknown_fields: Default::default(),
            },
            &[0x0a, 0x02, 0x08, 0x78, 0xa0, 0x06, 0x01][..],
        ),
        (
            CodecError {
                error: Some(codec_error::Error::Pem(Box::new(CodecPemError {
                    reason: EnumValue::from(
                        CodecErrorReason::CODEC_ERROR_REASON_PEM_UNSUPPORTED_LABEL,
                    ),
                    __buffa_unknown_fields: Default::default(),
                }))),
                origin: EnumValue::from(CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER),
                __buffa_unknown_fields: Default::default(),
            },
            &[0x12, 0x03, 0x08, 0xca, 0x01, 0xa0, 0x06, 0x01][..],
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
                origin: EnumValue::from(CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER),
                __buffa_unknown_fields: Default::default(),
            },
            &[0x1a, 0x03, 0x08, 0xae, 0x02, 0xa0, 0x06, 0x01][..],
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
                origin: EnumValue::from(CodecErrorOrigin::CODEC_ERROR_ORIGIN_PROVIDER),
                __buffa_unknown_fields: Default::default(),
            },
            &[0x22, 0x03, 0x08, 0x94, 0x03, 0xa0, 0x06, 0x02][..],
        ),
        (
            CodecError {
                error: Some(codec_error::Error::Backend(Box::new(CodecBackendError {
                    reason: EnumValue::from(CodecErrorReason::CODEC_ERROR_REASON_BACKEND_INTERNAL),
                    __buffa_unknown_fields: Default::default(),
                }))),
                origin: EnumValue::from(CodecErrorOrigin::CODEC_ERROR_ORIGIN_PROVIDER),
                __buffa_unknown_fields: Default::default(),
            },
            &[0x2a, 0x03, 0x08, 0xf4, 0x03, 0xa0, 0x06, 0x02][..],
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
fn generated_codec_errors_carry_explicit_validated_origin() {
    for (wire_error, expected_origin) in [
        (
            CodecWireError::try_new(
                CodecWireErrorBranch::BaseEncoding,
                CodecErrorReason::CODEC_ERROR_REASON_BASE_INVALID_HEX,
            )
            .unwrap(),
            CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER,
        ),
        (
            CodecWireError::try_new(
                CodecWireErrorBranch::Backend,
                CodecErrorReason::CODEC_ERROR_REASON_BACKEND_INTERNAL,
            )
            .unwrap(),
            CodecErrorOrigin::CODEC_ERROR_ORIGIN_PROVIDER,
        ),
    ] {
        let generated = codec_error(wire_error);
        assert_eq!(generated.origin.as_known(), Some(expected_origin));
        assert!(generated.error.is_some());
    }
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
