// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import com.google.gson.JsonParser
import me.really.codec.v1.CodecApiError
import me.really.codec.v1.CodecApiOperationRequest
import me.really.codec.v1.CodecApiOperationResponse
import me.really.codec.v1.CodecApiOperationResult
import me.really.codec.v1.CodecErrorOrigin
import me.really.codec.v1.CodecErrorReason
import java.io.File
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue
import kotlin.test.fail

class PemBundleVectorTest {
    @Test
    fun sharedPemBundleWireBytesAndTypedOutcomes() {
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
        val vectors = manifest.getAsJsonObject("pemBundleDecode")
        assertEquals("golden-operation", vectors.get("fixtureClass").asString)
        val cases = vectors.getAsJsonArray("cases")
        assertTrue(cases.size() > 0)
        for (element in cases) {
            val fixture = element.asJsonObject
            val name = fixture.get("name").asString
            val requestBytes = fixture.get("requestHex").asString.hexToBytes()
            val expectedBytes = fixture.get("responseHex").asString.hexToBytes()
            val request = CodecApiOperationRequest.parseFrom(requestBytes)
            assertEquals(CodecApiOperationRequest.OperationCase.PEM_BUNDLE_DECODE, request.operationCase, name)
            assertContentEquals(requestBytes, request.toByteArray(), name)
            val actualBytes = ReallyMeCodec.processOperation(requestBytes)
            try {
                assertContentEquals(expectedBytes, actualBytes, name)
                val actual = CodecApiOperationResponse.parseFrom(actualBytes)
                val expected = CodecApiOperationResponse.parseFrom(expectedBytes)
                assertEquals(expected.outcomeCase, actual.outcomeCase, name)
                if (actual.outcomeCase == CodecApiOperationResponse.OutcomeCase.RESULT) {
                    assertEquals(CodecApiOperationResult.ResultCase.PEM_BUNDLE_DECODE, actual.result.resultCase, name)
                    val blocks = actual.result.pemBundleDecode.blocksList
                    val expectedBlocks = expected.result.pemBundleDecode.blocksList
                    assertEquals(expectedBlocks.size, blocks.size, name)
                    for ((block, expectedBlock) in blocks.zip(expectedBlocks)) {
                        assertEquals(expectedBlock.label, block.label, name)
                        assertEquals(expectedBlock.der, block.der, name)
                    }
                } else {
                    assertEquals(CodecApiOperationResponse.OutcomeCase.ERROR, actual.outcomeCase, name)
                    assertEquals(CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER, actual.error.origin, name)
                    assertEquals(expected.error.errorCase, actual.error.errorCase, name)
                    when (actual.error.errorCase) {
                        CodecApiError.ErrorCase.PEM -> {
                            assertEquals(expected.error.pem.reason, actual.error.pem.reason, name)
                            assertNotEquals(CodecErrorReason.CODEC_ERROR_REASON_UNSPECIFIED, actual.error.pem.reason, name)
                        }
                        CodecApiError.ErrorCase.BASE_ENCODING -> {
                            assertEquals(expected.error.baseEncoding.reason, actual.error.baseEncoding.reason, name)
                            assertEquals(CodecErrorReason.CODEC_ERROR_REASON_BASE_INPUT_TOO_LARGE, actual.error.baseEncoding.reason, name)
                        }
                        else -> fail("unexpected error family: $name")
                    }
                }
            } finally {
                requestBytes.fill(0)
                actualBytes.fill(0)
                expectedBytes.fill(0)
            }
        }
    }
}
