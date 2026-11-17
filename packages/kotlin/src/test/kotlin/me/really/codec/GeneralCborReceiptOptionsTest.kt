// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import com.google.protobuf.ByteString
import me.really.codec.v1.CodecApiOperationRequest
import me.really.codec.v1.CodecApiOperationResponse
import me.really.codec.v1.CodecApiOperationResult
import me.really.codec.v1.CodecGeneralCborSourceRequest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class GeneralCborReceiptOptionsTest {
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
