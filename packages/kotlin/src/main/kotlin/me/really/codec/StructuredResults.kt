// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import me.really.codec.v1.CodecDagCborVerifyCidResult
import me.really.codec.v1.CodecMulticodecLookupResult
import me.really.codec.v1.CodecMulticodecSpec
import me.really.codec.v1.CodecMulticodecTableResult
import me.really.codec.v1.CodecMultikeyParseResult
import me.really.codec.v1.CodecPemDecodeResult

public class ReallyMeMulticodecMetadata internal constructor(
    public val name: String,
    public val algorithmName: String,
    public val tag: ReallyMeMulticodecTag,
    public val keyMaterialKind: ReallyMeKeyMaterialKind,
    prefix: ByteArray,
    public val expectedKeyLength: Long?,
) {
    private val prefixBytes: ByteArray = prefix.copyOf()

    public fun prefix(): ByteArray = prefixBytes.copyOf()

    override fun toString(): String = "ReallyMeMulticodecMetadata(<redacted>)"
}

public class ReallyMeMulticodecLookupResult internal constructor(
    public val name: String,
    public val prefixLength: Long,
    public val metadata: ReallyMeMulticodecMetadata,
) {
    override fun toString(): String = "ReallyMeMulticodecLookupResult(<redacted>)"
}

public class ReallyMeMulticodecTable internal constructor(
    entries: List<ReallyMeMulticodecMetadata>,
) {
    public val entries: List<ReallyMeMulticodecMetadata> = entries.toList()

    override fun toString(): String = "ReallyMeMulticodecTable(<redacted>)"
}

public class ReallyMeParsedMultikey internal constructor(
    public val codecName: String,
    public val algorithmName: String,
    publicKey: ByteArray,
    public val expectedPublicKeyLength: Long?,
) {
    private val publicKeyBytes: ByteArray = publicKey.copyOf()

    public fun publicKey(): ByteArray = publicKeyBytes.copyOf()

    override fun toString(): String = "ReallyMeParsedMultikey(<redacted>)"
}

public class ReallyMeDagCborCidVerification internal constructor(
    public val valid: Boolean,
    public val expectedCid: String,
    public val actualCid: String,
) {
    override fun toString(): String = "ReallyMeDagCborCidVerification(<redacted>)"
}

public class ReallyMePemDecodeOptions
    @JvmOverloads
    public constructor(
        allowedLabels: List<ReallyMePemLabel> = emptyList(),
        public val maxInputLen: Int = 0,
        public val maxDerLen: Int = 0,
    ) {
        public val allowedLabels: List<ReallyMePemLabel> = allowedLabels.toList()

        init {
            if (maxInputLen < 0 || maxDerLen < 0) {
                throw ReallyMeCodecException.InvalidInput()
            }
        }
    }

public class ReallyMePemEncodeOptions
    @JvmOverloads
    public constructor(
        public val maxDerLen: Int = 0,
        public val lineWidth: Int = 0,
        public val lineEnding: ReallyMePemLineEnding? = null,
    ) {
        init {
            if (maxDerLen < 0 || lineWidth < 0 || lineWidth > 76) {
                throw ReallyMeCodecException.InvalidInput()
            }
        }
    }

public class ReallyMePemDocument internal constructor(
    public val label: ReallyMePemLabel,
    der: ByteArray,
) : AutoCloseable {
    private var derBytes: ByteArray = der.copyOf()
    private var closed: Boolean = false

    @Synchronized
    public fun der(): ByteArray {
        if (closed) {
            throw ReallyMeCodecException.InvalidInput()
        }
        return derBytes.copyOf()
    }

    @Synchronized
    override fun close() {
        derBytes.fill(0)
        closed = true
    }

    override fun toString(): String = "ReallyMePemDocument(<redacted>)"
}

internal fun sdkMulticodecLookupResult(result: CodecMulticodecLookupResult): ReallyMeMulticodecLookupResult {
    if (result.reallyMeHasUnknownFieldsForValidation() || !result.hasMetadata()) {
        throw ReallyMeCodecException.ProviderFailure()
    }
    return ReallyMeMulticodecLookupResult(
        result.name,
        Integer.toUnsignedLong(result.prefixLength),
        sdkMulticodecMetadata(result.metadata),
    )
}

internal fun sdkMulticodecTable(result: CodecMulticodecTableResult): ReallyMeMulticodecTable {
    if (result.reallyMeHasUnknownFieldsForValidation()) {
        throw ReallyMeCodecException.ProviderFailure()
    }
    return ReallyMeMulticodecTable(result.entriesList.map { sdkMulticodecMetadata(it) })
}

internal fun sdkParsedMultikey(result: CodecMultikeyParseResult): ReallyMeParsedMultikey {
    if (result.reallyMeHasUnknownFieldsForValidation()) {
        throw ReallyMeCodecException.ProviderFailure()
    }
    // A parsed key has a nonzero fixed length or a variable rule. Check the
    // legacy pair before copying bytes so no contradictory state is hidden.
    if (result.variablePublicKeyLength != (result.expectedPublicKeyLength == 0) ||
        (
            !result.variablePublicKeyLength &&
                Integer.toUnsignedLong(result.expectedPublicKeyLength) != result.publicKey.size().toLong()
        )
    ) {
        throw ReallyMeCodecException.ProviderFailure()
    }
    val publicKey = result.publicKey.toByteArray()
    return try {
        ReallyMeParsedMultikey(
            result.codecName,
            result.algorithmName,
            publicKey,
            if (result.variablePublicKeyLength || result.expectedPublicKeyLength == 0) {
                null
            } else {
                Integer.toUnsignedLong(result.expectedPublicKeyLength)
            },
        )
    } finally {
        publicKey.fill(0)
    }
}

internal fun sdkDagCborCidVerification(result: CodecDagCborVerifyCidResult): ReallyMeDagCborCidVerification {
    if (result.reallyMeHasUnknownFieldsForValidation()) {
        throw ReallyMeCodecException.ProviderFailure()
    }
    return ReallyMeDagCborCidVerification(result.valid, result.expectedCid, result.actualCid)
}

internal fun sdkPemDocument(result: CodecPemDecodeResult): ReallyMePemDocument {
    if (result.reallyMeHasUnknownFieldsForValidation()) {
        throw ReallyMeCodecException.ProviderFailure()
    }
    val der = result.der.toByteArray()
    return try {
        ReallyMePemDocument(sdkPemLabel(result.label), der)
    } finally {
        der.fill(0)
    }
}

internal fun sdkMulticodecMetadata(result: CodecMulticodecSpec): ReallyMeMulticodecMetadata {
    if (result.reallyMeHasUnknownFieldsForValidation()) {
        throw ReallyMeCodecException.ProviderFailure()
    }
    // The legacy message also represents not-applicable as zero/false, but a
    // variable rule must never carry a nonzero fixed length.
    if (result.variableLength && result.fixedLength != 0) {
        throw ReallyMeCodecException.ProviderFailure()
    }
    return ReallyMeMulticodecMetadata(
        result.name,
        result.algorithmName,
        sdkMulticodecTag(result.tag),
        sdkKeyMaterialKind(result.keyMaterialKind),
        result.prefix.toByteArray(),
        if (result.variableLength || result.fixedLength == 0) {
            null
        } else {
            Integer.toUnsignedLong(result.fixedLength)
        },
    )
}
