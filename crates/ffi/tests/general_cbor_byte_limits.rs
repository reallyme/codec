// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native operation transports admit raised CBOR budgets without widening scalars.
#![allow(
    missing_docs,
    unsafe_code,
    clippy::unwrap_used,
    clippy::panic,
    reason = "bounded caller-owned ABI fixtures"
)]

use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::{decode_json, decode_protobuf, encode_protobuf};
use reallyme_codec_ffi::codec::{
    rm_codec_max_ffi_input_bytes, rm_codec_process_operation, rm_codec_process_operation_json,
};
use zeroize::Zeroizing;

type Process = unsafe extern "C" fn(*const u8, usize, *mut u8, usize, *mut usize) -> i32;

fn call(process: Process, input: &[u8]) -> Zeroizing<Vec<u8>> {
    let mut length = 0;
    // SAFETY: Input remains live and the distinct aligned length is writable;
    // null output with zero capacity is the ABI's documented sizing probe.
    let status = unsafe {
        process(
            input.as_ptr(),
            input.len(),
            core::ptr::null_mut(),
            0,
            &mut length,
        )
    };
    assert_eq!(
        status,
        proto::CodecLegacyAbiStatus::CODEC_LEGACY_ABI_STATUS_BUFFER_TOO_SMALL as i32
    );
    let mut output = Zeroizing::new(vec![0; length]);
    // SAFETY: Input, output, and length storage are distinct live allocations;
    // the second call uses the complete capacity returned by the sizing probe.
    let status = unsafe {
        process(
            input.as_ptr(),
            input.len(),
            output.as_mut_ptr(),
            output.len(),
            &mut length,
        )
    };
    assert_eq!(
        status,
        proto::CodecLegacyAbiStatus::CODEC_LEGACY_ABI_STATUS_UNSPECIFIED as i32
    );
    assert_eq!(length, output.len());
    output
}

#[test]
fn two_mib_general_cbor_passes_native_binary_and_json_operation_boundaries() {
    const MAX_BYTES: usize = 2_097_152;
    assert_eq!(rm_codec_max_ffi_input_bytes(), 1_048_576);
    let mut encoded = Zeroizing::new(vec![0x5a]);
    encoded.extend_from_slice(
        &u32::try_from(MAX_BYTES.checked_sub(5).unwrap())
            .unwrap()
            .to_be_bytes(),
    );
    encoded.resize(MAX_BYTES, 0xa5);
    let base64 = Zeroizing::new(codec_runtime::base64::bytes_to_base64(&encoded));
    let mut json = Zeroizing::new(String::from(
        "{\"generalCborValidateExact\":{\"encoded\":\"",
    ));
    json.push_str(&base64);
    json.push_str("\",\"options\":{\"serialization\":1,\"lengths\":1,\"floats\":1,\"ordering\":1,\"inputBytes\":\"2097152\",\"byteStringBytes\":\"2097152\"}}}");
    let request: proto::CodecApiOperationRequest = decode_json(json.as_bytes()).unwrap();
    let binary = encode_protobuf(&request).unwrap();
    let response = call(rm_codec_process_operation, &binary);
    let json_response = call(rm_codec_process_operation_json, json.as_bytes());
    assert_eq!(response, json_response);
    let response: proto::CodecApiOperationResponse = decode_protobuf(&response).unwrap();
    let Some(proto::codec_api_operation_response::Outcome::Result(result)) =
        response.outcome.as_ref()
    else {
        panic!("expected successful native validation");
    };
    let Some(proto::codec_api_operation_result::Result::GeneralCborValidateExact(frame)) =
        result.result.as_ref()
    else {
        panic!("expected native frame result");
    };
    assert_eq!(
        frame.received.as_option().unwrap().length,
        u64::try_from(MAX_BYTES).unwrap()
    );
    assert_eq!(
        frame.options.as_option().unwrap().input_bytes,
        Some(u64::try_from(MAX_BYTES).unwrap())
    );
}
