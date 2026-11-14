// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Generated from reallyme/codec/v1/codec.proto. Do not edit.

package me.really.codec

import me.really.codec.v1.CodecKeyMaterialKind
import me.really.codec.v1.CodecPemLabel
import me.really.codec.v1.CodecPemLineEnding
import me.really.codec.v1.CodecTag

public enum class ReallyMeMulticodecTag {
    ENCRYPTION,
    HASH,
    KEY,
    MULTIHASH,
    MULTIKEY,
}

internal fun sdkMulticodecTag(tag: CodecTag): ReallyMeMulticodecTag = when (tag) {
    CodecTag.CODEC_TAG_ENCRYPTION -> ReallyMeMulticodecTag.ENCRYPTION
    CodecTag.CODEC_TAG_HASH -> ReallyMeMulticodecTag.HASH
    CodecTag.CODEC_TAG_KEY -> ReallyMeMulticodecTag.KEY
    CodecTag.CODEC_TAG_MULTIHASH -> ReallyMeMulticodecTag.MULTIHASH
    CodecTag.CODEC_TAG_MULTIKEY -> ReallyMeMulticodecTag.MULTIKEY
    CodecTag.CODEC_TAG_UNSPECIFIED,
    CodecTag.UNRECOGNIZED,
    -> throw ReallyMeCodecException.ProviderFailure()
}

public enum class ReallyMeKeyMaterialKind {
    NOT_KEY,
    PUBLIC_KEY,
    PRIVATE_KEY,
    SYMMETRIC_KEY,
}

internal fun sdkKeyMaterialKind(kind: CodecKeyMaterialKind): ReallyMeKeyMaterialKind = when (kind) {
    CodecKeyMaterialKind.CODEC_KEY_MATERIAL_KIND_NOT_KEY -> ReallyMeKeyMaterialKind.NOT_KEY
    CodecKeyMaterialKind.CODEC_KEY_MATERIAL_KIND_PUBLIC_KEY -> ReallyMeKeyMaterialKind.PUBLIC_KEY
    CodecKeyMaterialKind.CODEC_KEY_MATERIAL_KIND_PRIVATE_KEY -> ReallyMeKeyMaterialKind.PRIVATE_KEY
    CodecKeyMaterialKind.CODEC_KEY_MATERIAL_KIND_SYMMETRIC_KEY -> ReallyMeKeyMaterialKind.SYMMETRIC_KEY
    CodecKeyMaterialKind.CODEC_KEY_MATERIAL_KIND_UNSPECIFIED,
    CodecKeyMaterialKind.UNRECOGNIZED,
    -> throw ReallyMeCodecException.ProviderFailure()
}

public enum class ReallyMePemLabel(public val label: String) {
    PRIVATE_KEY("PRIVATE KEY"),
    EC_PRIVATE_KEY("EC PRIVATE KEY"),
    PUBLIC_KEY("PUBLIC KEY"),
}

internal fun protoPemLabel(value: ReallyMePemLabel): CodecPemLabel = when (value) {
    ReallyMePemLabel.PRIVATE_KEY -> CodecPemLabel.CODEC_PEM_LABEL_PRIVATE_KEY
    ReallyMePemLabel.EC_PRIVATE_KEY -> CodecPemLabel.CODEC_PEM_LABEL_EC_PRIVATE_KEY
    ReallyMePemLabel.PUBLIC_KEY -> CodecPemLabel.CODEC_PEM_LABEL_PUBLIC_KEY
 }

internal fun sdkPemLabel(label: String): ReallyMePemLabel =
    ReallyMePemLabel.entries.firstOrNull { it.label == label } ?: throw ReallyMeCodecException.ProviderFailure()

public enum class ReallyMePemLineEnding {
    LF,
    CRLF,
}

internal fun protoPemLineEnding(value: ReallyMePemLineEnding?): CodecPemLineEnding = when (value) {
    ReallyMePemLineEnding.LF -> CodecPemLineEnding.CODEC_PEM_LINE_ENDING_LF
    ReallyMePemLineEnding.CRLF -> CodecPemLineEnding.CODEC_PEM_LINE_ENDING_CRLF
    null -> CodecPemLineEnding.CODEC_PEM_LINE_ENDING_UNSPECIFIED
 }
