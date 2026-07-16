// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! JNI bridge for the Kotlin ReallyMe codec package.
//!
//! The Kotlin codec facade deliberately delegates to the Rust codec crates
//! through the same C ABI Swift uses. Kotlin owns only type validation and
//! JVM packaging; base encodings, multicodec, DAG-CBOR, JCS, and PEM parsing
//! remain single-sourced in Rust.

use crate::codec::{rm_codec_process, rm_codec_process_bool, rm_codec_process_proto};
use crate::status::{
    CODEC_BUFFER_TOO_SMALL, CODEC_INTERNAL_ERROR, CODEC_INVALID_ARGUMENT, CODEC_OK,
    CODEC_PROTO_ERROR,
};
use jni::objects::{JByteArray, JObject, JValue};
use jni::signature::{FieldSignature, JavaType, MethodSignature, Primitive};
use jni::sys::{jbyteArray, jint, jobject};
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

struct NativeOutput {
    status: i32,
    bytes: Zeroizing<Vec<u8>>,
}

// SAFETY: The literal descriptor is the JVM field descriptor for the
// ReallyMeCodecProtoStatus enum type and matches the Java class loaded below.
const PROTO_STATUS_FIELD_SIGNATURE: FieldSignature<'static> = unsafe {
    FieldSignature::from_raw_parts(
        jni::jni_str!("Lme/really/codec/ReallyMeCodecProtoStatus;"),
        JavaType::Object,
    )
};
const PROTO_RESULT_CONSTRUCTOR_ARGS: &[JavaType] = &[JavaType::Object, JavaType::Array];
// SAFETY: The literal descriptor matches the Kotlin data class constructor
// `(ReallyMeCodecProtoStatus, byte[])`, returning `void` as all JVM
// constructors do.
const PROTO_RESULT_CONSTRUCTOR_SIGNATURE: MethodSignature<'static, 'static> = unsafe {
    MethodSignature::from_raw_parts(
        jni::jni_str!("(Lme/really/codec/ReallyMeCodecProtoStatus;[B)V"),
        PROTO_RESULT_CONSTRUCTOR_ARGS,
        JavaType::Primitive(Primitive::Void),
    )
};

/// Verifies that the loaded native image contains the expected JNI symbols.
#[no_mangle]
pub extern "system" fn Java_me_really_codec_ReallyMeCodecNative_probeNative(
    _env: EnvUnowned<'_>,
    _receiver: JObject<'_>,
) -> jint {
    1
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

/// Runs a protobuf-output codec operation for `me.really:codec`.
#[no_mangle]
pub extern "system" fn Java_me_really_codec_ReallyMeCodecNative_processProtoNative<'local>(
    env: EnvUnowned<'local>,
    _receiver: JObject<'local>,
    operation: jint,
    first: JByteArray<'local>,
    second: JByteArray<'local>,
    third: JByteArray<'local>,
) -> jbyteArray {
    process_with_function(env, operation, first, second, third, rm_codec_process_proto)
}

/// Runs a protobuf-output operation once and returns both its status and bytes.
#[no_mangle]
pub extern "system" fn Java_me_really_codec_ReallyMeCodecNative_processProtoResultNative<'local>(
    env: EnvUnowned<'local>,
    _receiver: JObject<'local>,
    operation: jint,
    first: JByteArray<'local>,
    second: JByteArray<'local>,
    third: JByteArray<'local>,
) -> jobject {
    process_proto_result(env, operation, first, second, third)
}

fn process_with_function<'local>(
    mut env: EnvUnowned<'local>,
    operation: jint,
    first: JByteArray<'local>,
    second: JByteArray<'local>,
    third: JByteArray<'local>,
    process: CodecProcessCFunction,
) -> jbyteArray {
    let outcome = env.with_env(|env| -> jni::errors::Result<jbyteArray> {
        let output = process_output_with_function(env, operation, first, second, third, process)?;
        env.byte_array_from_slice(&output.bytes)
            .map(|value| value.into_raw())
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

fn process_proto_result<'local>(
    mut env: EnvUnowned<'local>,
    operation: jint,
    first: JByteArray<'local>,
    second: JByteArray<'local>,
    third: JByteArray<'local>,
) -> jobject {
    let outcome = env.with_env(|env| -> jni::errors::Result<jobject> {
        let output = process_output_with_function(
            env,
            operation,
            first,
            second,
            third,
            rm_codec_process_proto,
        )?;
        let status = match output.status {
            CODEC_OK => env
                .get_static_field(
                    jni::jni_str!("me/really/codec/ReallyMeCodecProtoStatus"),
                    jni::jni_str!("RESULT"),
                    PROTO_STATUS_FIELD_SIGNATURE,
                )?
                .l()?,
            CODEC_PROTO_ERROR => env
                .get_static_field(
                    jni::jni_str!("me/really/codec/ReallyMeCodecProtoStatus"),
                    jni::jni_str!("CODEC_ERROR"),
                    PROTO_STATUS_FIELD_SIGNATURE,
                )?
                .l()?,
            _ => return throw_provider_failure(env),
        };
        let bytes = env.byte_array_from_slice(&output.bytes)?;
        let bytes_object = JObject::from(bytes);
        let result = env.new_object(
            jni::jni_str!("me/really/codec/ReallyMeCodecProtoResult"),
            PROTO_RESULT_CONSTRUCTOR_SIGNATURE,
            &[JValue::Object(&status), JValue::Object(&bytes_object)],
        )?;
        Ok(result.into_raw())
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
    if first_status != CODEC_OK
        && first_status != CODEC_PROTO_ERROR
        && first_status != CODEC_BUFFER_TOO_SMALL
    {
        return throw_for_status(env, first_status);
    }
    if produced_len == 0 {
        return Ok(NativeOutput {
            status: first_status,
            bytes: Zeroizing::new(Vec::new()),
        });
    }

    let mut output = Zeroizing::new(vec![0_u8; produced_len]);
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
    if (status != CODEC_OK && status != CODEC_PROTO_ERROR) || produced_len > output.len() {
        return throw_for_status(env, status);
    }
    output.truncate(produced_len);
    Ok(NativeOutput {
        status,
        bytes: output,
    })
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
    let outcome = env.with_env(|env| -> jni::errors::Result<jint> {
        let operation = match u32::try_from(operation) {
            Ok(value) => value,
            Err(_) => {
                env.throw_new_void(jni::jni_str!(
                    "me/really/codec/ReallyMeCodecException$InvalidInput"
                ))?;
                return Ok(-1);
            }
        };
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
