// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import com.google.protobuf.ByteString
import me.really.codec.v1.CodecApiOperationRequest
import me.really.codec.v1.CodecApiOperationResponse
import me.really.codec.v1.CodecApiOperationResult
import me.really.codec.v1.CodecGeneralCborErrorReason
import me.really.codec.v1.CodecGeneralCborFloatPolicy
import me.really.codec.v1.CodecGeneralCborLengthPolicy
import me.really.codec.v1.CodecGeneralCborMapOrdering
import me.really.codec.v1.CodecGeneralCborOptions
import me.really.codec.v1.CodecGeneralCborSerialization
import me.really.codec.v1.CodecGeneralCborSourceRequest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class GeneralCborReceiptOptionsTest {
    @Test
    fun explicitTwoMiBBudgetPassesNativeTransportWithoutChangingDefaults() {
        System
            .getProperty("reallyme.codec.testLibraryPath")
            ?.takeIf { it.isNotBlank() }
            ?.let { ReallyMeCodecRustNativeProvider.loadLibrary(it) }
        val encoded = ByteArray(2_097_152)
        encoded.fill(0xa5.toByte())
        byteArrayOf(0x5a, 0x00, 0x1f, 0xff.toByte(), 0xfb.toByte()).copyInto(encoded)
        try {
            val source = CodecGeneralCborSourceRequest.newBuilder().setEncoded(ByteString.copyFrom(encoded))
            val request = CodecApiOperationRequest.newBuilder().setGeneralCborValidateExact(source)
            val rejected = CodecApiOperationResponse.parseFrom(ReallyMeCodec.processOperation(request.build().toByteArray()))
            assertEquals(CodecApiOperationResponse.OutcomeCase.ERROR, rejected.outcomeCase)
            assertEquals(CodecGeneralCborErrorReason.CODEC_GENERAL_CBOR_ERROR_REASON_INPUT_LIMIT, rejected.error.generalCbor.detail)
            val options =
                CodecGeneralCborOptions
                    .newBuilder()
                    .setSerialization(CodecGeneralCborSerialization.CODEC_GENERAL_CBOR_SERIALIZATION_RECEIVED)
                    .setLengths(CodecGeneralCborLengthPolicy.CODEC_GENERAL_CBOR_LENGTH_POLICY_DEFINITE_ONLY)
                    .setFloats(CodecGeneralCborFloatPolicy.CODEC_GENERAL_CBOR_FLOAT_POLICY_ALL)
                    .setOrdering(CodecGeneralCborMapOrdering.CODEC_GENERAL_CBOR_MAP_ORDERING_RECEIVED)
                    .setInputBytes(encoded.size.toLong())
                    .setByteStringBytes(encoded.size.toLong())
            source.setOptions(options)
            request.setGeneralCborValidateExact(source)
            val accepted = CodecApiOperationResponse.parseFrom(ReallyMeCodec.processOperation(request.build().toByteArray()))
            assertEquals(CodecApiOperationResponse.OutcomeCase.RESULT, accepted.outcomeCase)
            val frame = accepted.result.generalCborValidateExact
            assertEquals(encoded.size.toLong(), frame.received.length)
            assertEquals(encoded.size.toLong(), frame.options.inputBytes)
            assertEquals(1_048_576L, frame.options.outputBytes)
        } finally {
            encoded.fill(0)
        }
    }

    @Test
    fun collectionOptionsAreSharedAndStandaloneOptionsRemainPresent() {
        System
            .getProperty("reallyme.codec.testLibraryPath")
            ?.takeIf { it.isNotBlank() }
            ?.let { ReallyMeCodecRustNativeProvider.loadLibrary(it) }
        val source = CodecGeneralCborSourceRequest.newBuilder().setEncoded(ByteString.copyFrom(byteArrayOf(0, 1)))
        val request = CodecApiOperationRequest.newBuilder().setGeneralCborCursor(source)
        val response = CodecApiOperationResponse.parseFrom(ReallyMeCodec.processOperation(request.build().toByteArray()))
        assertEquals(CodecApiOperationResponse.OutcomeCase.RESULT, response.outcomeCase)
        assertEquals(CodecApiOperationResult.ResultCase.GENERAL_CBOR_CURSOR, response.result.resultCase)
        val frames = response.result.generalCborCursor
        assertTrue(frames.hasOptions())
        assertEquals(4096L, frames.options.entries)
        assertEquals(65536L, frames.options.nodes)
        assertEquals(2, frames.valuesCount)
        assertEquals(2L, frames.consumedBytes)
        assertTrue(frames.valuesList.all { !it.hasOptions() })

        source.setEncoded(ByteString.copyFrom(byteArrayOf(0)))
        request.setGeneralCborFrameExact(source)
        val standalone = CodecApiOperationResponse.parseFrom(ReallyMeCodec.processOperation(request.build().toByteArray()))
        assertEquals(CodecApiOperationResponse.OutcomeCase.RESULT, standalone.outcomeCase)
        assertEquals(CodecApiOperationResult.ResultCase.GENERAL_CBOR_FRAME_EXACT, standalone.result.resultCase)
        assertTrue(standalone.result.generalCborFrameExact.hasOptions())
        assertEquals(frames.options, standalone.result.generalCborFrameExact.options)
    }
}
