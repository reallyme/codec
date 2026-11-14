// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! ProtoJSON default forms must enter the same semantic and error boundary.
#![cfg(feature = "operation-contract")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "bounded independent wire fixtures"
)]

use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::{decode_protobuf, encode_protobuf};
use reallyme_codec::operation_contract::{
    process_operation_response, process_operation_response_json,
};

#[test]
fn proto_json_default_forms_match_binary_responses_byte_for_byte() {
    for (json, normalized, cbor) in [
        (
            br#"{"generalCborEncode":{"value":{"arrayValue":{"values":null}}}}"#.as_slice(),
            br#"{"generalCborEncode":{"value":{"arrayValue":{"values":[]}}}}"#.as_slice(),
            0x80,
        ),
        (
            br#"{"generalCborEncode":{"value":{"boolValue":{"value":null}}}}"#,
            br#"{"generalCborEncode":{"value":{"boolValue":{"value":false}}}}"#,
            0xf4,
        ),
        (
            br#"{"generalCborEncode":{"value":{"simpleValue":{"value":"16"}}}}"#,
            br#"{"generalCborEncode":{"value":{"simpleValue":{"value":16}}}}"#,
            0xf0,
        ),
    ] {
        let request: proto::CodecApiOperationRequest =
            codec_proto::decode_json(normalized).unwrap();
        let binary = process_operation_response(&encode_protobuf(&request).unwrap());
        let json = process_operation_response_json(json);
        assert_eq!(json, binary);
        let response: proto::CodecApiOperationResponse = decode_protobuf(&json).unwrap();
        let Some(proto::codec_api_operation_response::Outcome::Result(result)) =
            response.outcome.as_ref()
        else {
            panic!("expected successful encoding");
        };
        let Some(proto::codec_api_operation_result::Result::GeneralCborEncode(encoded)) =
            result.result.as_ref()
        else {
            panic!("expected generated CBOR result");
        };
        assert_eq!(encoded.value, [cbor]);
    }

    let null = process_operation_response_json(br#"{"base64Encode":{"value":null}}"#);
    let empty = process_operation_response_json(br#"{"base64Encode":{"value":""}}"#);
    assert_eq!(null, empty);
    let response: proto::CodecApiOperationResponse = decode_protobuf(&null).unwrap();
    let Some(proto::codec_api_operation_response::Outcome::Result(result)) =
        response.outcome.as_ref()
    else {
        panic!("expected empty base64 success");
    };
    assert!(
        matches!(result.result.as_ref(), Some(proto::codec_api_operation_result::Result::Base64Encode(text)) if text.value.is_empty())
    );
}

#[test]
fn invalid_json_values_keep_stable_caller_errors() {
    for request in [
        br#"{"generalCborEncode":{"value":{"arrayValue":{"values":[null]}}}}"#.as_slice(),
        br#"{"generalCborEncode":{"value":{"simpleValue":{"value":"4294967296"}}}}"#,
        br#"{"generalCborEncode":{"value":{"boolValue":{"value":"false"}}}}"#,
        br#"{"generalCborEncode":{"value":{"simpleValue":{"value":16,"unexpected":true}}}}"#,
    ] {
        let response: proto::CodecApiOperationResponse =
            decode_protobuf(&process_operation_response_json(request)).unwrap();
        let Some(proto::codec_api_operation_response::Outcome::Error(error)) =
            response.outcome.as_ref()
        else {
            panic!("expected typed malformed JSON error");
        };
        assert_eq!(
            error.origin.as_known(),
            Some(proto::CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER)
        );
        assert!(
            matches!(error.error.as_ref(), Some(proto::codec_api_error::Error::Boundary(boundary)) if boundary.reason.as_known() == Some(proto::CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_JSON))
        );
    }
}
