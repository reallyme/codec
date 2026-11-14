// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::status::{
    CODEC_INTERNAL_ERROR, CODEC_INVALID_ARGUMENT, CODEC_INVALID_MULTICODEC_PREFIX,
    CODEC_NON_CANONICAL_HEX, CODEC_NON_CANONICAL_JSON, CODEC_UNKNOWN_MULTICODEC,
};
use jni::objects::JByteArray;
use jni::EnvUnowned;

pub(super) fn validate_managed_input_lengths<'local>(
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

pub(super) fn throw_provider_failure_if_clear(env: &mut EnvUnowned<'_>) {
    let _outcome = env.with_env(|env| -> jni::errors::Result<()> {
        if !env.exception_check() {
            env.throw_new_void(jni::jni_str!(
                "me/really/codec/ReallyMeCodecException$ProviderFailure"
            ))?;
        }
        Ok(())
    });
}

pub(super) fn throw_for_status<'local, T>(
    env: &mut jni::Env<'local>,
    status: i32,
) -> jni::errors::Result<T> {
    match status {
        CODEC_INVALID_ARGUMENT => throw_invalid_input(env),
        CODEC_NON_CANONICAL_HEX | CODEC_NON_CANONICAL_JSON => throw_non_canonical(env),
        CODEC_INVALID_MULTICODEC_PREFIX | CODEC_UNKNOWN_MULTICODEC => throw_unsupported_codec(env),
        CODEC_INTERNAL_ERROR => throw_provider_failure(env),
        _ => throw_provider_failure(env),
    }
}

pub(super) fn throw_unsupported_codec<'local, T>(
    env: &mut jni::Env<'local>,
) -> jni::errors::Result<T> {
    env.throw_new_void(jni::jni_str!(
        "me/really/codec/ReallyMeCodecException$UnsupportedCodec"
    ))?;
    Err(jni::errors::Error::JavaException)
}

pub(super) fn throw_non_canonical<'local, T>(env: &mut jni::Env<'local>) -> jni::errors::Result<T> {
    env.throw_new_void(jni::jni_str!(
        "me/really/codec/ReallyMeCodecException$NonCanonical"
    ))?;
    Err(jni::errors::Error::JavaException)
}

pub(super) fn throw_invalid_input<'local, T>(env: &mut jni::Env<'local>) -> jni::errors::Result<T> {
    env.throw_new_void(jni::jni_str!(
        "me/really/codec/ReallyMeCodecException$InvalidInput"
    ))?;
    Err(jni::errors::Error::JavaException)
}

pub(super) fn throw_provider_failure<'local, T>(
    env: &mut jni::Env<'local>,
) -> jni::errors::Result<T> {
    env.throw_new_void(jni::jni_str!(
        "me/really/codec/ReallyMeCodecException$ProviderFailure"
    ))?;
    Err(jni::errors::Error::JavaException)
}
