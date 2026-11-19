// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "operation-contract")]
#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "reviewed public fixtures"
)]

use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::{decode_json, decode_protobuf, encode_protobuf};
use reallyme_codec::operation_contract::process_operation_response;
use serde_json::Value;

fn hex_bytes(value: &Value) -> Vec<u8> {
    reallyme_codec::hex::lower_hex_to_bytes(value.as_str().unwrap()).unwrap()
}

#[test]
fn shared_bounded_base64_vectors_pin_wire_bytes_and_typed_outcomes() {
    let manifest: Value =
        serde_json::from_str(include_str!("../../../vectors/codec-vectors.json")).unwrap();
    assert_eq!(manifest["schemaVersion"], 2);
    let vectors = &manifest["base64DecodeWithLimits"];
    assert_eq!(vectors["fixtureClass"], "golden-operation");
    let cases = vectors["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    for fixture in cases {
        let name = fixture["name"].as_str().unwrap();
        let request: proto::CodecApiOperationRequest =
            decode_json(&serde_json::to_vec(&fixture["request"]).unwrap()).unwrap();
        let request_bytes = hex_bytes(&fixture["requestHex"]);
        assert_eq!(
            encode_protobuf(&request).unwrap().as_slice(),
            request_bytes,
            "{name}"
        );
        assert!(matches!(
            request.operation.as_ref(),
            Some(proto::codec_api_operation_request::Operation::Base64DecodeWithLimits(_))
        ));
        let expected: proto::CodecApiOperationResponse =
            decode_json(&serde_json::to_vec(&fixture["response"]).unwrap()).unwrap();
        let expected_bytes = hex_bytes(&fixture["responseHex"]);
        assert_eq!(
            encode_protobuf(&expected).unwrap().as_slice(),
            expected_bytes,
            "{name}"
        );

        let actual_bytes = process_operation_response(&request_bytes);
        assert_eq!(actual_bytes.as_slice(), expected_bytes, "{name}");
        let actual: proto::CodecApiOperationResponse = decode_protobuf(&actual_bytes).unwrap();
        match actual.outcome.as_ref().unwrap() {
            proto::codec_api_operation_response::Outcome::Result(result) => assert!(matches!(
                result.result.as_ref(),
                Some(proto::codec_api_operation_result::Result::Base64DecodeWithLimits(_))
            )),
            proto::codec_api_operation_response::Outcome::Error(error) => {
                assert_eq!(
                    error.origin.as_known(),
                    Some(proto::CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER)
                );
                let proto::codec_api_error::Error::BaseEncoding(detail) =
                    error.error.as_ref().unwrap()
                else {
                    panic!("unexpected Base64 error family")
                };
                assert!(detail.reason.as_known().is_some());
            }
        }
    }
}

#[test]
fn portrait_budgets_pass_through_the_native_operation_envelope() {
    use buffa::MessageField;
    const INPUT: u64 = 13_981_016;
    const OUTPUT: usize = 10_485_760;
    let payload = zeroize::Zeroizing::new(vec![0x5a; OUTPUT]);
    let encoded = reallyme_codec::base64::bytes_to_base64(&payload);
    let mut request = proto::CodecApiOperationRequest::default();
    request.operation = Some(
        proto::codec_api_operation_request::Operation::Base64DecodeWithLimits(Box::new(
            proto::CodecBase64DecodeRequest {
                value: encoded.into(),
                limits: MessageField::some(proto::CodecBase64DecodeLimits {
                    encoded_input_bytes: Some(INPUT),
                    decoded_output_bytes: Some(u64::try_from(OUTPUT).unwrap()),
                }),
            },
        )),
    );
    let encoded_request = encode_protobuf(&request).unwrap();
    let bytes = process_operation_response(&encoded_request);
    let response: proto::CodecApiOperationResponse = decode_protobuf(&bytes).unwrap();
    let Some(proto::codec_api_operation_response::Outcome::Result(result)) =
        response.outcome.as_ref()
    else {
        panic!("portrait should fit the structured native operation");
    };
    let Some(proto::codec_api_operation_result::Result::Base64DecodeWithLimits(decoded)) =
        result.result.as_ref()
    else {
        panic!("expected bounded Base64 bytes");
    };
    assert_eq!(decoded.value.as_slice(), payload.as_slice());
}

#[test]
fn legacy_scalar_transport_cap_is_not_raised_by_the_new_operation() {
    use codec_proto::MAX_CODEC_PROVIDER_INPUT_BYTES;
    for length in [
        MAX_CODEC_PROVIDER_INPUT_BYTES,
        MAX_CODEC_PROVIDER_INPUT_BYTES + 1,
        MAX_CODEC_PROVIDER_INPUT_BYTES + 4,
    ] {
        let mut request = proto::CodecApiOperationRequest::default();
        request.operation = Some(proto::codec_api_operation_request::Operation::Base64Decode(
            Box::new(proto::CodecTextRequest {
                value: "A".repeat(length).into(),
            }),
        ));
        let bytes = encode_protobuf(&request).unwrap();
        let response = process_operation_response(&bytes);
        let response: proto::CodecApiOperationResponse = decode_protobuf(&response).unwrap();
        if length == MAX_CODEC_PROVIDER_INPUT_BYTES {
            assert!(matches!(
                response.outcome.as_ref(),
                Some(proto::codec_api_operation_response::Outcome::Result(_))
            ));
        } else {
            let Some(proto::codec_api_operation_response::Outcome::Error(error)) =
                response.outcome.as_ref()
            else {
                panic!("legacy scalar request exceeded its unchanged 1 MiB cap");
            };
            let Some(proto::codec_api_error::Error::BaseEncoding(detail)) = error.error.as_ref()
            else {
                panic!("expected typed scalar input limit");
            };
            assert_eq!(
                detail.reason.as_known(),
                Some(proto::CodecErrorReason::CODEC_ERROR_REASON_BASE_INPUT_TOO_LARGE)
            );
        }
    }
}
