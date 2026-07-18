// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec

import com.google.protobuf.ByteString
import me.really.codec.v1.CodecDagCborVerifyCidResult
import me.really.codec.v1.CodecKeyMaterialKind
import me.really.codec.v1.CodecMulticodecLookupResult
import me.really.codec.v1.CodecMulticodecSpec
import me.really.codec.v1.CodecMulticodecTableResult
import me.really.codec.v1.CodecTag
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class MulticodecStage6ParityTest {
    private companion object {
        private val UNKNOWN_VARINT_FIELD = byteArrayOf(0x98.toByte(), 0x06, 0x01)

        private fun protobufMetadata(): CodecMulticodecSpec =
            CodecMulticodecSpec.newBuilder()
                .setName("ed25519-pub")
                .setAlgorithmName("Ed25519")
                .setTag(CodecTag.CODEC_TAG_KEY)
                .setKeyMaterialKind(
                    CodecKeyMaterialKind.CODEC_KEY_MATERIAL_KIND_PUBLIC_KEY,
                )
                .setCode(ByteString.copyFrom(byteArrayOf(0xed.toByte(), 0x01)))
                .setPrefix(ByteString.copyFrom(byteArrayOf(0xed.toByte(), 0x01)))
                .setFixedLength(32)
                .build()
    }

    @Test
    fun generatedMulticodecResultsMapEverySdkField() {
        val protobufMetadata = protobufMetadata()
        val metadata = sdkMulticodecMetadata(protobufMetadata)
        assertEquals(protobufMetadata.name, metadata.name)
        assertEquals(protobufMetadata.algorithmName, metadata.algorithmName)
        assertEquals(ReallyMeMulticodecTag.KEY, metadata.tag)
        assertEquals(ReallyMeKeyMaterialKind.PUBLIC_KEY, metadata.keyMaterialKind)
        assertContentEquals(protobufMetadata.prefix.toByteArray(), metadata.prefix())
        assertEquals(32L, metadata.expectedKeyLength)

        val protobufLookup = CodecMulticodecLookupResult.newBuilder()
            .setName(protobufMetadata.name)
            .setPrefixLength(protobufMetadata.prefix.size())
            .setMetadata(protobufMetadata)
            .build()
        val lookup = sdkMulticodecLookupResult(protobufLookup)
        assertEquals(protobufLookup.name, lookup.name)
        assertEquals(protobufLookup.prefixLength.toLong(), lookup.prefixLength)
        assertEquals(metadata.name, lookup.metadata.name)
        assertEquals(metadata.algorithmName, lookup.metadata.algorithmName)
        assertEquals(metadata.tag, lookup.metadata.tag)
        assertEquals(metadata.keyMaterialKind, lookup.metadata.keyMaterialKind)
        assertContentEquals(metadata.prefix(), lookup.metadata.prefix())
        assertEquals(metadata.expectedKeyLength, lookup.metadata.expectedKeyLength)

        val protobufTable = CodecMulticodecTableResult.newBuilder()
            .addEntries(protobufMetadata)
            .build()
        val table = sdkMulticodecTable(protobufTable)
        assertEquals(1, table.entries.size)
        assertEquals(metadata.name, table.entries.single().name)
        assertContentEquals(metadata.prefix(), table.entries.single().prefix())
    }

    @Test
    fun generatedMulticodecResultsRejectUnknownFieldsAtEveryLevel() {
        val protobufMetadata = protobufMetadata()
        val metadataWithUnknownField = CodecMulticodecSpec.parseFrom(
            protobufMetadata.toByteArray() + UNKNOWN_VARINT_FIELD,
        )
        assertFailsWith<ReallyMeCodecException.ProviderFailure> {
            sdkMulticodecMetadata(metadataWithUnknownField)
        }

        val protobufLookup = CodecMulticodecLookupResult.newBuilder()
            .setName(protobufMetadata.name)
            .setPrefixLength(protobufMetadata.prefix.size())
            .setMetadata(protobufMetadata)
            .build()
        val lookupWithUnknownField = CodecMulticodecLookupResult.parseFrom(
            protobufLookup.toByteArray() + UNKNOWN_VARINT_FIELD,
        )
        assertFailsWith<ReallyMeCodecException.ProviderFailure> {
            sdkMulticodecLookupResult(lookupWithUnknownField)
        }
        assertFailsWith<ReallyMeCodecException.ProviderFailure> {
            sdkMulticodecLookupResult(
                protobufLookup.toBuilder()
                    .setMetadata(metadataWithUnknownField)
                    .build(),
            )
        }

        val protobufTable = CodecMulticodecTableResult.newBuilder()
            .addEntries(protobufMetadata)
            .build()
        val tableWithUnknownField = CodecMulticodecTableResult.parseFrom(
            protobufTable.toByteArray() + UNKNOWN_VARINT_FIELD,
        )
        assertFailsWith<ReallyMeCodecException.ProviderFailure> {
            sdkMulticodecTable(tableWithUnknownField)
        }
        assertFailsWith<ReallyMeCodecException.ProviderFailure> {
            sdkMulticodecTable(
                protobufTable.toBuilder()
                    .setEntries(0, metadataWithUnknownField)
                    .build(),
            )
        }
    }

    @Test
    fun publicStructuredProviderResultsRejectUnknownFields() {
        val dagCborResult = CodecDagCborVerifyCidResult.newBuilder()
            .setValid(true)
            .setExpectedCid("expected")
            .setActualCid("actual")
            .build()

        val resultWithUnknownField = CodecDagCborVerifyCidResult.parseFrom(
            dagCborResult.toByteArray() + UNKNOWN_VARINT_FIELD,
        )
        assertFailsWith<ReallyMeCodecException.ProviderFailure> {
            sdkDagCborCidVerification(resultWithUnknownField)
        }
    }
}
