// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use super::{
    assert_redacts_sensitive_debug, deterministic_cbor_map_value, deterministic_cbor_text_value,
};
use buffa::{Message, MessageView};
use reallyme_codec_proto::generated::proto::reallyme::codec::v1::{
    CodecDagCborVerifyCidRequest, CodecDagCborVerifyCidResult, CodecDeterministicCborDecodeRequest,
    CodecDeterministicCborDecodeResult, CodecDeterministicCborEncodeRequest,
    CodecDeterministicCborEncodeResult, CodecDeterministicCborInteger,
    CodecDeterministicCborNegativeInteger, CodecDeterministicCborUnsignedInteger,
    CodecDeterministicCborValueView, CodecError, CodecErrorReason,
    CodecMulticodecLookupPrefixRequest, CodecMulticodecLookupResult,
    CodecMulticodecPrefixForNameRequest, CodecMulticodecSpec, CodecMulticodecTableRequest,
    CodecMulticodecTableResult, CodecMultikeyParseRequest, CodecMultikeyParseResult,
    CodecOperationRequest, CodecOperationResponse, CodecOperationResponseView,
    CodecOperationResult, CodecOperationResultView, CodecPemDecodeRequest, CodecPemDecodeResult,
};
use reallyme_codec_proto::{decode_protobuf, CodecWireErrorBranch};

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
        (
            CodecOperationRequest {
                operation: Some(
                    CodecDeterministicCborEncodeRequest {
                        value: buffa::MessageField::none(),
                        __buffa_unknown_fields: Default::default(),
                    }
                    .into(),
                ),
                __buffa_unknown_fields: Default::default(),
            },
            &[0xc2, 0xb8, 0x02, 0x00][..],
        ),
        (
            CodecOperationRequest {
                operation: Some(
                    CodecDeterministicCborDecodeRequest {
                        encoded: Vec::new(),
                        __buffa_unknown_fields: Default::default(),
                    }
                    .into(),
                ),
                __buffa_unknown_fields: Default::default(),
            },
            &[0xca, 0xb8, 0x02, 0x00][..],
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
fn operation_result_wire_tags_use_sparse_family_bands() {
    let cases = [
        (
            CodecOperationResult {
                result: Some(CodecMulticodecSpec::default().into()),
                __buffa_unknown_fields: Default::default(),
            },
            &[0xc2, 0x3e, 0x00][..],
        ),
        (
            CodecOperationResult {
                result: Some(CodecMulticodecLookupResult::default().into()),
                __buffa_unknown_fields: Default::default(),
            },
            &[0xca, 0x3e, 0x00][..],
        ),
        (
            CodecOperationResult {
                result: Some(CodecMulticodecTableResult::default().into()),
                __buffa_unknown_fields: Default::default(),
            },
            &[0xd2, 0x3e, 0x00][..],
        ),
        (
            CodecOperationResult {
                result: Some(CodecMultikeyParseResult::default().into()),
                __buffa_unknown_fields: Default::default(),
            },
            &[0x82, 0x7d, 0x00][..],
        ),
        (
            CodecOperationResult {
                result: Some(CodecDagCborVerifyCidResult::default().into()),
                __buffa_unknown_fields: Default::default(),
            },
            &[0xc2, 0xbb, 0x01, 0x00][..],
        ),
        (
            CodecOperationResult {
                result: Some(CodecPemDecodeResult::default().into()),
                __buffa_unknown_fields: Default::default(),
            },
            &[0x82, 0xfa, 0x01, 0x00][..],
        ),
        (
            CodecOperationResult {
                result: Some(CodecDeterministicCborEncodeResult::default().into()),
                __buffa_unknown_fields: Default::default(),
            },
            &[0xc2, 0xb8, 0x02, 0x00][..],
        ),
        (
            CodecOperationResult {
                result: Some(CodecDeterministicCborDecodeResult::default().into()),
                __buffa_unknown_fields: Default::default(),
            },
            &[0xca, 0xb8, 0x02, 0x00][..],
        ),
    ];

    for (result, expected) in cases {
        assert_eq!(result.encode_to_vec(), expected);
    }
}

#[test]
fn operation_response_outcome_wire_tags_are_stable() {
    let result = CodecOperationResponse {
        outcome: Some(CodecOperationResult::default().into()),
        __buffa_unknown_fields: Default::default(),
    };
    assert_eq!(result.encode_to_vec(), &[0x0a, 0x00]);

    let error = CodecOperationResponse {
        outcome: Some(CodecError::default().into()),
        __buffa_unknown_fields: Default::default(),
    };
    assert_eq!(error.encode_to_vec(), &[0x12, 0x00]);
}

#[test]
fn deterministic_cbor_contract_wire_bytes_are_stable() {
    let unsigned = CodecDeterministicCborUnsignedInteger {
        value: 24,
        __buffa_unknown_fields: Default::default(),
    };
    assert_eq!(unsigned.encode_to_vec(), &[0x08, 0x18]);

    let negative = CodecDeterministicCborNegativeInteger {
        value: -1,
        __buffa_unknown_fields: Default::default(),
    };
    assert_eq!(negative.encode_to_vec(), &[0x08, 0x01]);

    let integer = CodecDeterministicCborInteger {
        value: Some(unsigned.into()),
        __buffa_unknown_fields: Default::default(),
    };
    assert_eq!(integer.encode_to_vec(), &[0x0a, 0x02, 0x08, 0x18]);

    let text_value = deterministic_cbor_text_value("hi");
    assert_eq!(
        text_value.encode_to_vec(),
        &[0x22, 0x04, 0x0a, 0x02, b'h', b'i']
    );

    let encode_result = CodecDeterministicCborEncodeResult {
        encoded: vec![0xa0],
        __buffa_unknown_fields: Default::default(),
    };
    assert_eq!(encode_result.encode_to_vec(), &[0x0a, 0x01, 0xa0]);

    let prefix_result = CodecOperationResult {
        result: Some(CodecMulticodecSpec::default().into()),
        __buffa_unknown_fields: Default::default(),
    };
    assert_eq!(prefix_result.encode_to_vec(), &[0xc2, 0x3e, 0x00]);

    let operation_result = CodecOperationResult {
        result: Some(encode_result.into()),
        __buffa_unknown_fields: Default::default(),
    };
    assert_eq!(
        operation_result.encode_to_vec(),
        &[0xc2, 0xb8, 0x02, 0x03, 0x0a, 0x01, 0xa0]
    );

    let response = CodecOperationResponse {
        outcome: Some(operation_result.into()),
        __buffa_unknown_fields: Default::default(),
    };
    assert_eq!(
        response.encode_to_vec(),
        &[0x0a, 0x07, 0xc2, 0xb8, 0x02, 0x03, 0x0a, 0x01, 0xa0]
    );

    let decode_result = CodecOperationResult {
        result: Some(
            CodecDeterministicCborDecodeResult {
                value: buffa::MessageField::some(deterministic_cbor_text_value("hi")),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };
    assert_eq!(
        decode_result.encode_to_vec(),
        &[0xca, 0xb8, 0x02, 0x08, 0x0a, 0x06, 0x22, 0x04, 0x0a, 0x02, b'h', b'i',]
    );
}

#[test]
fn deterministic_cbor_generated_debug_redacts_recursive_values() {
    let text_secret = "P123456789";
    let map = deterministic_cbor_map_value(text_secret);
    let value_debug = format!("{map:?}");
    assert_redacts_sensitive_debug(value_debug);

    let encoded = map.encode_to_vec();
    let view = CodecDeterministicCborValueView::decode_view(&encoded).unwrap();
    assert_redacts_sensitive_debug(format!("{view:?}"));

    let operation_result = CodecOperationResult {
        result: Some(
            CodecDeterministicCborDecodeResult {
                value: buffa::MessageField::some(map),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };
    let response = CodecOperationResponse {
        outcome: Some(operation_result.clone().into()),
        __buffa_unknown_fields: Default::default(),
    };

    let response_debug = format!("{response:?}");
    assert_redacts_sensitive_debug(response_debug);

    let encoded_response = response.encode_to_vec();
    let response_view = CodecOperationResponseView::decode_view(&encoded_response).unwrap();
    assert_redacts_sensitive_debug(format!("{response_view:?}"));

    let result_debug = format!("{operation_result:?}");
    assert_redacts_sensitive_debug(result_debug);

    let encoded_result = operation_result.encode_to_vec();
    let result_view = CodecOperationResultView::decode_view(&encoded_result).unwrap();
    assert_redacts_sensitive_debug(format!("{result_view:?}"));
}
