// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import com.google.gson.JsonParser
import me.really.codec.v1.CodecApiError
import me.really.codec.v1.CodecApiOperationRequest
import me.really.codec.v1.CodecApiOperationResponse
import me.really.codec.v1.CodecErrorOrigin
import me.really.codec.v1.CodecErrorReason
import java.io.File
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals

class Base64LimitVectorTest {
    @Test
    fun sharedBoundedBase64WireBytesAndTypedOutcomes() {
        System
            .getProperty("reallyme.codec.testLibraryPath")
            ?.takeIf { it.isNotBlank() }
            ?.let { ReallyMeCodecRustNativeProvider.loadLibrary(it) }
        val root = File(System.getProperty("user.dir"))
        val file =
            listOf(File(root, "vectors/codec-vectors.json"), File(root, "../../vectors/codec-vectors.json"))
                .first { it.isFile }
        val manifest = JsonParser.parseString(file.readText(Charsets.UTF_8)).asJsonObject
        assertEquals(2, manifest.get("schemaVersion").asInt)
        val vectors = manifest.getAsJsonObject("base64DecodeWithLimits")
        assertEquals("golden-operation", vectors.get("fixtureClass").asString)
        val cases = vectors.getAsJsonArray("cases")
        assertEquals(16, cases.size())
        for (element in cases) {
            val fixture = element.asJsonObject
            val name = fixture.get("name").asString
            val requestBytes = fixture.get("requestHex").asString.hexToBytes()
            val expectedBytes = fixture.get("responseHex").asString.hexToBytes()
            val request = CodecApiOperationRequest.parseFrom(requestBytes)
            assertEquals(CodecApiOperationRequest.OperationCase.BASE64_DECODE_WITH_LIMITS, request.operationCase, name)
            assertContentEquals(requestBytes, request.toByteArray(), name)
            val actualBytes = ReallyMeCodec.processOperation(requestBytes)
            try {
                assertContentEquals(expectedBytes, actualBytes, name)
                val actual = CodecApiOperationResponse.parseFrom(actualBytes)
                val expected = CodecApiOperationResponse.parseFrom(expectedBytes)
                assertEquals(expected, actual, name)
                if (actual.outcomeCase == CodecApiOperationResponse.OutcomeCase.ERROR) {
                    assertEquals(CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER, actual.error.origin, name)
                    assertEquals(CodecApiError.ErrorCase.BASE_ENCODING, actual.error.errorCase, name)
                    assertNotEquals(CodecErrorReason.CODEC_ERROR_REASON_UNSPECIFIED, actual.error.baseEncoding.reason, name)
                }
            } finally {
                requestBytes.fill(0)
                expectedBytes.fill(0)
                actualBytes.fill(0)
            }
        }
    }
}
