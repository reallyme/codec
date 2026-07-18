// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_runtime::operation_contract::{
    process_operation_response, process_operation_response_json,
};

use crate::guard::ffi_guard;
use crate::pointer::read_slice;
use crate::status::{CodecStatus, CODEC_OK};

use super::{initialize_output_length, validate_proto_boundary_input_length, write_output};

/// Returns the authoritative maximum encoded operation-response size.
///
/// Native SDKs query this value only after validating
/// [`CODEC_ABI_VERSION`](super::CODEC_ABI_VERSION),
/// removing hardcoded cross-language copies of the protocol allocation bound.
#[no_mangle]
pub extern "C" fn rm_codec_max_operation_response_bytes() -> usize {
    codec_proto::MAX_CODEC_PROTO_MESSAGE_BYTES
}

/// Executes one self-describing protobuf request and returns a fully
/// discriminated `CodecOperationResponse`.
///
/// The C status reports only ABI success or failure. Operation success and
/// typed codec failures are represented exclusively by the response oneof.
///
/// # Safety
///
/// The pointer, length, output, aliasing, and ownership requirements are
/// identical to [`super::rm_codec_process`].
#[no_mangle]
pub unsafe extern "C" fn rm_codec_process_operation(
    request_ptr: *const u8,
    request_len: usize,
    output_ptr: *mut u8,
    output_len: usize,
    len_out: *mut usize,
) -> CodecStatus {
    ffi_guard(|| {
        process_operation_boundary(
            request_ptr,
            request_len,
            output_ptr,
            output_len,
            len_out,
            codec_proto::MAX_CODEC_PROTO_MESSAGE_BYTES,
            process_operation_response,
        )
    })
}

/// Executes one generated ProtoJSON request and returns a fully discriminated
/// binary `CodecOperationResponse`.
///
/// JSON is an input representation only. Binary protobuf and generated
/// ProtoJSON select the same semantic operation and return the same response
/// type, so no parallel JSON result operation contract exists.
///
/// # Safety
///
/// The pointer, length, output, aliasing, and ownership requirements are
/// identical to [`rm_codec_process_operation`].
#[no_mangle]
pub unsafe extern "C" fn rm_codec_process_operation_json(
    request_ptr: *const u8,
    request_len: usize,
    output_ptr: *mut u8,
    output_len: usize,
    len_out: *mut usize,
) -> CodecStatus {
    ffi_guard(|| {
        process_operation_boundary(
            request_ptr,
            request_len,
            output_ptr,
            output_len,
            len_out,
            codec_proto::MAX_CODEC_PROTO_JSON_BYTES,
            process_operation_response_json,
        )
    })
}

fn process_operation_boundary(
    request_ptr: *const u8,
    request_len: usize,
    output_ptr: *mut u8,
    output_len: usize,
    len_out: *mut usize,
    max_request_len: usize,
    process_request: fn(&[u8]) -> zeroize::Zeroizing<Vec<u8>>,
) -> CodecStatus {
    let output_status = initialize_output_length(output_ptr, output_len, len_out);
    if output_status != CODEC_OK {
        return output_status;
    }
    if let Err(status) = validate_proto_boundary_input_length(request_len, max_request_len) {
        return status;
    }
    // SAFETY: The request follows the caller-owned pointer/length operation
    // contract documented by the exported operation functions.
    let request = match unsafe { read_slice(request_ptr, request_len) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    write_output(output_ptr, output_len, len_out, process_request(request))
}
