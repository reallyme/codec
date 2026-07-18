// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use super::super::{rm_codec_process_operation, rm_codec_process_operation_json};
use super::assert_generated_error;
use crate::status::{CODEC_BUFFER_TOO_SMALL, CODEC_INVALID_ARGUMENT, CODEC_OK};
use codec_proto::generated::proto::reallyme::codec::v1::{
    __buffa::oneof::codec_operation_request::Operation as CodecOperation, codec_operation_response,
    codec_operation_result, CodecErrorOrigin, CodecErrorReason,
    CodecMulticodecPrefixForNameRequest, CodecMultikeyParseRequest, CodecOperationRequest,
    CodecOperationResponse,
};
use codec_proto::{decode_protobuf, encode_protobuf, CodecWireErrorBranch};

#[test]
fn operation_ffi_exports_return_resource_limit_responses_for_oversized_input() {
    type ProcessOperation =
        unsafe extern "C" fn(*const u8, usize, *mut u8, usize, *mut usize) -> i32;
    let cases = [
        (
            rm_codec_process_operation as ProcessOperation,
            codec_proto::MAX_CODEC_PROTO_MESSAGE_BYTES,
        ),
        (
            rm_codec_process_operation_json as ProcessOperation,
            codec_proto::MAX_CODEC_PROTO_JSON_BYTES,
        ),
    ];

    for (process, limit) in cases {
        let oversized = vec![0_u8; limit + 1];
        let mut produced_len = 0_usize;
        // SAFETY: The input pointer covers the entire caller-owned buffer,
        // and the null output is valid for a zero-length first-pass query.
        let status = unsafe {
            process(
                oversized.as_ptr(),
                oversized.len(),
                core::ptr::null_mut(),
                0,
                &mut produced_len,
            )
        };

        assert_eq!(status, CODEC_BUFFER_TOO_SMALL);
        assert!(produced_len > 0);

        let mut output = vec![0_u8; produced_len];
        // SAFETY: The input, output, and produced-length storage are
        // distinct caller-owned allocations valid for this call.
        let status = unsafe {
            process(
                oversized.as_ptr(),
                oversized.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut produced_len,
            )
        };
        assert_eq!(status, CODEC_OK);
        output.truncate(produced_len);

        assert_generated_error(
            &output,
            CodecWireErrorBranch::Boundary,
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
            CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER,
        );
    }
}

#[test]
fn operation_ffi_exports_reject_inputs_above_bounded_sentinel() {
    let oversized = vec![0_u8; codec_proto::MAX_CODEC_PROTO_JSON_BYTES + 2];

    for process in [rm_codec_process_operation, rm_codec_process_operation_json] {
        let mut produced_len = 0_usize;
        // SAFETY: The input pointer covers the caller-owned buffer and the
        // null output is valid for a zero-length first-pass query.
        let status = unsafe {
            process(
                oversized.as_ptr(),
                oversized.len(),
                core::ptr::null_mut(),
                0,
                &mut produced_len,
            )
        };
        assert_eq!(status, CODEC_INVALID_ARGUMENT);
        assert_eq!(produced_len, 0);
    }
}

#[test]
fn operation_error_is_carried_in_the_discriminated_response() {
    let request = CodecOperationRequest {
        operation: Some(CodecOperation::MultikeyParse(Box::new(
            CodecMultikeyParseRequest {
                multikey: "not-a-key".to_owned(),
                __buffa_unknown_fields: Default::default(),
            },
        ))),
        __buffa_unknown_fields: Default::default(),
    };
    let input = encode_protobuf(&request);
    let mut produced_len = 0_usize;

    // SAFETY: The input vector and produced-length output are valid for
    // the duration of this call. No output byte buffer is supplied.
    let probe_status = unsafe {
        rm_codec_process_operation(
            input.as_ptr(),
            input.len(),
            core::ptr::null_mut(),
            0,
            &mut produced_len,
        )
    };
    assert_eq!(probe_status, CODEC_BUFFER_TOO_SMALL);
    assert!(produced_len > 0);

    let mut output = vec![0_u8; produced_len];
    // SAFETY: All pointers describe valid caller-owned storage for the
    // duration of this call.
    let status = unsafe {
        rm_codec_process_operation(
            input.as_ptr(),
            input.len(),
            output.as_mut_ptr(),
            output.len(),
            &mut produced_len,
        )
    };

    assert_eq!(status, CODEC_OK);
    assert!(produced_len > 0);
    output.truncate(produced_len);
    assert_generated_error(
        &output,
        CodecWireErrorBranch::Multiformat,
        CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTIKEY,
        CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER,
    );
}

#[test]
fn operation_ffi_returns_a_fully_discriminated_generated_response() {
    let request = CodecOperationRequest {
        operation: Some(
            CodecMulticodecPrefixForNameRequest {
                name: "ed25519-pub".to_owned(),
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    };
    let input = encode_protobuf(&request);
    let mut produced_len = 0_usize;

    // SAFETY: The input and produced-length storage remain valid for this
    // sizing call; a null output pointer is valid with zero capacity.
    let probe_status = unsafe {
        rm_codec_process_operation(
            input.as_ptr(),
            input.len(),
            core::ptr::null_mut(),
            0,
            &mut produced_len,
        )
    };
    assert_eq!(probe_status, CODEC_BUFFER_TOO_SMALL);
    assert!(produced_len > 0);

    let mut output = vec![0_u8; produced_len];
    // SAFETY: Input, output, and length storage are distinct valid
    // caller-owned allocations for the duration of the call.
    let status = unsafe {
        rm_codec_process_operation(
            input.as_ptr(),
            input.len(),
            output.as_mut_ptr(),
            output.len(),
            &mut produced_len,
        )
    };
    assert_eq!(status, CODEC_OK);
    output.truncate(produced_len);

    let response = decode_protobuf::<CodecOperationResponse>(&output);
    assert!(response.is_ok());
    let Some(response) = response.ok() else {
        return;
    };
    let Some(codec_operation_response::Outcome::Result(result)) = response.outcome.as_ref() else {
        return;
    };
    assert!(matches!(
        result.result.as_ref(),
        Some(codec_operation_result::Result::MulticodecPrefixForName(_))
    ));
}

#[test]
fn generated_proto_json_returns_the_same_discriminated_response() {
    let input = br#"{"multikeyParse":{"multikey":"not-a-key"}}"#;
    let mut produced_len = 0_usize;

    // SAFETY: The input and produced-length storage remain valid for the
    // duration of the sizing call.
    let probe_status = unsafe {
        rm_codec_process_operation_json(
            input.as_ptr(),
            input.len(),
            core::ptr::null_mut(),
            0,
            &mut produced_len,
        )
    };
    assert_eq!(probe_status, CODEC_BUFFER_TOO_SMALL);
    assert!(produced_len > 0);

    let mut output = vec![0_u8; produced_len];
    // SAFETY: All pointers describe valid, non-overlapping caller-owned
    // storage for the duration of the call.
    let status = unsafe {
        rm_codec_process_operation_json(
            input.as_ptr(),
            input.len(),
            output.as_mut_ptr(),
            output.len(),
            &mut produced_len,
        )
    };

    assert_eq!(status, CODEC_OK);
    output.truncate(produced_len);
    assert_generated_error(
        &output,
        CodecWireErrorBranch::Multiformat,
        CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTIKEY,
        CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER,
    );
}
