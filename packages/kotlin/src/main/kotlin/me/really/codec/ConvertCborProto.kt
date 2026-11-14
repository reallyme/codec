// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import me.really.codec.v1.CodecDagCborDecodeRequest
import me.really.codec.v1.CodecDagCborEncodeRequest
import me.really.codec.v1.CodecDeterministicCborArray
import me.really.codec.v1.CodecDeterministicCborBool
import me.really.codec.v1.CodecDeterministicCborBytes
import me.really.codec.v1.CodecDeterministicCborDecodeRequest
import me.really.codec.v1.CodecDeterministicCborEncodeRequest
import me.really.codec.v1.CodecDeterministicCborInteger
import me.really.codec.v1.CodecDeterministicCborMap
import me.really.codec.v1.CodecDeterministicCborMapEntry
import me.really.codec.v1.CodecDeterministicCborMapKey
import me.really.codec.v1.CodecDeterministicCborNegativeInteger
import me.really.codec.v1.CodecDeterministicCborNull
import me.really.codec.v1.CodecDeterministicCborText
import me.really.codec.v1.CodecDeterministicCborUnsignedInteger
import me.really.codec.v1.CodecDeterministicCborValue
import me.really.codec.v1.CodecOperationRequest

internal fun deterministicCborEncodeRequest(value: ReallyMeDeterministicCborValue): CodecOperationRequest =
    CodecOperationRequest
        .newBuilder()
        .setDeterministicCborEncode(
            CodecDeterministicCborEncodeRequest
                .newBuilder()
                .setValue(protoValue(value)),
        ).build()

internal fun dagCborEncodeRequest(value: ReallyMeDeterministicCborValue): CodecOperationRequest =
    CodecOperationRequest
        .newBuilder()
        .setDagCborEncode(
            CodecDagCborEncodeRequest
                .newBuilder()
                .setValue(protoValue(value)),
        ).build()

internal fun deterministicCborDecodeRequest(bytes: ByteArray): CodecOperationRequest {
    requireBoundaryAggregate(bytes.size)
    return CodecOperationRequest
        .newBuilder()
        .setDeterministicCborDecode(
            CodecDeterministicCborDecodeRequest
                .newBuilder()
                .setEncoded(borrowedByteString(bytes)),
        ).build()
}

internal fun dagCborDecodeRequest(bytes: ByteArray): CodecOperationRequest {
    requireBoundaryAggregate(bytes.size)
    return CodecOperationRequest
        .newBuilder()
        .setDagCborDecode(
            CodecDagCborDecodeRequest
                .newBuilder()
                .setEncoded(borrowedByteString(bytes)),
        ).build()
}

private fun protoInteger(integer: ReallyMeDeterministicCborInteger): CodecDeterministicCborInteger =
    when (integer) {
        is ReallyMeDeterministicCborInteger.Unsigned ->
            CodecDeterministicCborInteger
                .newBuilder()
                .setUnsignedValue(
                    CodecDeterministicCborUnsignedInteger
                        .newBuilder()
                        .setValue(integer.value.toLong()),
                ).build()
        is ReallyMeDeterministicCborInteger.Negative ->
            CodecDeterministicCborInteger
                .newBuilder()
                .setNegativeValue(
                    CodecDeterministicCborNegativeInteger
                        .newBuilder()
                        .setValue(integer.value),
                ).build()
    }

private fun protoKey(key: ReallyMeDeterministicCborMapKey): CodecDeterministicCborMapKey =
    when (key) {
        is ReallyMeDeterministicCborMapKey.Integer ->
            CodecDeterministicCborMapKey
                .newBuilder()
                .setIntegerKey(protoInteger(key.value))
                .build()
        is ReallyMeDeterministicCborMapKey.Text ->
            CodecDeterministicCborMapKey
                .newBuilder()
                .setTextKey(
                    CodecDeterministicCborText
                        .newBuilder()
                        .setValue(key.value),
                ).build()
    }

private fun protoValue(value: ReallyMeDeterministicCborValue): CodecDeterministicCborValue =
    when (value) {
        ReallyMeDeterministicCborValue.Null ->
            CodecDeterministicCborValue
                .newBuilder()
                .setNullValue(CodecDeterministicCborNull.getDefaultInstance())
                .build()
        is ReallyMeDeterministicCborValue.Bool ->
            CodecDeterministicCborValue
                .newBuilder()
                .setBoolValue(
                    CodecDeterministicCborBool
                        .newBuilder()
                        .setValue(value.value),
                ).build()
        is ReallyMeDeterministicCborValue.Integer ->
            CodecDeterministicCborValue
                .newBuilder()
                .setIntegerValue(protoInteger(value.value))
                .build()
        is ReallyMeDeterministicCborValue.Text ->
            CodecDeterministicCborValue
                .newBuilder()
                .setTextValue(
                    CodecDeterministicCborText
                        .newBuilder()
                        .setValue(value.value),
                ).build()
        is ReallyMeDeterministicCborValue.Bytes ->
            CodecDeterministicCborValue
                .newBuilder()
                .setBytesValue(
                    CodecDeterministicCborBytes
                        .newBuilder()
                        .setValue(borrowedByteString(value.borrowedBytes())),
                ).build()
        is ReallyMeDeterministicCborValue.Array ->
            CodecDeterministicCborValue
                .newBuilder()
                .setArrayValue(
                    CodecDeterministicCborArray
                        .newBuilder()
                        .addAllValues(value.values.map(::protoValue)),
                ).build()
        is ReallyMeDeterministicCborValue.Map ->
            CodecDeterministicCborValue
                .newBuilder()
                .setMapValue(
                    CodecDeterministicCborMap
                        .newBuilder()
                        .addAllEntries(
                            value.entries.map { entry ->
                                CodecDeterministicCborMapEntry
                                    .newBuilder()
                                    .setKey(protoKey(entry.key))
                                    .setValue(protoValue(entry.value))
                                    .build()
                            },
                        ),
                ).build()
    }

private fun sdkInteger(integer: CodecDeterministicCborInteger): ReallyMeDeterministicCborInteger =
    when (integer.valueCase) {
        CodecDeterministicCborInteger.ValueCase.UNSIGNED_VALUE ->
            ReallyMeDeterministicCborInteger.Unsigned(integer.unsignedValue.value.toULong())
        CodecDeterministicCborInteger.ValueCase.NEGATIVE_VALUE ->
            ReallyMeDeterministicCborInteger.Negative.fromProvider(integer.negativeValue.value)
        CodecDeterministicCborInteger.ValueCase.VALUE_NOT_SET,
        null,
        -> throw ReallyMeCodecException.ProviderFailure()
    }

private fun sdkKey(key: CodecDeterministicCborMapKey): ReallyMeDeterministicCborMapKey =
    when (key.keyCase) {
        CodecDeterministicCborMapKey.KeyCase.INTEGER_KEY ->
            ReallyMeDeterministicCborMapKey.Integer(sdkInteger(key.integerKey))
        CodecDeterministicCborMapKey.KeyCase.TEXT_KEY ->
            ReallyMeDeterministicCborMapKey.Text(key.textKey.value)
        CodecDeterministicCborMapKey.KeyCase.KEY_NOT_SET,
        null,
        -> throw ReallyMeCodecException.ProviderFailure()
    }

internal fun sdkValue(value: CodecDeterministicCborValue): ReallyMeDeterministicCborValue =
    when (value.valueCase) {
        CodecDeterministicCborValue.ValueCase.NULL_VALUE ->
            ReallyMeDeterministicCborValue.Null
        CodecDeterministicCborValue.ValueCase.BOOL_VALUE ->
            ReallyMeDeterministicCborValue.Bool(value.boolValue.value)
        CodecDeterministicCborValue.ValueCase.INTEGER_VALUE ->
            ReallyMeDeterministicCborValue.Integer(sdkInteger(value.integerValue))
        CodecDeterministicCborValue.ValueCase.TEXT_VALUE ->
            ReallyMeDeterministicCborValue.Text(value.textValue.value)
        CodecDeterministicCborValue.ValueCase.BYTES_VALUE ->
            ReallyMeDeterministicCborValue.Bytes.fromOwnedProviderBytes(
                value.bytesValue.value.toByteArray(),
            )
        CodecDeterministicCborValue.ValueCase.ARRAY_VALUE ->
            ReallyMeDeterministicCborValue.Array(value.arrayValue.valuesList.map(::sdkValue))
        CodecDeterministicCborValue.ValueCase.MAP_VALUE ->
            ReallyMeDeterministicCborValue.Map(
                value.mapValue.entriesList.map { entry ->
                    if (!entry.hasKey() || !entry.hasValue()) {
                        throw ReallyMeCodecException.ProviderFailure()
                    }
                    ReallyMeDeterministicCborMapEntry(
                        key = sdkKey(entry.key),
                        value = sdkValue(entry.value),
                    )
                },
            )
        CodecDeterministicCborValue.ValueCase.VALUE_NOT_SET,
        null,
        -> throw ReallyMeCodecException.ProviderFailure()
    }

/**
 * Borrows the caller's array only for synchronous protobuf serialization.
 * `ByteString.copyFrom` would create an additional immutable payload owner
 * that the JVM cannot wipe. This wrapper creates no byte copy; the final
 * serialized request is the sole SDK-owned copy and is wiped in
 * [processOperation]. The wrapper must never escape or be retained.
 */
