// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "operation-contract")]
#![allow(clippy::unwrap_used, missing_docs)]

use buffa::{Message, MessageField};
use codec_proto::generated::proto::reallyme::codec::v1 as p;
use reallyme_codec::{json, operation_contract};

#[test]
#[allow(
    clippy::panic,
    reason = "test assertions on generated operation variants"
)]
fn generated_operation_and_native_adapter_share_bytes_and_reasons() {
    let input = br#"{"name":"hello\u0021","nested":[null,true,18446744073709551615]}"#;
    let expected = json::decode(input, None).unwrap();
    let mut request = p::CodecApiOperationRequest::default();
    request.operation = Some(p::codec_api_operation_request::Operation::JsonDecode(
        Box::new(p::CodecJsonDecodeRequest {
            encoded: input.to_vec().into(),
            limits: MessageField::default(),
        }),
    ));
    let encoded = zeroize::Zeroizing::new(request.encode_to_vec());
    let result = operation_contract::process_operation_response(&encoded);
    let response = p::CodecApiOperationResponse::decode_from_slice(&result).unwrap();
    let Some(p::codec_api_operation_response::Outcome::Result(result)) = response.outcome.as_ref()
    else {
        panic!("expected operation success")
    };
    let Some(p::codec_api_operation_result::Result::JsonDecode(decoded)) = result.result.as_ref()
    else {
        panic!("expected guarded JSON result")
    };
    assert_eq!(decoded.as_ref(), &expected);
}

#[test]
#[allow(
    clippy::panic,
    reason = "test assertions on generated operation variants"
)]
fn generated_json_failure_has_the_canonical_caller_reason() {
    let mut request = p::CodecApiOperationRequest::default();
    request.operation = Some(p::codec_api_operation_request::Operation::JsonDecode(
        Box::new(p::CodecJsonDecodeRequest {
            encoded: br#"{"x":1,"\u0078":2}"#.to_vec().into(),
            limits: MessageField::default(),
        }),
    ));
    let encoded = zeroize::Zeroizing::new(request.encode_to_vec());
    let result = operation_contract::process_operation_response(&encoded);
    let response = p::CodecApiOperationResponse::decode_from_slice(&result).unwrap();
    let Some(p::codec_api_operation_response::Outcome::Error(error)) = response.outcome.as_ref()
    else {
        panic!("expected operation failure")
    };
    assert_eq!(
        error.origin.as_known(),
        Some(p::CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER)
    );
    let Some(p::codec_api_error::Error::Canonicalization(error)) = error.error.as_ref() else {
        panic!("expected canonical JSON reason")
    };
    assert_eq!(
        error.reason.as_known(),
        Some(p::CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_JSON_DUPLICATE_PROPERTY)
    );
}

#[test]
fn maximum_json_depth_fits_the_existing_codec_wire_reader() {
    // JSON maps expand to the same three wire layers per level as CBOR maps.
    // Keep the new maximum inside the existing transport's depth allowance.
    for depth in [32, 64] {
        let mut input = "null".to_owned();
        for _ in 0..depth {
            input = format!(r#"{{"a":{input}}}"#);
        }
        let limits = p::CodecJsonLimits {
            nesting_depth: Some(64),
            ..Default::default()
        };
        let mut request = p::CodecApiOperationRequest::default();
        request.operation = Some(p::codec_api_operation_request::Operation::JsonDecode(
            Box::new(p::CodecJsonDecodeRequest {
                encoded: input.into_bytes().into(),
                limits: MessageField::some(limits),
            }),
        ));
        let encoded = zeroize::Zeroizing::new(request.encode_to_vec());
        let result = operation_contract::process_operation_response(&encoded);
        let response =
            codec_proto::decode_protobuf::<p::CodecApiOperationResponse>(&result).unwrap();
        assert!(matches!(
            response.outcome.as_ref(),
            Some(p::codec_api_operation_response::Outcome::Result(_))
        ));
    }
}

#[test]
fn generated_json_provider_failures_keep_their_origin() {
    for reason in [
        p::CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_JSON_ALLOCATION_FAILURE,
        p::CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_JSON_ARITHMETIC_OVERFLOW,
    ] {
        let error = codec_proto::CodecWireError::try_new(
            codec_proto::CodecWireErrorBranch::Canonicalization,
            reason,
        )
        .unwrap();
        assert_eq!(error.origin(), codec_proto::CodecWireErrorOrigin::Provider);
    }
}
