// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import com.google.protobuf.InvalidProtocolBufferException
import com.google.protobuf.UnsafeByteOperations
import me.really.codec.v1.CodecError
import me.really.codec.v1.CodecErrorOrigin
import me.really.codec.v1.CodecErrorReason
import me.really.codec.v1.CodecOperationRequest
import me.really.codec.v1.CodecOperationResponse
import me.really.codec.v1.CodecOperationResult

internal val emptyBytes: ByteArray = ByteArray(0)

internal fun process(
    operation: Int,
    first: ByteArray,
    second: ByteArray = emptyBytes,
    third: ByteArray = emptyBytes,
): ByteArray {
    ReallyMeCodecRustNativeProvider.requireLoaded()
    return try {
        ReallyMeCodecNative.processNative(operation, first, second, third)
    } catch (error: UnsatisfiedLinkError) {
        throw ReallyMeCodecException.ProviderFailure()
    }
}

/**
 * Executes one generated request through the fully discriminated response
 * boundary used by structured SDK convenience methods.
 *
 * The JNI request and response arrays are SDK-owned transient buffers and
 * are wiped on every path. Each public method must still require its exact
 * generated result case before converting to an SDK domain value.
 */
internal fun <T> processOperation(
    request: CodecOperationRequest,
    consume: (CodecOperationResult) -> T,
): T {
    ReallyMeCodecRustNativeProvider.requireLoaded()
    val serializedSize = request.serializedSize
    if (serializedSize < 0) {
        throw ReallyMeCodecException.ProviderFailure()
    }
    if (serializedSize > MAX_CODEC_PROTO_MESSAGE_BYTES) {
        throw ReallyMeCodecException.InvalidInput()
    }
    val requestBytes =
        try {
            request.toByteArray()
        } catch (_: RuntimeException) {
            throw ReallyMeCodecException.ProviderFailure()
        }
    val responseBytes =
        try {
            ReallyMeCodecNative.processOperationNative(requestBytes)
        } catch (_: UnsatisfiedLinkError) {
            throw ReallyMeCodecException.ProviderFailure()
        } finally {
            requestBytes.fill(0)
        }
    return try {
        // Parse from the response owner without making detached ByteString
        // copies. The consumer must finish before the finally block wipes
        // that owner; domain values copy only the bytes they return.
        val input = UnsafeByteOperations.unsafeWrap(responseBytes).newCodedInput()
        input.enableAliasing(true)
        input.setRecursionLimit(MAX_DETERMINISTIC_CBOR_PROTO_MESSAGE_DEPTH)
        val response = CodecOperationResponse.parseFrom(input)
        if (response.reallyMeHasUnknownFieldsForValidation()) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        when (response.outcomeCase) {
            CodecOperationResponse.OutcomeCase.RESULT -> {
                val result = response.result
                if (
                    result.reallyMeHasUnknownFieldsForValidation() ||
                    result.resultCase == CodecOperationResult.ResultCase.RESULT_NOT_SET
                ) {
                    throw ReallyMeCodecException.ProviderFailure()
                }
                consume(result)
            }
            CodecOperationResponse.OutcomeCase.ERROR ->
                throw exceptionForCodecError(response.error)
            CodecOperationResponse.OutcomeCase.OUTCOME_NOT_SET,
            null,
            -> throw ReallyMeCodecException.ProviderFailure()
        }
    } catch (_: InvalidProtocolBufferException) {
        throw ReallyMeCodecException.ProviderFailure()
    } catch (error: ReallyMeCodecException) {
        throw error
    } catch (_: RuntimeException) {
        throw ReallyMeCodecException.ProviderFailure()
    } finally {
        responseBytes.fill(0)
    }
}

private fun exceptionForCodecError(codecError: CodecError): ReallyMeCodecException {
    if (codecError.reallyMeHasUnknownFieldsForValidation()) {
        return ReallyMeCodecException.ProviderFailure()
    }
    val expectedOrigin =
        when (codecError.errorCase) {
            CodecError.ErrorCase.BASE_ENCODING -> {
                if (!knownProviderReason(
                        codecError.baseEncoding.reallyMeHasUnknownFieldsForValidation(),
                        codecError.baseEncoding.reason,
                        100..199,
                    )
                ) {
                    return ReallyMeCodecException.ProviderFailure()
                }
                CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER
            }
            CodecError.ErrorCase.PEM -> {
                if (!knownProviderReason(codecError.pem.reallyMeHasUnknownFieldsForValidation(), codecError.pem.reason, 200..299)) {
                    return ReallyMeCodecException.ProviderFailure()
                }
                CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER
            }
            CodecError.ErrorCase.MULTIFORMAT -> {
                if (!knownProviderReason(
                        codecError.multiformat.reallyMeHasUnknownFieldsForValidation(),
                        codecError.multiformat.reason,
                        300..399,
                    )
                ) {
                    return ReallyMeCodecException.ProviderFailure()
                }
                CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER
            }
            CodecError.ErrorCase.CANONICALIZATION -> {
                if (!knownProviderReason(
                        codecError.canonicalization.reallyMeHasUnknownFieldsForValidation(),
                        codecError.canonicalization.reason,
                        400..499,
                    )
                ) {
                    return ReallyMeCodecException.ProviderFailure()
                }
                if (
                    codecError.canonicalization.reason ==
                    CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_INTERNAL
                ) {
                    CodecErrorOrigin.CODEC_ERROR_ORIGIN_PROVIDER
                } else {
                    CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER
                }
            }
            CodecError.ErrorCase.BACKEND -> {
                if (!knownProviderReason(codecError.backend.reallyMeHasUnknownFieldsForValidation(), codecError.backend.reason, 500..599)) {
                    return ReallyMeCodecException.ProviderFailure()
                }
                CodecErrorOrigin.CODEC_ERROR_ORIGIN_PROVIDER
            }
            CodecError.ErrorCase.BOUNDARY -> {
                if (!knownProviderReason(
                        codecError.boundary.reallyMeHasUnknownFieldsForValidation(),
                        codecError.boundary.reason,
                        600..699,
                    )
                ) {
                    return ReallyMeCodecException.ProviderFailure()
                }
                CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER
            }
            CodecError.ErrorCase.ERROR_NOT_SET,
            null,
            -> return ReallyMeCodecException.ProviderFailure()
        }
    if (codecError.origin != expectedOrigin) {
        return ReallyMeCodecException.ProviderFailure()
    }
    if (expectedOrigin != CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER) {
        return ReallyMeCodecException.ProviderFailure()
    }
    val reason =
        when (codecError.errorCase) {
            CodecError.ErrorCase.BASE_ENCODING -> codecError.baseEncoding.reason
            CodecError.ErrorCase.CANONICALIZATION -> codecError.canonicalization.reason
            CodecError.ErrorCase.MULTIFORMAT -> codecError.multiformat.reason
            else -> null
        }
    if (reason == CodecErrorReason.CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC ||
        reason == CodecErrorReason.CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTICODEC_PREFIX
    ) {
        return ReallyMeCodecException.UnsupportedCodec()
    }
    if (reason == CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_UNSUPPORTED_IPLD_VALUE) {
        return ReallyMeCodecException.UnsupportedIpldValue()
    }
    return if (reason in
        setOf(
            CodecErrorReason.CODEC_ERROR_REASON_BASE_NON_CANONICAL_HEX,
            CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_NON_CANONICAL_CBOR,
            CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_NON_CANONICAL_JSON,
            CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_NON_MINIMAL_CBOR_INTEGER,
            CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_DUPLICATE_CBOR_MAP_KEY,
            CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_CBOR_MAP_KEYS_OUT_OF_ORDER,
            CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_CBOR_TRAILING_BYTES,
        )
    ) {
        ReallyMeCodecException.NonCanonical()
    } else {
        ReallyMeCodecException.InvalidInput()
    }
}

private fun knownProviderReason(
    hasUnknownFields: Boolean,
    reason: CodecErrorReason,
    expectedRange: IntRange,
): Boolean =
    !hasUnknownFields &&
        reason != CodecErrorReason.UNRECOGNIZED &&
        reason.number in expectedRange

internal fun processBool(
    operation: Int,
    first: ByteArray,
    second: ByteArray,
): Boolean {
    ReallyMeCodecRustNativeProvider.requireLoaded()
    return try {
        when (ReallyMeCodecNative.processBoolNative(operation, first, second)) {
            0 -> false
            1 -> true
            else -> throw ReallyMeCodecException.ProviderFailure()
        }
    } catch (error: UnsatisfiedLinkError) {
        throw ReallyMeCodecException.ProviderFailure()
    }
}
