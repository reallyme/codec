// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! JNI bridge for the Kotlin ReallyMe codec package.
//!
//! The Kotlin codec facade deliberately delegates to the Rust codec crates
//! through the same C ABI Swift uses. Kotlin owns only type validation and
//! JVM packaging; base encodings, multicodec, DAG-CBOR, JCS, and PEM parsing
//! remain single-sourced in Rust.

use crate::codec::{
    rm_codec_abi_version, rm_codec_max_ffi_input_bytes, rm_codec_max_ffi_output_bytes,
    rm_codec_max_operation_response_bytes, rm_codec_process, rm_codec_process_bool,
    rm_codec_process_operation, rm_codec_process_operation_json, MAX_CODEC_FFI_INPUT_BYTES,
    MAX_CODEC_FFI_OUTPUT_BYTES,
};
use crate::guard::with_redacted_panic_hook;
use crate::status::{
    CODEC_BUFFER_TOO_SMALL, CODEC_INTERNAL_ERROR, CODEC_INVALID_ARGUMENT, CODEC_OK,
};
use codec_proto::{MAX_CODEC_PROTO_JSON_BYTES, MAX_CODEC_PROTO_MESSAGE_BYTES};
use jni::objects::{JByteArray, JObject};
use jni::sys::{jbyteArray, jint, jlong};
use jni::{EnvUnowned, Outcome};
use std::ptr;
use zeroize::Zeroizing;

type CodecProcessCFunction = unsafe extern "C" fn(
    u32,
    *const u8,
    usize,
    *const u8,
    usize,
    *const u8,
    usize,
    *mut u8,
    usize,
    *mut usize,
) -> i32;

type CodecOperationProcessCFunction =
    unsafe extern "C" fn(*const u8, usize, *mut u8, usize, *mut usize) -> i32;

struct NativeOutput {
    bytes: Zeroizing<Vec<u8>>,
}

fn probed_output_capacity(status: i32, produced_len: usize) -> Option<usize> {
    match (status, produced_len) {
        (CODEC_OK, 0) => Some(0),
        (CODEC_BUFFER_TOO_SMALL, 1..=MAX_CODEC_FFI_OUTPUT_BYTES) => Some(produced_len),
        _ => None,
    }
}

/// Verifies that the loaded native image contains the expected JNI symbols.
#[no_mangle]
pub extern "system" fn Java_me_really_codec_ReallyMeCodecNative_probeNative(
    _env: EnvUnowned<'_>,
    _receiver: JObject<'_>,
) -> jint {
    1
}

/// Returns the C ABI version implemented by the loaded native image.
#[no_mangle]
pub extern "system" fn Java_me_really_codec_ReallyMeCodecNative_abiVersionNative(
    _env: EnvUnowned<'_>,
    _receiver: JObject<'_>,
) -> jint {
    jint::try_from(rm_codec_abi_version()).unwrap_or(-1)
}

/// Returns Rust's authoritative generic C ABI input limit for Kotlin callers.
#[no_mangle]
pub extern "system" fn Java_me_really_codec_ReallyMeCodecNative_maxFfiInputBytesNative(
    _env: EnvUnowned<'_>,
    _receiver: JObject<'_>,
) -> jlong {
    jlong::try_from(rm_codec_max_ffi_input_bytes()).unwrap_or(-1)
}

/// Returns Rust's authoritative generic C ABI output limit for Kotlin callers.
#[no_mangle]
pub extern "system" fn Java_me_really_codec_ReallyMeCodecNative_maxFfiOutputBytesNative(
    _env: EnvUnowned<'_>,
    _receiver: JObject<'_>,
) -> jlong {
    jlong::try_from(rm_codec_max_ffi_output_bytes()).unwrap_or(-1)
}

/// Returns Rust's authoritative generated operation response size limit.
#[no_mangle]
pub extern "system" fn Java_me_really_codec_ReallyMeCodecNative_maxOperationResponseBytesNative(
    _env: EnvUnowned<'_>,
    _receiver: JObject<'_>,
) -> jlong {
    jlong::try_from(rm_codec_max_operation_response_bytes()).unwrap_or(-1)
}

/// Runs a codec operation for `me.really:codec`.
#[no_mangle]
pub extern "system" fn Java_me_really_codec_ReallyMeCodecNative_processNative<'local>(
    env: EnvUnowned<'local>,
    _receiver: JObject<'local>,
    operation: jint,
    first: JByteArray<'local>,
    second: JByteArray<'local>,
    third: JByteArray<'local>,
) -> jbyteArray {
    process_with_function(env, operation, first, second, third, rm_codec_process)
}

/// Executes one binary protobuf request and returns a fully discriminated
/// `CodecOperationResponse`.
#[no_mangle]
pub extern "system" fn Java_me_really_codec_ReallyMeCodecNative_processOperationNative<'local>(
    env: EnvUnowned<'local>,
    _receiver: JObject<'local>,
    request: JByteArray<'local>,
) -> jbyteArray {
    process_operation_output(
        env,
        request,
        MAX_CODEC_PROTO_MESSAGE_BYTES,
        rm_codec_process_operation,
    )
}

/// Executes one generated ProtoJSON request and returns a fully discriminated
/// binary `CodecOperationResponse`.
#[no_mangle]
pub extern "system" fn Java_me_really_codec_ReallyMeCodecNative_processOperationJsonNative<
    'local,
>(
    env: EnvUnowned<'local>,
    _receiver: JObject<'local>,
    request: JByteArray<'local>,
) -> jbyteArray {
    process_operation_output(
        env,
        request,
        MAX_CODEC_PROTO_JSON_BYTES,
        rm_codec_process_operation_json,
    )
}

fn process_operation_output<'local>(
    mut env: EnvUnowned<'local>,
    request: JByteArray<'local>,
    max_request_len: usize,
    process: CodecOperationProcessCFunction,
) -> jbyteArray {
    let outcome = with_redacted_panic_hook(|| {
        env.with_env(|env| -> jni::errors::Result<jbyteArray> {
            let request = bounded_proto_request_bytes(env, request, max_request_len)?;
            let output = call_operation_boundary(env, request.as_slice(), process)?;
            env.byte_array_from_slice(output.as_slice())
                .map(|value| value.into_raw())
        })
    });
    match outcome.into_outcome() {
        Outcome::Ok(value) => value,
        Outcome::Err(_) | Outcome::Panic(_) => {
            throw_provider_failure_if_clear(&mut env);
            ptr::null_mut()
        }
    }
}

fn process_with_function<'local>(
    mut env: EnvUnowned<'local>,
    operation: jint,
    first: JByteArray<'local>,
    second: JByteArray<'local>,
    third: JByteArray<'local>,
    process: CodecProcessCFunction,
) -> jbyteArray {
    let outcome = with_redacted_panic_hook(|| {
        env.with_env(|env| -> jni::errors::Result<jbyteArray> {
            let output =
                process_output_with_function(env, operation, first, second, third, process)?;
            env.byte_array_from_slice(&output.bytes)
                .map(|value| value.into_raw())
        })
    });

    match outcome.into_outcome() {
        Outcome::Ok(value) => value,
        Outcome::Err(_) => {
            throw_provider_failure_if_clear(&mut env);
            ptr::null_mut()
        }
        Outcome::Panic(_) => {
            throw_provider_failure_if_clear(&mut env);
            ptr::null_mut()
        }
    }
}

fn call_operation_boundary<'local>(
    env: &mut jni::Env<'local>,
    request_bytes: &[u8],
    process: CodecOperationProcessCFunction,
) -> jni::errors::Result<Zeroizing<Vec<u8>>> {
    let mut produced_len = 0_usize;
    // SAFETY: The request vector and produced-length output are owned by this
    // JNI frame and remain valid for the duration of the call.
    let probe_status = unsafe {
        process(
            request_bytes.as_ptr(),
            request_bytes.len(),
            ptr::null_mut(),
            0,
            &mut produced_len,
        )
    };
    if probe_status != CODEC_BUFFER_TOO_SMALL
        || produced_len == 0
        || produced_len > MAX_CODEC_PROTO_MESSAGE_BYTES
    {
        return throw_provider_failure(env);
    }

    let mut output = Zeroizing::new(vec![0_u8; produced_len]);
    // SAFETY: The request and output vectors are distinct JNI-frame-owned
    // allocations and `produced_len` is writable stack storage.
    let status = unsafe {
        process(
            request_bytes.as_ptr(),
            request_bytes.len(),
            output.as_mut_ptr(),
            output.len(),
            &mut produced_len,
        )
    };
    if status != CODEC_OK || produced_len != output.len() {
        return throw_provider_failure(env);
    }
    Ok(output)
}

fn bounded_proto_request_bytes<'local>(
    env: &mut jni::Env<'local>,
    request: JByteArray<'local>,
    max_request_len: usize,
) -> jni::errors::Result<Zeroizing<Vec<u8>>> {
    let request_len = match request.len(env) {
        Ok(value) => value,
        Err(_) => return throw_provider_failure(env),
    };
    if request_len > max_request_len {
        // Resource-limit failures belong in the generated operation response. A
        // bounded over-limit sentinel asks the native boundary to construct
        // that envelope without copying an attacker-sized managed array.
        let sentinel_len = match max_request_len.checked_add(1) {
            Some(value) => value,
            None => return throw_provider_failure(env),
        };
        return Ok(Zeroizing::new(vec![0_u8; sentinel_len]));
    }
    match env.convert_byte_array(&request) {
        Ok(value) => Ok(Zeroizing::new(value)),
        Err(_) => throw_provider_failure(env),
    }
}

fn process_output_with_function<'local>(
    env: &mut jni::Env<'local>,
    operation: jint,
    first: JByteArray<'local>,
    second: JByteArray<'local>,
    third: JByteArray<'local>,
    process: CodecProcessCFunction,
) -> jni::errors::Result<NativeOutput> {
    let operation = match u32::try_from(operation) {
        Ok(value) => value,
        Err(_) => return throw_invalid_input(env),
    };
    validate_managed_input_lengths(env, &[&first, &second, &third], MAX_CODEC_FFI_INPUT_BYTES)?;
    let first_bytes = match env.convert_byte_array(&first) {
        Ok(value) => Zeroizing::new(value),
        Err(_) => return throw_provider_failure(env),
    };
    let second_bytes = match env.convert_byte_array(&second) {
        Ok(value) => Zeroizing::new(value),
        Err(_) => return throw_provider_failure(env),
    };
    let third_bytes = match env.convert_byte_array(&third) {
        Ok(value) => Zeroizing::new(value),
        Err(_) => return throw_provider_failure(env),
    };

    let mut produced_len = 0_usize;
    // SAFETY: The byte vectors are owned by this JNI frame and stay alive for
    // the call. The first pass provides no output buffer, only a writable
    // stack-owned produced-length pointer.
    let first_status = unsafe {
        process(
            operation,
            first_bytes.as_ptr(),
            first_bytes.len(),
            second_bytes.as_ptr(),
            second_bytes.len(),
            third_bytes.as_ptr(),
            third_bytes.len(),
            ptr::null_mut(),
            0,
            &mut produced_len,
        )
    };
    if first_status != CODEC_OK && first_status != CODEC_BUFFER_TOO_SMALL {
        return throw_for_status(env, first_status);
    }
    let Some(output_capacity) = probed_output_capacity(first_status, produced_len) else {
        return throw_provider_failure(env);
    };
    if output_capacity == 0 {
        return Ok(NativeOutput {
            bytes: Zeroizing::new(Vec::new()),
        });
    }

    let mut output = Zeroizing::new(vec![0_u8; output_capacity]);
    // SAFETY: The input byte vectors and output vector are owned by this JNI
    // frame, non-aliasing, and live for the call. `produced_len` is a writable
    // stack-owned length output.
    let status = unsafe {
        process(
            operation,
            first_bytes.as_ptr(),
            first_bytes.len(),
            second_bytes.as_ptr(),
            second_bytes.len(),
            third_bytes.as_ptr(),
            third_bytes.len(),
            output.as_mut_ptr(),
            output.len(),
            &mut produced_len,
        )
    };
    if status != CODEC_OK {
        return throw_for_status(env, status);
    }
    if produced_len != output.len() {
        return throw_provider_failure(env);
    }
    Ok(NativeOutput { bytes: output })
}

/// Runs a codec predicate for `me.really:codec`.
#[no_mangle]
pub extern "system" fn Java_me_really_codec_ReallyMeCodecNative_processBoolNative<'local>(
    mut env: EnvUnowned<'local>,
    _receiver: JObject<'local>,
    operation: jint,
    first: JByteArray<'local>,
    second: JByteArray<'local>,
) -> jint {
    let outcome = with_redacted_panic_hook(|| {
        env.with_env(|env| -> jni::errors::Result<jint> {
            let operation = match u32::try_from(operation) {
                Ok(value) => value,
                Err(_) => {
                    env.throw_new_void(jni::jni_str!(
                        "me/really/codec/ReallyMeCodecException$InvalidInput"
                    ))?;
                    return Ok(-1);
                }
            };
            validate_managed_input_lengths(env, &[&first, &second], MAX_CODEC_FFI_INPUT_BYTES)?;
            let first_bytes = match env.convert_byte_array(&first) {
                Ok(value) => Zeroizing::new(value),
                Err(_) => {
                    env.throw_new_void(jni::jni_str!(
                        "me/really/codec/ReallyMeCodecException$ProviderFailure"
                    ))?;
                    return Ok(-1);
                }
            };
            let second_bytes = match env.convert_byte_array(&second) {
                Ok(value) => Zeroizing::new(value),
                Err(_) => {
                    env.throw_new_void(jni::jni_str!(
                        "me/really/codec/ReallyMeCodecException$ProviderFailure"
                    ))?;
                    return Ok(-1);
                }
            };

            let mut result = 0_i32;
            // SAFETY: The byte vectors are owned by this JNI frame and live for
            // the call. `result` is writable stack-owned `i32` output storage.
            let status = unsafe {
                rm_codec_process_bool(
                    operation,
                    first_bytes.as_ptr(),
                    first_bytes.len(),
                    second_bytes.as_ptr(),
                    second_bytes.len(),
                    &mut result,
                )
            };
            if status != CODEC_OK {
                throw_for_status(env, status)?;
                return Ok(-1);
            }
            Ok(result)
        })
    });

    match outcome.into_outcome() {
        Outcome::Ok(value) => value,
        Outcome::Err(_) => {
            throw_provider_failure_if_clear(&mut env);
            -1
        }
        Outcome::Panic(_) => {
            throw_provider_failure_if_clear(&mut env);
            -1
        }
    }
}

fn validate_managed_input_lengths<'local>(
    env: &mut jni::Env<'local>,
    inputs: &[&JByteArray<'local>],
    maximum: usize,
) -> jni::errors::Result<()> {
    let mut aggregate = 0_usize;
    for input in inputs {
        let length = match input.len(env) {
            Ok(value) => value,
            Err(_) => return throw_provider_failure(env),
        };
        aggregate = match aggregate.checked_add(length) {
            Some(value) => value,
            None => return throw_invalid_input(env),
        };
        if aggregate > maximum {
            return throw_invalid_input(env);
        }
    }
    Ok(())
}

fn throw_provider_failure_if_clear(env: &mut EnvUnowned<'_>) {
    let _outcome = env.with_env(|env| -> jni::errors::Result<()> {
        if !env.exception_check() {
            env.throw_new_void(jni::jni_str!(
                "me/really/codec/ReallyMeCodecException$ProviderFailure"
            ))?;
        }
        Ok(())
    });
}

fn throw_for_status<'local, T>(env: &mut jni::Env<'local>, status: i32) -> jni::errors::Result<T> {
    match status {
        CODEC_INVALID_ARGUMENT => throw_invalid_input(env),
        CODEC_INTERNAL_ERROR => throw_provider_failure(env),
        _ => throw_provider_failure(env),
    }
}

fn throw_invalid_input<'local, T>(env: &mut jni::Env<'local>) -> jni::errors::Result<T> {
    env.throw_new_void(jni::jni_str!(
        "me/really/codec/ReallyMeCodecException$InvalidInput"
    ))?;
    Err(jni::errors::Error::JavaException)
}

fn throw_provider_failure<'local, T>(env: &mut jni::Env<'local>) -> jni::errors::Result<T> {
    env.throw_new_void(jni::jni_str!(
        "me/really/codec/ReallyMeCodecException$ProviderFailure"
    ))?;
    Err(jni::errors::Error::JavaException)
}

#[cfg(test)]
mod tests {
    use super::{probed_output_capacity, MAX_CODEC_FFI_OUTPUT_BYTES};
    use crate::status::{CODEC_BUFFER_TOO_SMALL, CODEC_INTERNAL_ERROR, CODEC_OK};

    #[test]
    fn generic_output_probe_is_strict_and_bounded() {
        assert_eq!(probed_output_capacity(CODEC_OK, 0), Some(0));
        assert_eq!(probed_output_capacity(CODEC_BUFFER_TOO_SMALL, 1), Some(1));
        assert_eq!(probed_output_capacity(CODEC_OK, 1), None);
        assert_eq!(probed_output_capacity(CODEC_BUFFER_TOO_SMALL, 0), None);
        assert_eq!(
            probed_output_capacity(CODEC_BUFFER_TOO_SMALL, MAX_CODEC_FFI_OUTPUT_BYTES + 1),
            None
        );
        assert_eq!(probed_output_capacity(CODEC_INTERNAL_ERROR, 1), None);
    }
}
