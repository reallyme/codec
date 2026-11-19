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
fn shared_pem_bundle_vectors_pin_wire_bytes_and_typed_outcomes() {
    let manifest: Value =
        serde_json::from_str(include_str!("../../../vectors/codec-vectors.json")).unwrap();
    assert_eq!(manifest["schemaVersion"], 2);
    let vectors = &manifest["pemBundleDecode"];
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
            Some(proto::codec_api_operation_request::Operation::PemBundleDecode(_))
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
                Some(proto::codec_api_operation_result::Result::PemBundleDecode(
                    _
                ))
            )),
            proto::codec_api_operation_response::Outcome::Error(error) => {
                assert_eq!(
                    error.origin.as_known(),
                    Some(proto::CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER)
                );
                match error.error.as_ref().unwrap() {
                    proto::codec_api_error::Error::Pem(error) => {
                        assert!(error.reason.as_known().is_some());
                    }
                    proto::codec_api_error::Error::BaseEncoding(error) => assert_eq!(
                        error.reason.as_known(),
                        Some(proto::CodecErrorReason::CODEC_ERROR_REASON_BASE_INPUT_TOO_LARGE)
                    ),
                    _ => panic!("unexpected error family in PEM bundle fixture"),
                }
            }
        }
    }
}
