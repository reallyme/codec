// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec

import com.google.protobuf.ByteString
import com.google.protobuf.InvalidProtocolBufferException
import com.google.protobuf.UnsafeByteOperations
import me.really.codec.v1.CodecBoundaryError
import me.really.codec.v1.CodecDagCborVerifyCidRequest
import me.really.codec.v1.CodecError
import me.really.codec.v1.CodecErrorReason
import me.really.codec.v1.CodecMulticodecLookupPrefixRequest
import me.really.codec.v1.CodecMulticodecPrefixForNameRequest
import me.really.codec.v1.CodecMulticodecTableRequest
import me.really.codec.v1.CodecMultikeyParseRequest
import me.really.codec.v1.CodecOperationRequest
import java.nio.ByteBuffer
import java.nio.charset.CharacterCodingException
import java.nio.charset.CodingErrorAction

private object CodecOperation {
    const val BASE64_ENCODE: Int = 1
    const val BASE64_DECODE: Int = 2
    const val BASE64URL_ENCODE: Int = 3
    const val BASE64URL_DECODE: Int = 4
    const val LOWER_HEX_ENCODE: Int = 5
    const val LOWER_HEX_DECODE: Int = 6
    const val BASE58BTC_ENCODE: Int = 7
    const val BASE58BTC_DECODE: Int = 8
    const val MULTIBASE_BASE58BTC_ENCODE: Int = 9
    const val MULTIBASE_BASE64URL_ENCODE: Int = 10
    const val MULTIBASE_DECODE: Int = 11
    const val MULTICODEC_PREFIX_FOR_NAME: Int = 12
    const val MULTICODEC_LOOKUP_PREFIX: Int = 13
    const val MULTICODEC_STRIP_PREFIX: Int = 14
    const val MULTICODEC_TABLE: Int = 15
    const val MULTIKEY_ENCODE: Int = 16
    const val MULTIKEY_PARSE: Int = 17
    const val REQUIRE_SUPPORTED_MULTICODEC: Int = 18
    const val DAG_CBOR_ENCODE: Int = 19
    const val DAG_CBOR_DECODE: Int = 20
    const val DAG_CBOR_COMPUTE_CID: Int = 21
    const val DAG_CBOR_VERIFY_CID: Int = 22
    const val DAG_CBOR_SHA256_CONTENT_HASH: Int = 23
    const val DAG_CBOR_MULTIHASH: Int = 24
    const val TRY_PARSE_CID: Int = 25
    const val DAG_CBOR_CODEC_CODE: Int = 26
    const val CANONICALIZE_JSON: Int = 27
    const val PEM_DECODE: Int = 28
    const val PEM_ENCODE: Int = 29
    const val VALIDATE_KEY_BINDING: Int = 30
}

private object CodecBoolOperation {
    const val BINDING_TYPE_MATCHES_CODEC: Int = 1
    const val IS_VALID_CID_STRING: Int = 2
}

private const val MAX_CODEC_FFI_INPUT_BYTES: Int = 1_048_576
private const val MAX_CODEC_PROTO_MESSAGE_BYTES: Int = 1_048_576

/**
 * Kotlin facade for ReallyMe codec operations backed by the Rust codec crates.
 */
public object ReallyMeCodec {
    private val emptyBytes: ByteArray = ByteArray(0)

    @JvmStatic
    public fun base64Encode(bytes: ByteArray): String =
        text(process(CodecOperation.BASE64_ENCODE, bytes))

    @JvmStatic
    public fun base64Decode(text: String): ByteArray =
        withTextBytes(text) { encoded ->
            process(CodecOperation.BASE64_DECODE, encoded)
        }

    @JvmStatic
    public fun base64urlEncode(bytes: ByteArray): String =
        text(process(CodecOperation.BASE64URL_ENCODE, bytes))

    @JvmStatic
    public fun base64urlDecode(text: String): ByteArray =
        withTextBytes(text) { encoded ->
            process(CodecOperation.BASE64URL_DECODE, encoded)
        }

    @JvmStatic
    public fun bytesToLowerHex(bytes: ByteArray): String =
        text(process(CodecOperation.LOWER_HEX_ENCODE, bytes))

    @JvmStatic
    public fun lowerHexToBytes(text: String): ByteArray =
        withTextBytes(text) { encoded ->
            process(CodecOperation.LOWER_HEX_DECODE, encoded)
        }

    @JvmStatic
    public fun base58btcEncode(bytes: ByteArray): String =
        text(process(CodecOperation.BASE58BTC_ENCODE, bytes))

    @JvmStatic
    public fun base58btcDecode(text: String): ByteArray =
        withTextBytes(text) { encoded ->
            process(CodecOperation.BASE58BTC_DECODE, encoded)
        }

    @JvmStatic
    public fun multibaseBase58btcEncode(bytes: ByteArray): String =
        text(process(CodecOperation.MULTIBASE_BASE58BTC_ENCODE, bytes))

    @JvmStatic
    public fun multibaseBase64urlEncode(bytes: ByteArray): String =
        text(process(CodecOperation.MULTIBASE_BASE64URL_ENCODE, bytes))

    @JvmStatic
    public fun multibaseDecode(text: String): ByteArray =
        withTextBytes(text) { encoded ->
            process(CodecOperation.MULTIBASE_DECODE, encoded)
        }

    @JvmStatic
    public fun multicodecPrefixForName(name: String): String =
        withTextBytes(name) { encoded ->
            text(process(CodecOperation.MULTICODEC_PREFIX_FOR_NAME, encoded))
        }

    @JvmStatic
    public fun multicodecPrefixForNameProto(name: String): ByteArray =
        processProtoPayload(multicodecPrefixForNameRequest(name))

    @JvmStatic
    public fun multicodecPrefixForNameProtoResult(name: String): ReallyMeCodecProtoResult {
        val request = try {
            multicodecPrefixForNameRequest(name)
        } catch (_: ReallyMeCodecException.InvalidInput) {
            return boundaryResourceLimitResult()
        }
        return processProtoResult(request)
    }

    @JvmStatic
    public fun multicodecLookupPrefix(bytes: ByteArray): String =
        text(process(CodecOperation.MULTICODEC_LOOKUP_PREFIX, bytes))

    @JvmStatic
    public fun multicodecLookupPrefixProto(bytes: ByteArray): ByteArray =
        processProtoPayload(multicodecLookupPrefixRequest(bytes))

    @JvmStatic
    public fun multicodecLookupPrefixProtoResult(bytes: ByteArray): ReallyMeCodecProtoResult {
        val request = try {
            multicodecLookupPrefixRequest(bytes)
        } catch (_: ReallyMeCodecException.InvalidInput) {
            return boundaryResourceLimitResult()
        }
        return processProtoResult(request)
    }

    @JvmStatic
    public fun multicodecStripPrefix(bytes: ByteArray): ByteArray =
        process(CodecOperation.MULTICODEC_STRIP_PREFIX, bytes)

    @JvmStatic
    public fun multicodecTable(): String =
        text(process(CodecOperation.MULTICODEC_TABLE, emptyBytes))

    @JvmStatic
    public fun multicodecTableProto(): ByteArray =
        processProtoPayload(
            CodecOperationRequest.newBuilder()
                .setMulticodecTable(CodecMulticodecTableRequest.getDefaultInstance())
                .build()
        )

    @JvmStatic
    public fun multicodecTableProtoResult(): ReallyMeCodecProtoResult =
        processProtoResult(
            CodecOperationRequest.newBuilder()
                .setMulticodecTable(CodecMulticodecTableRequest.getDefaultInstance())
                .build()
        )

    @JvmStatic
    public fun multikeyEncode(codecName: String, publicKey: ByteArray): String =
        withTextBytes(codecName) { encodedCodecName ->
            text(process(CodecOperation.MULTIKEY_ENCODE, encodedCodecName, publicKey))
        }

    @JvmStatic
    public fun multikeyParse(multikey: String): String =
        withTextBytes(multikey) { encoded ->
            text(process(CodecOperation.MULTIKEY_PARSE, encoded))
        }

    @JvmStatic
    public fun multikeyParseProto(multikey: String): ByteArray =
        processProtoPayload(multikeyParseRequest(multikey))

    @JvmStatic
    public fun multikeyParseProtoResult(multikey: String): ReallyMeCodecProtoResult {
        val request = try {
            multikeyParseRequest(multikey)
        } catch (_: ReallyMeCodecException.InvalidInput) {
            return boundaryResourceLimitResult()
        }
        return processProtoResult(request)
    }

    @JvmStatic
    public fun requireSupportedMulticodec(name: String) {
        withTextBytes(name) { encoded ->
            process(CodecOperation.REQUIRE_SUPPORTED_MULTICODEC, encoded)
        }
    }

    @JvmStatic
    public fun bindingTypeMatchesCodec(bindingType: String, codecName: String): Boolean =
        withTextBytes(bindingType, codecName) { encodedBindingType, encodedCodecName ->
            processBool(
                CodecBoolOperation.BINDING_TYPE_MATCHES_CODEC,
                encodedBindingType,
                encodedCodecName,
            )
        }

    @JvmStatic
    public fun validateKeyBinding(bindingType: String, algorithm: String?, multikey: String) {
        withTextBytes(bindingType, algorithm ?: "", multikey) {
                encodedBindingType, encodedAlgorithm, encodedMultikey ->
            process(
                CodecOperation.VALIDATE_KEY_BINDING,
                encodedBindingType,
                encodedAlgorithm,
                encodedMultikey,
            )
        }
    }

    @JvmStatic
    public fun dagCborEncode(taggedJson: String): ByteArray =
        withTextBytes(taggedJson) { encoded ->
            process(CodecOperation.DAG_CBOR_ENCODE, encoded)
        }

    @JvmStatic
    public fun dagCborDecode(bytes: ByteArray): String =
        text(process(CodecOperation.DAG_CBOR_DECODE, bytes))

    @JvmStatic
    public fun dagCborComputeCid(bytes: ByteArray): String =
        text(process(CodecOperation.DAG_CBOR_COMPUTE_CID, bytes))

    @JvmStatic
    public fun dagCborVerifyCid(cid: String, bytes: ByteArray): String =
        withTextBytes(cid) { encodedCid ->
            text(process(CodecOperation.DAG_CBOR_VERIFY_CID, encodedCid, bytes))
        }

    @JvmStatic
    public fun dagCborVerifyCidProto(cid: String, bytes: ByteArray): ByteArray =
        processProtoPayload(dagCborVerifyCidRequest(cid, bytes))

    @JvmStatic
    public fun dagCborVerifyCidProtoResult(cid: String, bytes: ByteArray): ReallyMeCodecProtoResult {
        val request = try {
            dagCborVerifyCidRequest(cid, bytes)
        } catch (_: ReallyMeCodecException.InvalidInput) {
            return boundaryResourceLimitResult()
        }
        return processProtoResult(request)
    }

    @JvmStatic
    public fun dagCborSha256ContentHash(bytes: ByteArray): ByteArray =
        process(CodecOperation.DAG_CBOR_SHA256_CONTENT_HASH, bytes)

    @JvmStatic
    public fun dagCborMultihash(bytes: ByteArray): ByteArray =
        process(CodecOperation.DAG_CBOR_MULTIHASH, bytes)

    @JvmStatic
    public fun isValidCidString(cid: String): Boolean =
        withTextBytes(cid) { encoded ->
            processBool(CodecBoolOperation.IS_VALID_CID_STRING, encoded, emptyBytes)
        }

    @JvmStatic
    public fun tryParseCid(cid: String): String? =
        try {
            withTextBytes(cid) { encoded ->
                text(process(CodecOperation.TRY_PARSE_CID, encoded))
            }
        } catch (_: ReallyMeCodecException.InvalidInput) {
            null
        }

    @JvmStatic
    public fun dagCborCodecCode(): Int {
        val code = text(process(CodecOperation.DAG_CBOR_CODEC_CODE, emptyBytes)).toIntOrNull()
        return code ?: throw ReallyMeCodecException.ProviderFailure()
    }

    @JvmStatic
    public fun canonicalizeJson(json: String): String =
        withTextBytes(json) { encoded ->
            text(process(CodecOperation.CANONICALIZE_JSON, encoded))
        }

    /**
     * Executes one binary generated [CodecOperationRequest].
     *
     * The returned bytes are always a binary `CodecProtoResultEnvelope`.
     * Malformed input and operation failures are represented inside that
     * envelope rather than collapsed into a JNI exception.
     */
    @JvmStatic
    public fun processProto(request: ByteArray): ByteArray {
        ReallyMeCodecRustNativeProvider.requireLoaded()
        return try {
            ReallyMeCodecNative.processProtoNative(request)
        } catch (error: UnsatisfiedLinkError) {
            throw ReallyMeCodecException.ProviderFailure()
        }
    }

    /**
     * Executes the generated ProtoJSON view of [CodecOperationRequest].
     *
     * JSON is request-only; the returned bytes are the same binary result
     * envelope used by [processProto].
     */
    @JvmStatic
    public fun processProtoJson(requestJson: ByteArray): ByteArray {
        ReallyMeCodecRustNativeProvider.requireLoaded()
        return try {
            ReallyMeCodecNative.processProtoJsonNative(requestJson)
        } catch (error: UnsatisfiedLinkError) {
            throw ReallyMeCodecException.ProviderFailure()
        }
    }

    @JvmStatic
    @JvmOverloads
    public fun decodePem(pem: ByteArray, optionsJson: String = ""): ByteArray =
        withTextBytes(optionsJson) { encodedOptions ->
            process(CodecOperation.PEM_DECODE, pem, encodedOptions)
        }

    @JvmStatic
    @JvmOverloads
    public fun encodePem(label: String, der: ByteArray, optionsJson: String = ""): ByteArray =
        withTextBytes(label, optionsJson) { encodedLabel, encodedOptions ->
            process(CodecOperation.PEM_ENCODE, encodedLabel, der, encodedOptions)
        }

    private fun process(
        operation: Int,
        first: ByteArray,
        second: ByteArray = emptyBytes,
        third: ByteArray = emptyBytes,
    ): ByteArray {
        ReallyMeCodecRustNativeProvider.requireLoaded()
        return try {
            ReallyMeCodecNative.processNative(operation, first, second, third)
        } catch (error: UnsatisfiedLinkError) {
            throw ReallyMeCodecException.ProviderFailure()
        }
    }

    private fun processProtoPayload(request: CodecOperationRequest): ByteArray {
        val result = processProtoResult(request)
        if (result.status == ReallyMeCodecProtoStatus.CODEC_ERROR) {
            val error = exceptionForCodecErrorPayload(result.bytes)
            result.bytes.fill(0)
            throw error
        }
        return result.bytes
    }

    internal fun exceptionForCodecErrorPayload(bytes: ByteArray): ReallyMeCodecException {
        val codecError = try {
            CodecError.parseFrom(bytes)
        } catch (_: InvalidProtocolBufferException) {
            return ReallyMeCodecException.ProviderFailure()
        } catch (_: RuntimeException) {
            return ReallyMeCodecException.ProviderFailure()
        }
        return when (codecError.errorCase) {
            CodecError.ErrorCase.BASE_ENCODING ->
                inputErrorOrProviderFailure(codecError.baseEncoding.reason, 100..199)
            CodecError.ErrorCase.PEM ->
                inputErrorOrProviderFailure(codecError.pem.reason, 200..299)
            CodecError.ErrorCase.MULTIFORMAT ->
                inputErrorOrProviderFailure(codecError.multiformat.reason, 300..399)
            CodecError.ErrorCase.CANONICALIZATION -> {
                if (
                    codecError.canonicalization.reason ==
                    CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_INTERNAL
                ) {
                    ReallyMeCodecException.ProviderFailure()
                } else {
                    inputErrorOrProviderFailure(codecError.canonicalization.reason, 400..499)
                }
            }
            CodecError.ErrorCase.BACKEND -> ReallyMeCodecException.ProviderFailure()
            CodecError.ErrorCase.BOUNDARY -> {
                if (
                    codecError.boundary.reason ==
                    CodecErrorReason.CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED
                ) {
                    ReallyMeCodecException.InvalidInput()
                } else {
                    ReallyMeCodecException.ProviderFailure()
                }
            }
            CodecError.ErrorCase.ERROR_NOT_SET,
            null,
            -> ReallyMeCodecException.ProviderFailure()
        }
    }

    private fun inputErrorOrProviderFailure(
        reason: CodecErrorReason,
        expectedRange: IntRange,
    ): ReallyMeCodecException =
        if (
            reason != CodecErrorReason.UNRECOGNIZED &&
            reason.number in expectedRange
        ) {
            ReallyMeCodecException.InvalidInput()
        } else {
            ReallyMeCodecException.ProviderFailure()
        }

    private fun processProtoResult(request: CodecOperationRequest): ReallyMeCodecProtoResult {
        ReallyMeCodecRustNativeProvider.requireLoaded()
        val serializedSize = request.serializedSize
        if (serializedSize < 0) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        if (serializedSize > MAX_CODEC_PROTO_MESSAGE_BYTES) {
            return boundaryResourceLimitResult()
        }
        val requestBytes = try {
            request.toByteArray()
        } catch (_: RuntimeException) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        return try {
            ReallyMeCodecNative.processProtoResultNative(requestBytes)
        } catch (error: UnsatisfiedLinkError) {
            throw ReallyMeCodecException.ProviderFailure()
        } finally {
            requestBytes.fill(0)
        }
    }

    private fun processBool(operation: Int, first: ByteArray, second: ByteArray): Boolean {
        ReallyMeCodecRustNativeProvider.requireLoaded()
        return try {
            when (ReallyMeCodecNative.processBoolNative(operation, first, second)) {
                0 -> false
                1 -> true
                else -> throw ReallyMeCodecException.ProviderFailure()
            }
        } catch (error: UnsatisfiedLinkError) {
            throw ReallyMeCodecException.ProviderFailure()
        }
    }

    private fun boundaryResourceLimitResult(): ReallyMeCodecProtoResult {
        val boundary = CodecBoundaryError.newBuilder()
            .setReason(CodecErrorReason.CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED)
            .build()
        val errorBytes = try {
            CodecError.newBuilder()
                .setBoundary(boundary)
                .build()
                .toByteArray()
        } catch (_: RuntimeException) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        return ReallyMeCodecProtoResult(ReallyMeCodecProtoStatus.CODEC_ERROR, errorBytes)
    }

    private fun multicodecPrefixForNameRequest(name: String): CodecOperationRequest {
        requireBoundaryAggregate(utf8Length(name))
        return CodecOperationRequest.newBuilder()
            .setMulticodecPrefixForName(
                CodecMulticodecPrefixForNameRequest.newBuilder().setName(name)
            )
            .build()
    }

    private fun multicodecLookupPrefixRequest(bytes: ByteArray): CodecOperationRequest {
        requireBoundaryAggregate(bytes.size)
        return CodecOperationRequest.newBuilder()
            .setMulticodecLookupPrefix(
                CodecMulticodecLookupPrefixRequest.newBuilder()
                    .setValue(borrowedByteString(bytes))
            )
            .build()
    }

    private fun multikeyParseRequest(multikey: String): CodecOperationRequest {
        requireBoundaryAggregate(utf8Length(multikey))
        return CodecOperationRequest.newBuilder()
            .setMultikeyParse(CodecMultikeyParseRequest.newBuilder().setMultikey(multikey))
            .build()
    }

    private fun dagCborVerifyCidRequest(cid: String, bytes: ByteArray): CodecOperationRequest {
        requireBoundaryAggregate(utf8Length(cid), bytes.size)
        return CodecOperationRequest.newBuilder()
            .setDagCborVerifyCid(
                CodecDagCborVerifyCidRequest.newBuilder()
                    .setCid(cid)
                    .setPayload(borrowedByteString(bytes))
            )
            .build()
    }

    /**
     * Borrows the caller's array only for synchronous protobuf serialization.
     * `ByteString.copyFrom` would create an additional immutable payload owner
     * that the JVM cannot wipe. This wrapper creates no byte copy; the final
     * serialized request is the sole SDK-owned copy and is wiped in
     * [processProtoResult]. The wrapper must never escape or be retained.
     */
    private fun borrowedByteString(bytes: ByteArray): ByteString =
        UnsafeByteOperations.unsafeWrap(bytes)

    private fun bytes(text: String): ByteArray {
        utf8Length(text)
        val encoded = text.toByteArray(Charsets.UTF_8)
        if (encoded.size > MAX_CODEC_FFI_INPUT_BYTES) {
            encoded.fill(0)
            throw ReallyMeCodecException.InvalidInput()
        }
        return encoded
    }

    /**
     * Limits the lifetime of a mutable UTF-8 copy created from an immutable
     * JVM string. The caller retains the original string, but every additional
     * buffer created by this facade is wiped on success and failure paths.
     */
    private fun <T> withTextBytes(text: String, action: (ByteArray) -> T): T {
        val encoded = bytes(text)
        return try {
            action(encoded)
        } finally {
            encoded.fill(0)
        }
    }

    private fun <T> withTextBytes(
        first: String,
        second: String,
        action: (ByteArray, ByteArray) -> T,
    ): T = withTextBytes(first) { firstBytes ->
        withTextBytes(second) { secondBytes ->
            action(firstBytes, secondBytes)
        }
    }

    private fun <T> withTextBytes(
        first: String,
        second: String,
        third: String,
        action: (ByteArray, ByteArray, ByteArray) -> T,
    ): T = withTextBytes(first, second) { firstBytes, secondBytes ->
        withTextBytes(third) { thirdBytes ->
            action(firstBytes, secondBytes, thirdBytes)
        }
    }

    private fun utf8Length(text: String): Int {
        if (text.length > MAX_CODEC_FFI_INPUT_BYTES) {
            throw ReallyMeCodecException.InvalidInput()
        }
        var length = 0
        var index = 0
        while (index < text.length) {
            val character = text[index]
            val increment = when {
                character.code <= 0x7f -> 1
                character.code <= 0x7ff -> 2
                Character.isHighSurrogate(character) &&
                    index + 1 < text.length &&
                    Character.isLowSurrogate(text[index + 1]) -> {
                    index += 1
                    4
                }
                else -> 3
            }
            if (length > MAX_CODEC_FFI_INPUT_BYTES - increment) {
                throw ReallyMeCodecException.InvalidInput()
            }
            length += increment
            index += 1
        }
        return length
    }

    private fun requireBoundaryAggregate(vararg lengths: Int) {
        var aggregate = 0L
        for (length in lengths) {
            aggregate += length.toLong()
            if (aggregate > MAX_CODEC_FFI_INPUT_BYTES.toLong()) {
                throw ReallyMeCodecException.InvalidInput()
            }
        }
    }

    private fun text(bytes: ByteArray): String {
        val decoder = Charsets.UTF_8.newDecoder()
            .onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT)
        return try {
            val characters = decoder.decode(ByteBuffer.wrap(bytes))
            try {
                characters.toString()
            } finally {
                if (characters.hasArray()) {
                    characters.array().fill('\u0000')
                } else if (!characters.isReadOnly) {
                    for (index in 0 until characters.limit()) {
                        characters.put(index, '\u0000')
                    }
                }
            }
        } catch (_: CharacterCodingException) {
            throw ReallyMeCodecException.ProviderFailure()
        } finally {
            bytes.fill(0)
        }
    }

}
