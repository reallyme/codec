// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuzz the executable protobuf and generated ProtoJSON dispatch boundaries.

#![no_main]

use codec_proto::generated::proto::reallyme::codec::v1::CodecOperationResponse;
use codec_proto::{decode_protobuf, MAX_CODEC_PROTO_MESSAGE_BYTES};
use libfuzzer_sys::fuzz_target;
use reallyme_codec::operation_contract::{
    process_operation_response, process_operation_response_json,
};

fuzz_target!(|data: &[u8]| {
    let binary_response = process_operation_response(data);
    assert!(binary_response.len() <= MAX_CODEC_PROTO_MESSAGE_BYTES);
    assert!(
        matches!(decode_protobuf::<CodecOperationResponse>(&binary_response), Ok(response) if response.outcome.is_some())
    );

    // ProtoJSON accepts bytes directly and must reject invalid UTF-8 and JSON
    // through the same generated response channel without panicking.
    let json_response = process_operation_response_json(data);
    assert!(json_response.len() <= MAX_CODEC_PROTO_MESSAGE_BYTES);
    assert!(
        matches!(decode_protobuf::<CodecOperationResponse>(&json_response), Ok(response) if response.outcome.is_some())
    );
});
