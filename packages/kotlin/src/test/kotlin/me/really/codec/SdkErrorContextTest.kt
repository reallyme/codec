// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import me.really.codec.v1.CodecBackendError
import me.really.codec.v1.CodecBaseEncodingError
import me.really.codec.v1.CodecBoundaryError
import me.really.codec.v1.CodecCanonicalizationError
import me.really.codec.v1.CodecError
import me.really.codec.v1.CodecErrorOrigin
import me.really.codec.v1.CodecErrorReason
import me.really.codec.v1.CodecMultiformatError
import me.really.codec.v1.CodecPemError
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
import kotlin.test.assertNull

class SdkErrorContextTest {
    @Test
    fun convenienceFailureExposesWireContextWithoutRawResponseDecoding() {
        val error =
            assertFailsWith<ReallyMeCodecException.NonCanonical> {
                ReallyMeCodec.deterministicCborDecode(byteArrayOf(0x18, 0x00))
            }
        assertEquals(CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_NON_MINIMAL_CBOR_INTEGER, error.reason)
        assertEquals(CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER, error.origin)
    }

    @Test
    fun everyErrorFamilyPreservesItsGeneratedReasonAndOrigin() {
        val caller = CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER
        val provider = CodecErrorOrigin.CODEC_ERROR_ORIGIN_PROVIDER
        val cases =
            listOf(
                CodecError
                    .newBuilder()
                    .setOrigin(caller)
                    .setBaseEncoding(
                        CodecBaseEncodingError.newBuilder().setReason(CodecErrorReason.CODEC_ERROR_REASON_BASE_INVALID_BASE64),
                    ).build(),
                CodecError
                    .newBuilder()
                    .setOrigin(caller)
                    .setPem(
                        CodecPemError.newBuilder().setReason(CodecErrorReason.CODEC_ERROR_REASON_PEM_RESOURCE_LIMIT_EXCEEDED),
                    ).build(),
                CodecError
                    .newBuilder()
                    .setOrigin(caller)
                    .setMultiformat(
                        CodecMultiformatError.newBuilder().setReason(CodecErrorReason.CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC),
                    ).build(),
                CodecError
                    .newBuilder()
                    .setOrigin(caller)
                    .setCanonicalization(
                        CodecCanonicalizationError.newBuilder().setReason(CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_NON_CANONICAL_JSON),
                    ).build(),
                CodecError
                    .newBuilder()
                    .setOrigin(provider)
                    .setCanonicalization(
                        CodecCanonicalizationError.newBuilder().setReason(CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_INTERNAL),
                    ).build(),
                CodecError
                    .newBuilder()
                    .setOrigin(provider)
                    .setBackend(
                        CodecBackendError.newBuilder().setReason(CodecErrorReason.CODEC_ERROR_REASON_BACKEND_INTERNAL),
                    ).build(),
                boundary(CodecErrorReason.CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED.number, caller),
            )
        for (error in cases) {
            val expectedReason =
                when (error.errorCase) {
                    CodecError.ErrorCase.BASE_ENCODING -> error.baseEncoding.reason
                    CodecError.ErrorCase.PEM -> error.pem.reason
                    CodecError.ErrorCase.MULTIFORMAT -> error.multiformat.reason
                    CodecError.ErrorCase.CANONICALIZATION -> error.canonicalization.reason
                    CodecError.ErrorCase.BACKEND -> error.backend.reason
                    else -> error.boundary.reason
                }
            val mapped = exceptionForCodecError(error)
            assertEquals(expectedReason, mapped.reason)
            assertEquals(error.origin, mapped.origin)
            if (error.origin == provider) assertIs<ReallyMeCodecException.ProviderFailure>(mapped)
            if (expectedReason == CodecErrorReason.CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC) {
                assertIs<ReallyMeCodecException.UnsupportedCodec>(mapped)
            }
            if (expectedReason == CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_NON_CANONICAL_JSON) {
                assertIs<ReallyMeCodecException.NonCanonical>(mapped)
            }
        }
    }

    @Test
    fun malformedEnvelopesAndLocalErrorsHaveNoFabricatedContext() {
        for (error in listOf(
            boundary(699, CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER),
            boundary(101, CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER),
            boundary(601, CodecErrorOrigin.CODEC_ERROR_ORIGIN_PROVIDER),
            CodecError.getDefaultInstance(),
        )) {
            val mapped = exceptionForCodecError(error)
            assertIs<ReallyMeCodecException.ProviderFailure>(mapped)
            assertNull(mapped.reason)
            assertNull(mapped.origin)
        }
        assertNull(ReallyMeCodecException.InvalidInput().reason)
        assertNull(ReallyMeCodecException.ProviderFailure().origin)
    }

    private fun boundary(
        reason: Int,
        origin: CodecErrorOrigin,
    ): CodecError =
        CodecError
            .newBuilder()
            .setOrigin(origin)
            .setBoundary(
                CodecBoundaryError.newBuilder().setReasonValue(reason),
            ).build()
}
