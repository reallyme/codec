// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuzz the executable protobuf and generated ProtoJSON dispatch boundaries.

#![no_main]

use codec_proto::generated::proto::reallyme::codec::v1::codec_api_operation_response::Outcome;
use codec_proto::generated::proto::reallyme::codec::v1::CodecApiOperationResponse;
use codec_proto::{decode_protobuf, MAX_CODEC_PROTO_MESSAGE_BYTES};
use libfuzzer_sys::fuzz_target;
use reallyme_codec::operation_contract::{
    process_operation_response, process_operation_response_json,
};

fuzz_target!(|data: &[u8]| {
    let binary_response = process_operation_response(data);
    assert_discriminated_response(&binary_response);

    // ProtoJSON accepts bytes directly and must reject invalid UTF-8 and JSON
    // through the same generated response channel without panicking.
    let json_response = process_operation_response_json(data);
    assert_discriminated_response(&json_response);
});

fn assert_discriminated_response(bytes: &[u8]) {
    assert!(bytes.len() <= MAX_CODEC_PROTO_MESSAGE_BYTES);
    // Strict decoding checks the response against all operation and error
    // branches that the dispatchers can emit, including provider metadata.
    let response = decode_protobuf::<CodecApiOperationResponse>(bytes)
        .expect("dispatcher returned an invalid protobuf response");
    match response.outcome.as_ref() {
        Some(Outcome::Result(result)) => assert!(result.result.is_some()),
        Some(Outcome::Error(error)) => assert!(error.error.is_some()),
        None => panic!("dispatcher returned a response without an outcome"),
    }
}
