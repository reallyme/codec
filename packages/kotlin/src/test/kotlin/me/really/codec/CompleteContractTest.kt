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

class CompleteContractTest {
    @Test
    fun everySchemaOperationPreservesBinaryAndJsonResponses() {
        System
            .getProperty("reallyme.codec.testLibraryPath")
            ?.takeIf { it.isNotBlank() }
            ?.let { ReallyMeCodecRustNativeProvider.loadLibrary(it) }
        val root = File(System.getProperty("user.dir"))
        val file =
            listOf(File(root, "vectors/codec-operation-vectors.json"), File(root, "../../vectors/codec-operation-vectors.json"))
                .first { it.isFile }
        val fixture = JsonParser.parseString(file.readText(Charsets.UTF_8)).asJsonObject
        assertEquals(1, fixture.get("schemaVersion").asInt)
        for (entry in fixture.getAsJsonArray("cases")) {
            val vector = entry.asJsonObject
            val operation = vector.get("operation").asString
            val binary =
                vector
                    .get("requestHex")
                    .asString
                    .chunked(2)
                    .map { it.toInt(16).toByte() }
                    .toByteArray()
            val json = vector.get("requestJson").asString.toByteArray(Charsets.UTF_8)
            val binaryResponse = ReallyMeCodec.processOperation(binary)
            val jsonResponse = ReallyMeCodec.processOperationJson(json)
            try {
                assertEquals(vector.get("fieldNumber").asInt, CodecApiOperationRequest.parseFrom(binary).operationCase.number)
                assertContentEquals(binaryResponse, jsonResponse, operation)
                val response = CodecApiOperationResponse.parseFrom(binaryResponse)
                assertEquals(CodecApiOperationResponse.OutcomeCase.RESULT, response.outcomeCase, operation)
                assertEquals(vector.get("fieldNumber").asInt, response.result.resultCase.number, operation)
                val invalid = JsonParser.parseString(vector.get("requestJson").asString).asJsonObject
                invalid.getAsJsonObject(operation).addProperty("unexpected", true)
                val invalidRequest = invalid.toString().toByteArray(Charsets.UTF_8)
                val invalidResponse = ReallyMeCodec.processOperationJson(invalidRequest)
                try {
                    val error = CodecApiOperationResponse.parseFrom(invalidResponse)
                    assertEquals(CodecApiOperationResponse.OutcomeCase.ERROR, error.outcomeCase)
                    assertEquals(CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER, error.error.origin)
                    assertEquals(CodecApiError.ErrorCase.BOUNDARY, error.error.errorCase)
                    assertEquals(CodecErrorReason.CODEC_ERROR_REASON_BOUNDARY_MALFORMED_JSON, error.error.boundary.reason)
                } finally {
                    invalidRequest.fill(0)
                    invalidResponse.fill(0)
                }
            } finally {
                binary.fill(0)
                json.fill(0)
                binaryResponse.fill(0)
                jsonResponse.fill(0)
            }
        }
    }
}
