// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import com.google.protobuf.ByteString
import me.really.codec.v1.CodecDerValue
import me.really.codec.v1.CodecGeneralCborOptions
import me.really.codec.v1.CodecKeyMaterialKind
import me.really.codec.v1.CodecMulticodecSpec
import me.really.codec.v1.CodecMultikeyParseResult
import me.really.codec.v1.CodecTag
import me.really.codec.v1.CodecValidateKeyBindingRequest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class FieldPresenceTest {
    private fun metadata(
        fixed: Int,
        variable: Boolean,
    ): CodecMulticodecSpec =
        CodecMulticodecSpec
            .newBuilder()
            .setTag(CodecTag.CODEC_TAG_KEY)
            .setKeyMaterialKind(CodecKeyMaterialKind.CODEC_KEY_MATERIAL_KIND_PUBLIC_KEY)
            .setFixedLength(fixed)
            .setVariableLength(variable)
            .build()

    @Test
    fun metadataLengthStates() {
        assertFailsWith<ReallyMeCodecException.ProviderFailure> { sdkMulticodecMetadata(metadata(32, true)) }
        assertEquals(32L, sdkMulticodecMetadata(metadata(32, false)).expectedKeyLength)
        assertNull(sdkMulticodecMetadata(metadata(0, true)).expectedKeyLength)
        assertNull(sdkMulticodecMetadata(metadata(0, false)).expectedKeyLength)
    }

    @Test
    fun parsedKeyLengthStates() {
        val value = CodecMultikeyParseResult.newBuilder().setPublicKey(ByteString.copyFrom(ByteArray(32)))
        for ((expected, variable) in listOf(32 to true, 0 to false, 31 to false)) {
            value.setExpectedPublicKeyLength(expected).setVariablePublicKeyLength(variable)
            assertFailsWith<ReallyMeCodecException.ProviderFailure> { sdkParsedMultikey(value.build()) }
        }
        value.setExpectedPublicKeyLength(32).setVariablePublicKeyLength(false)
        assertEquals(32L, sdkParsedMultikey(value.build()).expectedPublicKeyLength)
        value.setExpectedPublicKeyLength(0).setVariablePublicKeyLength(true)
        assertNull(sdkParsedMultikey(value.build()).expectedPublicKeyLength)
    }

    @Test
    fun generatedPresencePreservesDefaultsInBinary() {
        assertFalse(CodecValidateKeyBindingRequest.getDefaultInstance().hasAlgorithm())
        val algorithm = CodecValidateKeyBindingRequest.newBuilder().setAlgorithm("").build()
        assertContentEquals(byteArrayOf(0x12, 0), algorithm.toByteArray())
        assertTrue(CodecValidateKeyBindingRequest.parseFrom(algorithm.toByteArray()).hasAlgorithm())
        assertEquals("", algorithm.algorithm)

        assertFalse(CodecGeneralCborOptions.getDefaultInstance().hasInputBytes())
        val limits = CodecGeneralCborOptions.newBuilder().setInputBytes(0).build()
        assertContentEquals(byteArrayOf(0x28, 0), limits.toByteArray())
        assertTrue(CodecGeneralCborOptions.parseFrom(limits.toByteArray()).hasInputBytes())

        assertFalse(CodecDerValue.getDefaultInstance().hasBoolValue())
        val der = CodecDerValue.newBuilder().setBoolValue(false).build()
        assertContentEquals(byteArrayOf(0x28, 0), der.toByteArray())
        assertTrue(CodecDerValue.parseFrom(der.toByteArray()).hasBoolValue())
    }
}
