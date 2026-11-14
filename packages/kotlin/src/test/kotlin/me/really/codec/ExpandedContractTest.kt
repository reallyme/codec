// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import com.google.protobuf.ByteString
import me.really.codec.v1.CodecApiOperationRequest
import me.really.codec.v1.CodecApiOperationResponse
import me.really.codec.v1.CodecApiOperationResult
import me.really.codec.v1.CodecBytesRequest
import me.really.codec.v1.CodecGeneralCborEncodeRequest
import me.really.codec.v1.CodecGeneralCborFloat
import me.really.codec.v1.CodecGeneralCborFloatWidth
import me.really.codec.v1.CodecGeneralCborValue
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals

class ExpandedContractTest {
    @Test
    fun generatedOperationsPreserveScalarAndIeeeBytes() {
        System
            .getProperty("reallyme.codec.testLibraryPath")
            ?.takeIf { it.isNotBlank() }
            ?.let { ReallyMeCodecRustNativeProvider.loadLibrary(it) }
        val scalar =
            CodecApiOperationRequest
                .newBuilder()
                .setBase64Encode(
                    CodecBytesRequest.newBuilder().setValue(ByteString.copyFrom(byteArrayOf(0xfb.toByte(), 0xff.toByte()))),
                ).build()
        val scalarResult = CodecApiOperationResponse.parseFrom(ReallyMeCodec.processOperation(scalar.toByteArray()))
        assertEquals(CodecApiOperationResponse.OutcomeCase.RESULT, scalarResult.outcomeCase)
        assertEquals(CodecApiOperationResult.ResultCase.BASE64_ENCODE, scalarResult.result.resultCase)
        assertEquals("+/8=", scalarResult.result.base64Encode.value)
        val cbor =
            CodecApiOperationRequest
                .newBuilder()
                .setGeneralCborEncode(
                    CodecGeneralCborEncodeRequest.newBuilder().setValue(
                        CodecGeneralCborValue.newBuilder().setFloatValue(
                            CodecGeneralCborFloat
                                .newBuilder()
                                .setWidth(CodecGeneralCborFloatWidth.CODEC_GENERAL_CBOR_FLOAT_WIDTH_BINARY16)
                                .setBits(0xfe01L),
                        ),
                    ),
                ).build()
        val cborResult = CodecApiOperationResponse.parseFrom(ReallyMeCodec.processOperation(cbor.toByteArray()))
        assertEquals(CodecApiOperationResponse.OutcomeCase.RESULT, cborResult.outcomeCase)
        assertEquals(CodecApiOperationResult.ResultCase.GENERAL_CBOR_ENCODE, cborResult.result.resultCase)
        assertContentEquals(
            byteArrayOf(0xf9.toByte(), 0xfe.toByte(), 1),
            cborResult.result.generalCborEncode.value
                .toByteArray(),
        )
    }
}
