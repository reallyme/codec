// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use super::{
    rm_codec_abi_version, rm_codec_max_ffi_input_bytes, rm_codec_max_ffi_output_bytes,
    rm_codec_max_operation_response_bytes, rm_codec_process, rm_codec_process_bool,
    validate_boundary_input_lengths, CODEC_ABI_VERSION, CODEC_BASE58BTC_DECODE,
    CODEC_BASE58BTC_ENCODE, CODEC_BASE64_ENCODE, CODEC_CANONICALIZE_JSON,
    CODEC_DAG_CBOR_VERIFY_CID, CODEC_MULTICODEC_LOOKUP_PREFIX, CODEC_MULTICODEC_PREFIX_FOR_NAME,
    CODEC_MULTICODEC_TABLE, CODEC_MULTIKEY_PARSE, CODEC_PEM_DECODE, CODEC_PEM_ENCODE,
    MAX_CODEC_FFI_INPUT_BYTES, MAX_CODEC_FFI_OUTPUT_BYTES,
};
use crate::status::{CODEC_BUFFER_TOO_SMALL, CODEC_INVALID_ARGUMENT, CODEC_OK};
use codec_proto::generated::proto::reallyme::codec::v1::CodecErrorReason;
use codec_proto::generated::proto::reallyme::codec::v1::{
    __buffa::oneof::codec_error, codec_operation_response, CodecErrorOrigin, CodecOperationResponse,
};
use codec_proto::{decode_protobuf, CodecWireErrorBranch};
use codec_runtime::scalar_ops::MAX_BASE58BTC_INPUT_BYTES;

mod operation;

const RETIRED_CODEC_DAG_CBOR_ENCODE: u32 = 19;
const RETIRED_CODEC_DAG_CBOR_DECODE: u32 = 20;

fn generated_error(
    response_bytes: &[u8],
) -> Option<(CodecWireErrorBranch, CodecErrorReason, CodecErrorOrigin)> {
    let response = decode_protobuf::<CodecOperationResponse>(response_bytes).ok()?;
    let Some(codec_operation_response::Outcome::Error(error)) = response.outcome.as_ref() else {
        return None;
    };
    let (branch, reason) = match error.error.as_ref()? {
        codec_error::Error::BaseEncoding(value) => {
            (CodecWireErrorBranch::BaseEncoding, value.reason.as_known())
        }
        codec_error::Error::Pem(value) => (CodecWireErrorBranch::Pem, value.reason.as_known()),
        codec_error::Error::Multiformat(value) => {
            (CodecWireErrorBranch::Multiformat, value.reason.as_known())
        }
        codec_error::Error::Canonicalization(value) => (
            CodecWireErrorBranch::Canonicalization,
            value.reason.as_known(),
        ),
        codec_error::Error::Backend(value) => {
            (CodecWireErrorBranch::Backend, value.reason.as_known())
        }
        codec_error::Error::Boundary(value) => {
            (CodecWireErrorBranch::Boundary, value.reason.as_known())
        }
    };
    Some((branch, reason?, error.origin.as_known()?))
}

fn assert_generated_error(
    response_bytes: &[u8],
    expected_branch: CodecWireErrorBranch,
    expected_reason: CodecErrorReason,
    expected_origin: CodecErrorOrigin,
) {
    assert_eq!(
        generated_error(response_bytes),
        Some((expected_branch, expected_reason, expected_origin))
    );
}

#[test]
fn abi_version_export_matches_the_sdk_contract() {
    assert_eq!(rm_codec_abi_version(), CODEC_ABI_VERSION);
    assert_eq!(
        rm_codec_max_operation_response_bytes(),
        codec_proto::MAX_CODEC_PROTO_MESSAGE_BYTES
    );
    assert_eq!(rm_codec_max_ffi_input_bytes(), MAX_CODEC_FFI_INPUT_BYTES);
    assert_eq!(rm_codec_max_ffi_output_bytes(), MAX_CODEC_FFI_OUTPUT_BYTES);
}

#[test]
fn retired_scalar_structured_json_ids_fail_closed() {
    for operation in [
        CODEC_MULTICODEC_PREFIX_FOR_NAME,
        CODEC_MULTICODEC_LOOKUP_PREFIX,
        CODEC_MULTICODEC_TABLE,
        CODEC_MULTIKEY_PARSE,
        RETIRED_CODEC_DAG_CBOR_ENCODE,
        RETIRED_CODEC_DAG_CBOR_DECODE,
        CODEC_DAG_CBOR_VERIFY_CID,
        CODEC_PEM_DECODE,
        CODEC_PEM_ENCODE,
    ] {
        let mut produced_len = usize::MAX;
        // SAFETY: The produced-length storage remains valid for the call.
        // Retired structured scalar IDs fail before reading inputs or
        // producing any JSON payload.
        let status = unsafe {
            rm_codec_process(
                operation,
                core::ptr::null(),
                0,
                core::ptr::null(),
                0,
                core::ptr::null(),
                0,
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
fn aggregate_ffi_limit_rejects_oversize_and_integer_overflow() {
    assert!(validate_boundary_input_lengths(&[MAX_CODEC_FFI_INPUT_BYTES]).is_ok());
    assert_eq!(
        validate_boundary_input_lengths(&[MAX_CODEC_FFI_INPUT_BYTES, 1]),
        Err(CODEC_INVALID_ARGUMENT)
    );
    assert_eq!(
        validate_boundary_input_lengths(&[usize::MAX, 1]),
        Err(CODEC_INVALID_ARGUMENT)
    );
}

#[test]
fn base58_decode_rejects_inputs_above_ffi_cap() {
    let input = vec![b'1'; MAX_BASE58BTC_INPUT_BYTES + 1];
    let mut produced_len = 0_usize;

    // SAFETY: The input vector and produced-length output are valid for
    // the duration of this call. No output byte buffer is supplied.
    let status = unsafe {
        rm_codec_process(
            CODEC_BASE58BTC_DECODE,
            input.as_ptr(),
            input.len(),
            core::ptr::null(),
            0,
            core::ptr::null(),
            0,
            core::ptr::null_mut(),
            0,
            &mut produced_len,
        )
    };

    assert_eq!(status, CODEC_INVALID_ARGUMENT);
    assert_eq!(produced_len, 0);
}

#[test]
fn base58_encode_rejects_inputs_above_ffi_cap() {
    let input = vec![0_u8; MAX_BASE58BTC_INPUT_BYTES + 1];
    let mut produced_len = 0_usize;

    // SAFETY: The input vector and produced-length output are valid for
    // the duration of this call. No output byte buffer is supplied.
    let status = unsafe {
        rm_codec_process(
            CODEC_BASE58BTC_ENCODE,
            input.as_ptr(),
            input.len(),
            core::ptr::null(),
            0,
            core::ptr::null(),
            0,
            core::ptr::null_mut(),
            0,
            &mut produced_len,
        )
    };

    assert_eq!(status, CODEC_INVALID_ARGUMENT);
    assert_eq!(produced_len, 0);
}

#[test]
fn generic_ffi_rejects_oversized_ignored_arguments_before_dispatch() {
    let oversized = vec![0_u8; MAX_CODEC_FFI_INPUT_BYTES + 1];
    let input = b"abc";
    let mut produced_len = 0_usize;

    // SAFETY: Every pointer describes valid caller-owned storage. The
    // third argument is deliberately oversized and must be rejected even
    // though the selected operation would otherwise ignore it.
    let status = unsafe {
        rm_codec_process(
            CODEC_BASE64_ENCODE,
            input.as_ptr(),
            input.len(),
            core::ptr::null(),
            0,
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

#[test]
fn canonicalization_boundaries_reject_ambiguous_object_keys() {
    let duplicate_json = br#"{"a":1,"a":2}"#;
    let mut produced_len = 0_usize;

    // SAFETY: The input and produced-length storage remain valid for the
    // duration of the call. No output byte buffer is supplied because the
    // operation must reject this ambiguous JSON text before encoding.
    let jcs_status = unsafe {
        rm_codec_process(
            CODEC_CANONICALIZE_JSON,
            duplicate_json.as_ptr(),
            duplicate_json.len(),
            core::ptr::null(),
            0,
            core::ptr::null(),
            0,
            core::ptr::null_mut(),
            0,
            &mut produced_len,
        )
    };
    assert_eq!(jcs_status, CODEC_INVALID_ARGUMENT);
    assert_eq!(produced_len, 0);
}

#[test]
fn process_allows_input_output_alias_after_result_is_copied() {
    let mut buffer = *b"abc\0";
    let mut produced_len = 0_usize;

    // SAFETY: The input and output ranges are valid for the call. This
    // deliberately aliases input and output to cover the documented
    // copy-then-write invariant.
    let status = unsafe {
        rm_codec_process(
            CODEC_BASE64_ENCODE,
            buffer.as_ptr(),
            3,
            core::ptr::null(),
            0,
            core::ptr::null(),
            0,
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut produced_len,
        )
    };

    assert_eq!(status, CODEC_OK);
    assert_eq!(produced_len, 4);
    assert_eq!(&buffer, b"YWJj");
}

#[test]
fn process_reports_length_before_rejecting_short_output_buffer() {
    let input = b"abc";
    let mut output = [0_u8; 3];
    let mut produced_len = 0_usize;

    // SAFETY: All pointers describe valid caller-owned storage for the
    // duration of this call.
    let status = unsafe {
        rm_codec_process(
            CODEC_BASE64_ENCODE,
            input.as_ptr(),
            input.len(),
            core::ptr::null(),
            0,
            core::ptr::null(),
            0,
            output.as_mut_ptr(),
            output.len(),
            &mut produced_len,
        )
    };

    assert_eq!(status, CODEC_BUFFER_TOO_SMALL);
    assert_eq!(produced_len, 4);
    assert_eq!(output, [0_u8; 3]);
}

#[test]
fn failure_paths_initialize_scalar_out_parameters() {
    let mut produced_len = usize::MAX;
    // SAFETY: `produced_len` is valid writable storage. The deliberately
    // invalid operation has no input or byte output ranges to validate.
    let status = unsafe {
        rm_codec_process(
            u32::MAX,
            core::ptr::null(),
            0,
            core::ptr::null(),
            0,
            core::ptr::null(),
            0,
            core::ptr::null_mut(),
            0,
            &mut produced_len,
        )
    };
    assert_eq!(status, CODEC_INVALID_ARGUMENT);
    assert_eq!(produced_len, 0);

    let mut result = i32::MAX;
    // SAFETY: `result` is valid writable storage and both empty input
    // ranges satisfy the predicate boundary operation contract.
    let status = unsafe {
        rm_codec_process_bool(
            u32::MAX,
            core::ptr::null(),
            0,
            core::ptr::null(),
            0,
            &mut result,
        )
    };
    assert_eq!(status, CODEC_INVALID_ARGUMENT);
    assert_eq!(result, 0);
}
