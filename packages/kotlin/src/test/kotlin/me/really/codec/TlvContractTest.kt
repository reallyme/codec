// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import com.google.gson.JsonParser
import com.google.protobuf.ByteString
import me.really.codec.v1.CodecApiOperationRequest
import me.really.codec.v1.CodecApiOperationResponse
import me.really.codec.v1.CodecTlvProfile
import me.really.codec.v1.codecApiOperationRequest
import me.really.codec.v1.codecTlvLimits
import me.really.codec.v1.codecTlvOptions
import me.really.codec.v1.codecTlvSourceRequest
import java.io.File
import java.util.Base64
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals

class TlvContractTest {
    @Test
    fun tlvReceiptsAndTypedFailuresMatchIndependentWireGoldens() {
        System
            .getProperty("reallyme.codec.testLibraryPath")
            ?.takeIf { it.isNotBlank() }
            ?.let { ReallyMeCodecRustNativeProvider.loadLibrary(it) }
        val root = File(System.getProperty("user.dir"))
        val file =
            listOf(File(root, "vectors/tlv-framing-vectors.json"), File(root, "../../vectors/tlv-framing-vectors.json"))
                .first { it.isFile }
        val manifest = JsonParser.parseString(file.readText(Charsets.UTF_8)).asJsonObject
        assertEquals(1, manifest.get("schemaVersion").asInt)
        for (entry in manifest.getAsJsonArray("cases")) {
            val vector = entry.asJsonObject
            val label = vector.get("name").asString + " " + vector.get("operation").asString
            val binary =
                vector
                    .get("requestHex")
                    .asString
                    .chunked(2)
                    .map { it.toInt(16).toByte() }
                    .toByteArray()
            val expected =
                vector
                    .get("responseHex")
                    .asString
                    .chunked(2)
                    .map { it.toInt(16).toByte() }
                    .toByteArray()
            val json = vector.get("requestJson").asString.toByteArray(Charsets.UTF_8)
            val binaryResponse = ReallyMeCodec.processOperation(binary)
            val jsonResponse = ReallyMeCodec.processOperationJson(json)
            try {
                assertContentEquals(CodecApiOperationRequest.parseFrom(binary).toByteArray(), binary, label)
                assertContentEquals(binaryResponse, expected, label)
                assertContentEquals(jsonResponse, expected, label)
                assertContentEquals(CodecApiOperationResponse.parseFrom(expected).toByteArray(), expected, label)
            } finally {
                binary.fill(0)
                expected.fill(0)
                json.fill(0)
                binaryResponse.fill(0)
                jsonResponse.fill(0)
            }
        }
    }

    @Test
    fun tlvAcceptsAnExplicitSixteenMiBSourceThroughJni() {
        val maximum = 16 * 1024 * 1024
        val source = ByteArray(maximum)
        source[0] = 4
        val request =
            codecApiOperationRequest {
                tlvReadHeader =
                    codecTlvSourceRequest {
                        encoded = ByteString.copyFrom(source)
                        options =
                            codecTlvOptions {
                                profile = CodecTlvProfile.CODEC_TLV_PROFILE_ISO7816_DEFINITE_BER
                                limits = codecTlvLimits { inputBytes = maximum.toLong() }
                            }
                    }
            }
        val encoded = Base64.getEncoder().encodeToString(source)
        val json =
            """
            {"tlvReadHeader":{"encoded":"$encoded","options":{"profile":"CODEC_TLV_PROFILE_ISO7816_DEFINITE_BER",
            "limits":{"inputBytes":"$maximum"}}}}
            """.trimIndent().encodeToByteArray()
        val binary = ReallyMeCodec.processOperation(request.toByteArray())
        assertContentEquals(binary, ReallyMeCodec.processOperationJson(json))
        val response = CodecApiOperationResponse.parseFrom(binary)
        assertEquals(2L, response.result.tlvReadHeader.consumedBytes)
        assertEquals(0L, response.result.tlvReadHeader.declaredValueLength)
    }
}
