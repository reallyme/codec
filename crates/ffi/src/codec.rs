// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_runtime::multicodec::{
    prefix_for_name as multicodec_prefix_for_name, strip_prefix as multicodec_strip_prefix,
    MulticodecOperationError,
};
use codec_runtime::scalar_ops::{
    binding_matches_codec, canonicalize_json, compute_dag_cbor_cid, dag_cbor_codec_code,
    dag_cbor_content_hash, dag_cbor_multihash_value, decode_base58btc, decode_base64,
    decode_base64url, decode_lower_hex, decode_multibase, encode_base58btc, encode_base64,
    encode_base64url, encode_lower_hex, encode_multibase_base58btc, encode_multibase_base64url,
    encode_multikey, parse_cid, parse_multikey_value, valid_cid, validate_binding,
};
use zeroize::{Zeroize, Zeroizing};

use crate::guard::ffi_guard;
use crate::pointer::{read_slice, validate_output_len_pair, write_i32, write_len, write_slice};
use crate::status::{
    CodecStatus, CODEC_BUFFER_TOO_SMALL, CODEC_INTERNAL_ERROR, CODEC_INVALID_ARGUMENT, CODEC_OK,
};

/// Maximum aggregate caller-controlled input accepted by the generic C ABI.
///
/// This check runs before decoding or copying so every operation, including
/// future dispatch additions, inherits a finite allocation budget.
pub(crate) const MAX_CODEC_FFI_INPUT_BYTES: usize = 1024 * 1024;

/// Maximum generic result accepted by SDK adapters after an FFI size probe.
///
/// Structured outputs can expand substantially at SDK boundaries, so this is
/// deliberately larger than the input budget. Keeping it finite prevents a
/// corrupted or mismatched provider from turning the two-pass ABI into an
/// unbounded allocation request in Swift or JNI callers.
pub(crate) const MAX_CODEC_FFI_OUTPUT_BYTES: usize = 64 * 1024 * 1024;

/// Version of the exported C function signatures and calling conventions.
///
/// SDKs must reject a library that does not expose this exact value before
/// casting or calling any other dynamically resolved symbol.
pub const CODEC_ABI_VERSION: u32 = 5;

const CODEC_BASE64_ENCODE: u32 = 1;
const CODEC_BASE64_DECODE: u32 = 2;
const CODEC_BASE64URL_ENCODE: u32 = 3;
const CODEC_BASE64URL_DECODE: u32 = 4;
const CODEC_LOWER_HEX_ENCODE: u32 = 5;
const CODEC_LOWER_HEX_DECODE: u32 = 6;
const CODEC_BASE58BTC_ENCODE: u32 = 7;
const CODEC_BASE58BTC_DECODE: u32 = 8;
const CODEC_MULTIBASE_BASE58BTC_ENCODE: u32 = 9;
const CODEC_MULTIBASE_BASE64URL_ENCODE: u32 = 10;
const CODEC_MULTIBASE_DECODE: u32 = 11;
const CODEC_MULTICODEC_PREFIX_FOR_NAME: u32 = 12;
const CODEC_MULTICODEC_LOOKUP_PREFIX: u32 = 13;
const CODEC_MULTICODEC_STRIP_PREFIX: u32 = 14;
const CODEC_MULTICODEC_TABLE: u32 = 15;
const CODEC_MULTIKEY_ENCODE: u32 = 16;
const CODEC_MULTIKEY_PARSE: u32 = 17;
const CODEC_REQUIRE_SUPPORTED_MULTICODEC: u32 = 18;
const CODEC_DAG_CBOR_COMPUTE_CID: u32 = 21;
const CODEC_DAG_CBOR_VERIFY_CID: u32 = 22;
const CODEC_DAG_CBOR_SHA256_CONTENT_HASH: u32 = 23;
const CODEC_DAG_CBOR_MULTIHASH: u32 = 24;
const CODEC_TRY_PARSE_CID: u32 = 25;
const CODEC_DAG_CBOR_CODEC_CODE: u32 = 26;
const CODEC_CANONICALIZE_JSON: u32 = 27;
const CODEC_PEM_DECODE: u32 = 28;
const CODEC_PEM_ENCODE: u32 = 29;
const CODEC_VALIDATE_KEY_BINDING: u32 = 30;

const CODEC_BOOL_BINDING_TYPE_MATCHES_CODEC: u32 = 1;
const CODEC_BOOL_IS_VALID_CID_STRING: u32 = 2;

type CodecOutput = Zeroizing<Vec<u8>>;

mod operation;
pub use operation::{
    rm_codec_max_operation_response_bytes, rm_codec_process_operation,
    rm_codec_process_operation_json,
};

/// Returns the exact C ABI operation contract version implemented by this library.
///
/// This leaf function only returns a compile-time constant and therefore has
/// no operation that can panic or unwind across the C boundary.
#[no_mangle]
pub extern "C" fn rm_codec_abi_version() -> u32 {
    CODEC_ABI_VERSION
}

/// Returns the authoritative generic C ABI aggregate input limit.
///
/// Platform SDKs use this value after ABI-version validation so caller-side
/// fail-fast checks cannot drift from the Rust boundary.
#[no_mangle]
pub extern "C" fn rm_codec_max_ffi_input_bytes() -> usize {
    MAX_CODEC_FFI_INPUT_BYTES
}

/// Returns the authoritative generic C ABI output allocation limit.
///
/// Two-pass SDK callers reject provider-reported lengths above this value
/// before allocating managed arrays.
#[no_mangle]
pub extern "C" fn rm_codec_max_ffi_output_bytes() -> usize {
    MAX_CODEC_FFI_OUTPUT_BYTES
}

fn read_text<'a>(ptr: *const u8, len: usize) -> Result<&'a str, CodecStatus> {
    // SAFETY: All exported ABI entry points define input pointers as borrowed
    // `(ptr, len)` byte ranges owned by the caller for the duration of the
    // call. `read_slice` validates null and oversized ranges before borrowing.
    let bytes = unsafe { read_slice(ptr, len) }?;
    core::str::from_utf8(bytes).map_err(|_| CODEC_INVALID_ARGUMENT)
}

fn validate_boundary_input_lengths(lengths: &[usize]) -> Result<(), CodecStatus> {
    let mut aggregate = 0_usize;
    for length in lengths {
        aggregate = aggregate
            .checked_add(*length)
            .ok_or(CODEC_INVALID_ARGUMENT)?;
        if aggregate > MAX_CODEC_FFI_INPUT_BYTES {
            return Err(CODEC_INVALID_ARGUMENT);
        }
    }
    Ok(())
}

fn validate_proto_boundary_input_length(
    request_len: usize,
    maximum: usize,
) -> Result<(), CodecStatus> {
    // Native SDKs use one bounded byte beyond the transport limit as a
    // sentinel so the core can return its stable resource-limit envelope.
    let sentinel_maximum = maximum.checked_add(1).ok_or(CODEC_INVALID_ARGUMENT)?;
    if request_len > sentinel_maximum {
        return Err(CODEC_INVALID_ARGUMENT);
    }
    Ok(())
}

fn empty_or_text<'a>(ptr: *const u8, len: usize) -> Result<&'a str, CodecStatus> {
    read_text(ptr, len)
}

fn write_output(
    output_ptr: *mut u8,
    output_len: usize,
    len_out: *mut usize,
    mut bytes: CodecOutput,
) -> CodecStatus {
    if bytes.len() > MAX_CODEC_FFI_OUTPUT_BYTES {
        bytes.zeroize();
        return CODEC_INTERNAL_ERROR;
    }
    let status = validate_output_len_pair(output_ptr, output_len, len_out);
    if status != CODEC_OK {
        return status;
    }
    let produced_len = bytes.len();
    // SAFETY: `validate_output_len_pair` rejected null, misaligned, and
    // overlapping produced-length storage before this write.
    let len_status = unsafe { write_len(len_out, produced_len) };
    if len_status != CODEC_OK {
        return len_status;
    }
    if output_len < produced_len {
        bytes.zeroize();
        return CODEC_BUFFER_TOO_SMALL;
    }
    // SAFETY: `validate_output_len_pair` checked the output byte range and its
    // disjointness from `len_out`; `bytes` is an immutable Rust slice and the
    // subsequent copy writes only `bytes.len()` initialized bytes.
    let Ok(output) = (unsafe { write_slice(output_ptr, output_len) }) else {
        return CODEC_INVALID_ARGUMENT;
    };
    output[..produced_len].copy_from_slice(&bytes);
    bytes.zeroize();
    CODEC_OK
}

fn initialize_output_length(
    output_ptr: *mut u8,
    output_len: usize,
    len_out: *mut usize,
) -> CodecStatus {
    let status = validate_output_len_pair(output_ptr, output_len, len_out);
    if status != CODEC_OK {
        return status;
    }
    // SAFETY: The shared pair validator rejected a null, misaligned, or
    // overlapping produced-length pointer before this initialization.
    unsafe { write_len(len_out, 0) }
}

fn output_bytes(value: Vec<u8>) -> CodecOutput {
    Zeroizing::new(value)
}

fn text_bytes(value: String) -> CodecOutput {
    output_bytes(value.into_bytes())
}

fn multicodec_status(error: MulticodecOperationError) -> CodecStatus {
    match error {
        MulticodecOperationError::UnknownName | MulticodecOperationError::InvalidPrefix => {
            CODEC_INVALID_ARGUMENT
        }
        MulticodecOperationError::RegistryInvariant
        | MulticodecOperationError::AllocationFailure => CODEC_INTERNAL_ERROR,
        _ => CODEC_INTERNAL_ERROR,
    }
}

fn process(
    operation: u32,
    first_ptr: *const u8,
    first_len: usize,
    second_ptr: *const u8,
    second_len: usize,
    third_ptr: *const u8,
    third_len: usize,
) -> Result<CodecOutput, CodecStatus> {
    validate_boundary_input_lengths(&[first_len, second_len, third_len])?;
    // SAFETY: The C ABI operation contract supplies each input as a caller-owned byte
    // range valid for the duration of this call. `read_slice` validates null
    // pointers and impossible lengths before constructing borrowed slices.
    let first = unsafe { read_slice(first_ptr, first_len) }?;
    // SAFETY: Same ABI input operation contract and validation as `first`.
    let second = unsafe { read_slice(second_ptr, second_len) }?;
    // SAFETY: Same ABI input operation contract and validation as `first`.
    let _third = unsafe { read_slice(third_ptr, third_len) }?;
    match operation {
        CODEC_BASE64_ENCODE => Ok(text_bytes(encode_base64(first))),
        CODEC_BASE64_DECODE => {
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            decode_base64(text)
                .map(output_bytes)
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_BASE64URL_ENCODE => Ok(text_bytes(encode_base64url(first))),
        CODEC_BASE64URL_DECODE => {
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            decode_base64url(text)
                .map(output_bytes)
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_LOWER_HEX_ENCODE => Ok(text_bytes(encode_lower_hex(first))),
        CODEC_LOWER_HEX_DECODE => {
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            decode_lower_hex(text)
                .map(output_bytes)
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_BASE58BTC_ENCODE => encode_base58btc(first)
            .map(text_bytes)
            .map_err(|_| CODEC_INVALID_ARGUMENT),
        CODEC_BASE58BTC_DECODE => {
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            decode_base58btc(text)
                .map(output_bytes)
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_MULTIBASE_BASE58BTC_ENCODE => encode_multibase_base58btc(first)
            .map(text_bytes)
            .map_err(|_| CODEC_INVALID_ARGUMENT),
        CODEC_MULTIBASE_BASE64URL_ENCODE => encode_multibase_base64url(first)
            .map(text_bytes)
            .map_err(|_| CODEC_INVALID_ARGUMENT),
        CODEC_MULTIBASE_DECODE => {
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            decode_multibase(text)
                .map(output_bytes)
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_MULTICODEC_PREFIX_FOR_NAME
        | CODEC_MULTICODEC_LOOKUP_PREFIX
        | CODEC_MULTICODEC_TABLE
        | CODEC_MULTIKEY_PARSE
        | CODEC_DAG_CBOR_VERIFY_CID
        | CODEC_PEM_DECODE
        | CODEC_PEM_ENCODE => Err(CODEC_INVALID_ARGUMENT),
        CODEC_MULTICODEC_STRIP_PREFIX => {
            let stripped = multicodec_strip_prefix(first).map_err(multicodec_status)?;
            Ok(output_bytes(stripped.to_vec()))
        }
        CODEC_MULTIKEY_ENCODE => {
            let codec_name = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            encode_multikey(codec_name, second)
                .map(text_bytes)
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_REQUIRE_SUPPORTED_MULTICODEC => {
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            multicodec_prefix_for_name(text)
                .map(|_| output_bytes(Vec::new()))
                .map_err(multicodec_status)
        }
        CODEC_DAG_CBOR_COMPUTE_CID => compute_dag_cbor_cid(first)
            .map(text_bytes)
            .map_err(|_| CODEC_INVALID_ARGUMENT),
        CODEC_DAG_CBOR_SHA256_CONTENT_HASH => dag_cbor_content_hash(first)
            .map(|hash| output_bytes(hash.to_vec()))
            .map_err(|_| CODEC_INVALID_ARGUMENT),
        CODEC_DAG_CBOR_MULTIHASH => dag_cbor_multihash_value(first)
            .map(|hash| output_bytes(hash.to_bytes()))
            .map_err(|_| CODEC_INVALID_ARGUMENT),
        CODEC_TRY_PARSE_CID => {
            let cid = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            parse_cid(cid).map(text_bytes).ok_or(CODEC_INVALID_ARGUMENT)
        }
        CODEC_DAG_CBOR_CODEC_CODE => Ok(text_bytes(dag_cbor_codec_code().to_string())),
        CODEC_CANONICALIZE_JSON => {
            let text = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            canonicalize_json(text)
                .map(text_bytes)
                .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        CODEC_VALIDATE_KEY_BINDING => {
            let binding_type = core::str::from_utf8(first).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            let algorithm = empty_or_text(second_ptr, second_len)?;
            let multikey = empty_or_text(third_ptr, third_len)?;
            let parsed = parse_multikey_value(multikey).map_err(|_| CODEC_INVALID_ARGUMENT)?;
            validate_binding(
                binding_type,
                if algorithm.is_empty() {
                    None
                } else {
                    Some(algorithm)
                },
                &parsed,
            )
            .map(|()| output_bytes(Vec::new()))
            .map_err(|_| CODEC_INVALID_ARGUMENT)
        }
        _ => Err(CODEC_INVALID_ARGUMENT),
    }
}

fn process_bool(
    operation: u32,
    first_ptr: *const u8,
    first_len: usize,
    second_ptr: *const u8,
    second_len: usize,
) -> Result<i32, CodecStatus> {
    validate_boundary_input_lengths(&[first_len, second_len])?;
    let first = read_text(first_ptr, first_len)?;
    let second = read_text(second_ptr, second_len)?;
    match operation {
        CODEC_BOOL_BINDING_TYPE_MATCHES_CODEC => {
            Ok(i32::from(binding_matches_codec(first, second)))
        }
        CODEC_BOOL_IS_VALID_CID_STRING => Ok(i32::from(valid_cid(first))),
        _ => Err(CODEC_INVALID_ARGUMENT),
    }
}

/// Run a ReallyMe codec operation through the shared Rust implementation.
///
/// Text arguments and text outputs are UTF-8 bytes. Structured scalar outputs
/// whose families have generated result messages use compact generated
/// ProtoJSON at this ABI boundary. Structured DAG-CBOR encode/decode do not
/// use this scalar ABI lane in 0.2.0; callers must use the generated operation
/// response operation contract so result variants remain fully discriminated.
///
/// # Safety
///
/// Non-empty input ranges must point to initialized caller-owned bytes that
/// remain valid for the duration of the call. The implementation copies all
/// operation results into Rust-owned memory before mutating output, so callers
/// may use the same byte storage for input and output when their platform ABI
/// permits it. Non-empty output ranges must point to writable caller-owned
/// bytes and must not alias `len_out`. `len_out` must point to writable,
/// aligned `usize` storage. Once those output pointers validate, `len_out` is
/// initialized to zero before inputs are processed. A buffer-too-small result
/// replaces it with the required length; every other failure leaves it zero.
#[no_mangle]
pub unsafe extern "C" fn rm_codec_process(
    operation: u32,
    first_ptr: *const u8,
    first_len: usize,
    second_ptr: *const u8,
    second_len: usize,
    third_ptr: *const u8,
    third_len: usize,
    output_ptr: *mut u8,
    output_len: usize,
    len_out: *mut usize,
) -> CodecStatus {
    ffi_guard(|| {
        let output_status = initialize_output_length(output_ptr, output_len, len_out);
        if output_status != CODEC_OK {
            return output_status;
        }
        let output = match process(
            operation, first_ptr, first_len, second_ptr, second_len, third_ptr, third_len,
        ) {
            Ok(value) => value,
            Err(status) => return status,
        };
        write_output(output_ptr, output_len, len_out, output)
    })
}

/// Run a ReallyMe codec predicate through the shared Rust implementation.
///
/// # Safety
///
/// Non-empty input ranges must point to initialized caller-owned bytes that
/// remain valid for the duration of the call. `result_out` must point to
/// writable, aligned `i32` storage. Once validated, `result_out` is initialized
/// to false (`0`) before inputs are processed and remains false on failure.
#[no_mangle]
pub unsafe extern "C" fn rm_codec_process_bool(
    operation: u32,
    first_ptr: *const u8,
    first_len: usize,
    second_ptr: *const u8,
    second_len: usize,
    result_out: *mut i32,
) -> CodecStatus {
    ffi_guard(|| {
        // SAFETY: `write_i32` validates null and alignment before writing the
        // deterministic failure value.
        let result_status = unsafe { write_i32(result_out, 0) };
        if result_status != CODEC_OK {
            return result_status;
        }
        let value = match process_bool(operation, first_ptr, first_len, second_ptr, second_len) {
            Ok(value) => value,
            Err(status) => return status,
        };
        // SAFETY: `result_out` is governed by this export's safety operation contract,
        // and `write_i32` validates null and alignment before writing.
        unsafe { write_i32(result_out, value) }
    })
}

#[cfg(test)]
mod tests;
