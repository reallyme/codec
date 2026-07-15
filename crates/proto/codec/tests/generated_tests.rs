// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Tests for generated codec protobuf bindings.

#![cfg(feature = "generated")]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use buffa::{EnumValue, Enumeration, Message};
use reallyme_codec_proto::generated::{
    proto::reallyme::codec::v1::{
        __buffa::oneof::codec_error, CodecBaseEncodingError, CodecCanonicalizationError,
        CodecDagCborVerifyCidResult, CodecError, CodecErrorReason, CodecKeyMaterialKind,
        CodecMulticodecSpec, CodecMultiformatError, CodecMultikeyParseResult, CodecPemDecodeResult,
        CodecPemError, CodecTag,
    },
    CODEC_PROTO_PACKAGE,
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
    ];

    for (message, expected) in cases {
        assert_eq!(message.encode_to_vec(), expected);
    }
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
