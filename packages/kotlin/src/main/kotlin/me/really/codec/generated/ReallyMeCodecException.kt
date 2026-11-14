// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Generated from reallyme/codec/v1/codec.proto. Do not edit.

package me.really.codec

import me.really.codec.v1.CodecSdkErrorCategory

/** Typed SDK failures contain no raw input or backend diagnostics. */
public sealed class ReallyMeCodecException private constructor(category: CodecSdkErrorCategory) :
    RuntimeException(codecSdkErrorMessage(category)) {
    public open class InvalidInput : ReallyMeCodecException(CodecSdkErrorCategory.CODEC_SDK_ERROR_CATEGORY_INVALID_INPUT)

    public class UnsupportedCodec : InvalidInput()

    public class UnsupportedIpldValue : InvalidInput()

    public class NonCanonical : InvalidInput()

    public class ProviderFailure : ReallyMeCodecException(CodecSdkErrorCategory.CODEC_SDK_ERROR_CATEGORY_PROVIDER_FAILURE)
}

private fun codecSdkErrorMessage(category: CodecSdkErrorCategory): String =
    when (category) {
        CodecSdkErrorCategory.CODEC_SDK_ERROR_CATEGORY_INVALID_INPUT -> "invalid input"
        CodecSdkErrorCategory.CODEC_SDK_ERROR_CATEGORY_PROVIDER_FAILURE -> "provider failure"
        else -> "provider failure"
    }
