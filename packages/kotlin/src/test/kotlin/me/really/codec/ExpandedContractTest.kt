// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import com.google.protobuf.ByteString
import me.really.codec.v1.CodecApiOperationRequest
import me.really.codec.v1.CodecApiOperationResponse
import me.really.codec.v1.CodecApiOperationResult
import me.really.codec.v1.CodecBytesRequest
import me.really.codec.v1.CodecErrorReason
import me.really.codec.v1.CodecGeneralCborEncodeRequest
import me.really.codec.v1.CodecGeneralCborFloat
import me.really.codec.v1.CodecGeneralCborFloatWidth
import me.really.codec.v1.CodecGeneralCborValue
import me.really.codec.v1.CodecJsonDecodeRequest
import me.really.codec.v1.CodecPemBundleDecodeOptions
import me.really.codec.v1.CodecPemBundleDecodeRequest
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

    @Test
    fun generatedPemBundlePreservesBytesAndTypedResourceFailure() {
        System
            .getProperty("reallyme.codec.testLibraryPath")
            ?.takeIf { it.isNotBlank() }
            ?.let { ReallyMeCodecRustNativeProvider.loadLibrary(it) }
        val block = "-----BEGIN CERTIFICATE-----\rAQID\r-----END CERTIFICATE-----\r"
        val pem = CodecPemBundleDecodeRequest.newBuilder().setPem(ByteString.copyFromUtf8(block + block))
        val request = CodecApiOperationRequest.newBuilder().setPemBundleDecode(pem)
        val response = CodecApiOperationResponse.parseFrom(ReallyMeCodec.processOperation(request.build().toByteArray()))
        assertEquals(CodecApiOperationResponse.OutcomeCase.RESULT, response.outcomeCase)
        assertEquals(CodecApiOperationResult.ResultCase.PEM_BUNDLE_DECODE, response.result.resultCase)
        assertEquals(2, response.result.pemBundleDecode.blocksCount)
        assertEquals(
            "CERTIFICATE",
            response.result.pemBundleDecode
                .getBlocks(0)
                .label,
        )
        assertContentEquals(
            byteArrayOf(1, 2, 3),
            response.result.pemBundleDecode
                .getBlocks(0)
                .der
                .toByteArray(),
        )
        pem.setOptions(CodecPemBundleDecodeOptions.newBuilder().setMaxBlocks(1))
        request.setPemBundleDecode(pem)
        val rejected = CodecApiOperationResponse.parseFrom(ReallyMeCodec.processOperation(request.build().toByteArray()))
        assertEquals(CodecApiOperationResponse.OutcomeCase.ERROR, rejected.outcomeCase)
        assertEquals(CodecErrorReason.CODEC_ERROR_REASON_PEM_RESOURCE_LIMIT_EXCEEDED, rejected.error.pem.reason)
    }

    @Test
    fun generatedGuardedJsonPreservesUnicodeAndDuplicateReason() {
        System
            .getProperty("reallyme.codec.testLibraryPath")
            ?.takeIf { it.isNotBlank() }
            ?.let { ReallyMeCodecRustNativeProvider.loadLibrary(it) }
        val json =
            CodecJsonDecodeRequest
                .newBuilder()
                .setEncoded(ByteString.copyFromUtf8("""{"hello":"world\u0021"}"""))
        val request = CodecApiOperationRequest.newBuilder().setJsonDecode(json)
        val response = CodecApiOperationResponse.parseFrom(ReallyMeCodec.processOperation(request.build().toByteArray()))
        assertEquals(CodecApiOperationResponse.OutcomeCase.RESULT, response.outcomeCase)
        assertEquals(CodecApiOperationResult.ResultCase.JSON_DECODE, response.result.resultCase)
        val member =
            response.result.jsonDecode.value.objectValue
                .getMembers(0)
        assertEquals("hello", member.name)
        assertEquals("world!", member.value.stringValue)
        json.setEncoded(ByteString.copyFromUtf8("""{"x":1,"\u0078":2}"""))
        request.setJsonDecode(json)
        val rejected = CodecApiOperationResponse.parseFrom(ReallyMeCodec.processOperation(request.build().toByteArray()))
        assertEquals(CodecApiOperationResponse.OutcomeCase.ERROR, rejected.outcomeCase)
        assertEquals(CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_JSON_DUPLICATE_PROPERTY, rejected.error.canonicalization.reason)
    }
}
