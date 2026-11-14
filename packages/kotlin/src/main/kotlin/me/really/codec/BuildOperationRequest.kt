// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import me.really.codec.v1.CodecDagCborVerifyCidRequest
import me.really.codec.v1.CodecMulticodecLookupPrefixRequest
import me.really.codec.v1.CodecMulticodecPrefixForNameRequest
import me.really.codec.v1.CodecMulticodecTableRequest
import me.really.codec.v1.CodecMultikeyParseRequest
import me.really.codec.v1.CodecOperationRequest
import me.really.codec.v1.CodecPemDecodeOptions
import me.really.codec.v1.CodecPemDecodeRequest
import me.really.codec.v1.CodecPemEncodeOptions
import me.really.codec.v1.CodecPemEncodeRequest

internal fun multicodecPrefixForNameRequest(name: String): CodecOperationRequest {
    requireBoundaryAggregate(utf8Length(name))
    return CodecOperationRequest
        .newBuilder()
        .setMulticodecPrefixForName(
            CodecMulticodecPrefixForNameRequest.newBuilder().setName(name),
        ).build()
}

internal fun multicodecLookupPrefixRequest(bytes: ByteArray): CodecOperationRequest {
    requireBoundaryAggregate(bytes.size)
    return CodecOperationRequest
        .newBuilder()
        .setMulticodecLookupPrefix(
            CodecMulticodecLookupPrefixRequest
                .newBuilder()
                .setValue(borrowedByteString(bytes)),
        ).build()
}

internal fun multicodecTableRequest(): CodecOperationRequest =
    CodecOperationRequest
        .newBuilder()
        .setMulticodecTable(CodecMulticodecTableRequest.getDefaultInstance())
        .build()

internal fun multikeyParseRequest(multikey: String): CodecOperationRequest {
    requireBoundaryAggregate(utf8Length(multikey))
    return CodecOperationRequest
        .newBuilder()
        .setMultikeyParse(CodecMultikeyParseRequest.newBuilder().setMultikey(multikey))
        .build()
}

internal fun dagCborVerifyCidRequest(
    cid: String,
    bytes: ByteArray,
): CodecOperationRequest {
    requireBoundaryAggregate(utf8Length(cid), bytes.size)
    return CodecOperationRequest
        .newBuilder()
        .setDagCborVerifyCid(
            CodecDagCborVerifyCidRequest
                .newBuilder()
                .setCid(cid)
                .setPayload(borrowedByteString(bytes)),
        ).build()
}

internal fun pemDecodeRequest(
    pem: ByteArray,
    options: ReallyMePemDecodeOptions,
): CodecOperationRequest {
    requireBoundaryAggregate(pem.size)
    val optionBuilder =
        CodecPemDecodeOptions
            .newBuilder()
            .setMaxInputLen(options.maxInputLen)
            .setMaxDerLen(options.maxDerLen)
    for (label in options.allowedLabels) {
        optionBuilder.addAllowedLabels(protoPemLabel(label))
    }
    return CodecOperationRequest
        .newBuilder()
        .setPemDecode(
            CodecPemDecodeRequest
                .newBuilder()
                .setPem(borrowedByteString(pem))
                .setOptions(optionBuilder),
        ).build()
}

internal fun pemEncodeRequest(
    label: ReallyMePemLabel,
    der: ByteArray,
    options: ReallyMePemEncodeOptions,
): CodecOperationRequest {
    requireBoundaryAggregate(der.size)
    val optionBuilder =
        CodecPemEncodeOptions
            .newBuilder()
            .setMaxDerLen(options.maxDerLen)
            .setLineWidth(options.lineWidth)
            .setLineEnding(protoPemLineEnding(options.lineEnding))
    return CodecOperationRequest
        .newBuilder()
        .setPemEncode(
            CodecPemEncodeRequest
                .newBuilder()
                .setLabel(protoPemLabel(label))
                .setDer(borrowedByteString(der))
                .setOptions(optionBuilder),
        ).build()
}
