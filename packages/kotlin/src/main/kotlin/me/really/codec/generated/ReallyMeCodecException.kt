// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Generated from reallyme/codec/v1/codec.proto. Do not edit.

package me.really.codec

import me.really.codec.v1.CodecErrorOrigin
import me.really.codec.v1.CodecErrorReason
import me.really.codec.v1.CodecSdkErrorCategory

/** Typed SDK failures contain no raw input or backend diagnostics. */
public sealed class ReallyMeCodecException private constructor(
    category: CodecSdkErrorCategory,
    public val reason: CodecErrorReason?,
    public val origin: CodecErrorOrigin?,
) : RuntimeException(codecSdkErrorMessage(category)) {
    public open class InvalidInput internal constructor(
        reason: CodecErrorReason?,
        origin: CodecErrorOrigin?,
    ) : ReallyMeCodecException(CodecSdkErrorCategory.CODEC_SDK_ERROR_CATEGORY_INVALID_INPUT, reason, origin) {
        public constructor() : this(null, null)
    }

    public class UnsupportedCodec internal constructor(
        reason: CodecErrorReason?,
        origin: CodecErrorOrigin?,
    ) : InvalidInput(reason, origin) {
        public constructor() : this(null, null)
    }

    public class UnsupportedIpldValue internal constructor(
        reason: CodecErrorReason?,
        origin: CodecErrorOrigin?,
    ) : InvalidInput(reason, origin) {
        public constructor() : this(null, null)
    }

    public class NonCanonical internal constructor(
        reason: CodecErrorReason?,
        origin: CodecErrorOrigin?,
    ) : InvalidInput(reason, origin) {
        public constructor() : this(null, null)
    }

    public class ProviderFailure internal constructor(
        reason: CodecErrorReason?,
        origin: CodecErrorOrigin?,
    ) : ReallyMeCodecException(CodecSdkErrorCategory.CODEC_SDK_ERROR_CATEGORY_PROVIDER_FAILURE, reason, origin) {
        public constructor() : this(null, null)
    }
}

private fun codecSdkErrorMessage(category: CodecSdkErrorCategory): String =
    when (category) {
        CodecSdkErrorCategory.CODEC_SDK_ERROR_CATEGORY_INVALID_INPUT -> "invalid input"
        CodecSdkErrorCategory.CODEC_SDK_ERROR_CATEGORY_PROVIDER_FAILURE -> "provider failure"
        else -> "provider failure"
    }
