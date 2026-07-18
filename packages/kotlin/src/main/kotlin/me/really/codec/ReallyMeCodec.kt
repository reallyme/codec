// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec

import com.google.protobuf.ByteString
import com.google.protobuf.CodedInputStream
import com.google.protobuf.InvalidProtocolBufferException
import com.google.protobuf.UnsafeByteOperations
import me.really.codec.v1.CodecDagCborDecodeRequest
import me.really.codec.v1.CodecDagCborEncodeRequest
import me.really.codec.v1.CodecDagCborVerifyCidRequest
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
import me.really.codec.v1.CodecError
import me.really.codec.v1.CodecErrorOrigin
import me.really.codec.v1.CodecErrorReason
import me.really.codec.v1.CodecMulticodecLookupPrefixRequest
import me.really.codec.v1.CodecMulticodecPrefixForNameRequest
import me.really.codec.v1.CodecMulticodecTableRequest
import me.really.codec.v1.CodecMultikeyParseRequest
import me.really.codec.v1.CodecOperationRequest
import me.really.codec.v1.CodecOperationResponse
import me.really.codec.v1.CodecOperationResult
import me.really.codec.v1.CodecPemDecodeOptions
import me.really.codec.v1.CodecPemDecodeRequest
import me.really.codec.v1.CodecPemEncodeOptions
import me.really.codec.v1.CodecPemEncodeRequest
import me.really.codec.v1.CodecPemLineEnding
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
    const val MULTICODEC_STRIP_PREFIX: Int = 14
    const val MULTIKEY_ENCODE: Int = 16
    const val REQUIRE_SUPPORTED_MULTICODEC: Int = 18
    const val DAG_CBOR_COMPUTE_CID: Int = 21
    const val DAG_CBOR_SHA256_CONTENT_HASH: Int = 23
    const val DAG_CBOR_MULTIHASH: Int = 24
    const val TRY_PARSE_CID: Int = 25
    const val DAG_CBOR_CODEC_CODE: Int = 26
    const val CANONICALIZE_JSON: Int = 27
    const val VALIDATE_KEY_BINDING: Int = 30
}

private object CodecBoolOperation {
    const val BINDING_TYPE_MATCHES_CODEC: Int = 1
    const val IS_VALID_CID_STRING: Int = 2
}

private const val MAX_DETERMINISTIC_CBOR_NESTING_DEPTH: Int = 64
private const val MAX_DETERMINISTIC_CBOR_NODES: Int = 65_536
private const val MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES: Int = 16_384
private const val MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES: Int = 1_048_576
private const val MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES: Int = 1_048_576
private const val MAX_CODEC_PROTO_STRUCTURAL_BYTES_PER_NODE: Int = 128
private const val MAX_CODEC_PROTO_FIXED_OPERATION_BYTES: Int = 4_096
private const val MAX_CODEC_PROTO_MESSAGE_BYTES: Int =
    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES +
        MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES +
        (MAX_DETERMINISTIC_CBOR_NODES * MAX_CODEC_PROTO_STRUCTURAL_BYTES_PER_NODE) +
        MAX_CODEC_PROTO_FIXED_OPERATION_BYTES
// One semantic map level expands to Value -> Map -> MapEntry. Five outer/key
// wrappers cover the deepest generated request/result path. Java Protobuf
// Lite's default of 100 cannot carry the documented semantic depth of 64.
private const val MAX_DETERMINISTIC_CBOR_PROTO_MESSAGE_DEPTH: Int =
    (MAX_DETERMINISTIC_CBOR_NESTING_DEPTH * 3) + 5

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
    public fun multicodecPrefixForName(name: String): ReallyMeMulticodecMetadata {
        val result = processOperation(multicodecPrefixForNameRequest(name))
        if (result.resultCase != CodecOperationResult.ResultCase.MULTICODEC_PREFIX_FOR_NAME) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        return sdkMulticodecMetadata(result.multicodecPrefixForName)
    }

    @JvmStatic
    public fun multicodecLookupPrefix(bytes: ByteArray): ReallyMeMulticodecLookupResult {
        val result = processOperation(multicodecLookupPrefixRequest(bytes))
        if (result.resultCase != CodecOperationResult.ResultCase.MULTICODEC_LOOKUP_PREFIX) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        return sdkMulticodecLookupResult(result.multicodecLookupPrefix)
    }

    @JvmStatic
    public fun multicodecStripPrefix(bytes: ByteArray): ByteArray =
        process(CodecOperation.MULTICODEC_STRIP_PREFIX, bytes)

    @JvmStatic
    public fun multicodecTable(): ReallyMeMulticodecTable {
        val result = processOperation(multicodecTableRequest())
        if (result.resultCase != CodecOperationResult.ResultCase.MULTICODEC_TABLE) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        return sdkMulticodecTable(result.multicodecTable)
    }

    @JvmStatic
    public fun multikeyEncode(codecName: String, publicKey: ByteArray): String =
        withTextBytes(codecName) { encodedCodecName ->
            text(process(CodecOperation.MULTIKEY_ENCODE, encodedCodecName, publicKey))
        }

    @JvmStatic
    public fun multikeyParse(multikey: String): ReallyMeParsedMultikey {
        val result = processOperation(multikeyParseRequest(multikey))
        if (result.resultCase != CodecOperationResult.ResultCase.MULTIKEY_PARSE) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        return sdkParsedMultikey(result.multikeyParse)
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
    public fun dagCborEncode(value: ReallyMeDeterministicCborValue): ByteArray {
        validateDeterministicCborValue(value)
        val operationResult = processOperation(dagCborEncodeRequest(value))
        if (operationResult.resultCase != CodecOperationResult.ResultCase.DAG_CBOR_ENCODE) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        val result = operationResult.dagCborEncode
        if (result.reallyMeHasUnknownFieldsForValidation()) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        return result.encoded.toByteArray()
    }

    @JvmStatic
    public fun dagCborDecode(bytes: ByteArray): ReallyMeDeterministicCborValue {
        requireBoundaryAggregate(bytes.size)
        val ownedBytes = bytes.copyOf()
        return try {
            val operationResult = processOperation(dagCborDecodeRequest(ownedBytes))
            if (operationResult.resultCase != CodecOperationResult.ResultCase.DAG_CBOR_DECODE) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            val result = operationResult.dagCborDecode
            if (result.reallyMeHasUnknownFieldsForValidation() || !result.hasValue()) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            validateProviderDeterministicCborValue(result.value)
            sdkValue(result.value)
        } finally {
            ownedBytes.fill(0)
        }
    }

    @JvmStatic
    public fun dagCborComputeCid(bytes: ByteArray): String =
        text(process(CodecOperation.DAG_CBOR_COMPUTE_CID, bytes))

    @JvmStatic
    public fun dagCborVerifyCid(
        cid: String,
        bytes: ByteArray,
    ): ReallyMeDagCborCidVerification {
        val result = processOperation(dagCborVerifyCidRequest(cid, bytes))
        if (result.resultCase != CodecOperationResult.ResultCase.DAG_CBOR_VERIFY_CID) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        return sdkDagCborCidVerification(result.dagCborVerifyCid)
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
    public fun deterministicCborEncode(value: ReallyMeDeterministicCborValue): ByteArray {
        validateDeterministicCborValue(value)
        val operationResult = processOperation(deterministicCborEncodeRequest(value))
        if (operationResult.resultCase != CodecOperationResult.ResultCase.DETERMINISTIC_CBOR_ENCODE) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        val result = operationResult.deterministicCborEncode
        if (result.reallyMeHasUnknownFieldsForValidation()) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        return result.encoded.toByteArray()
    }

    @JvmStatic
    public fun deterministicCborDecode(bytes: ByteArray): ReallyMeDeterministicCborValue {
        requireBoundaryAggregate(bytes.size)
        // ByteArray is mutable and may be shared with another thread. Validate
        // and serialize one SDK-owned snapshot, then wipe that same owner.
        val ownedBytes = bytes.copyOf()
        return try {
            val operationResult = processOperation(deterministicCborDecodeRequest(ownedBytes))
            if (
                operationResult.resultCase !=
                CodecOperationResult.ResultCase.DETERMINISTIC_CBOR_DECODE
            ) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            val result = operationResult.deterministicCborDecode
            if (result.reallyMeHasUnknownFieldsForValidation() || !result.hasValue()) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            validateProviderDeterministicCborValue(result.value)
            sdkValue(result.value)
        } finally {
            ownedBytes.fill(0)
        }
    }

    @JvmStatic
    public fun canonicalizeJson(json: String): String =
        withTextBytes(json) { encoded ->
            text(process(CodecOperation.CANONICALIZE_JSON, encoded))
        }

    /**
     * Executes one binary generated [CodecOperationRequest].
     *
     * The returned bytes are always a binary `CodecOperationResponse`.
     * Malformed input and operation failures are represented by its generated
     * error oneof rather than collapsed into a JNI exception.
     */
    @JvmStatic
    public fun processOperation(request: ByteArray): ByteArray {
        ReallyMeCodecRustNativeProvider.requireLoaded()
        return try {
            ReallyMeCodecNative.processOperationNative(request)
        } catch (error: UnsatisfiedLinkError) {
            throw ReallyMeCodecException.ProviderFailure()
        }
    }

    /**
     * Executes the generated ProtoJSON view of [CodecOperationRequest].
     *
     * JSON is request-only; the returned bytes are the same discriminated
     * binary response used by [processOperation].
     */
    @JvmStatic
    public fun processOperationJson(requestJson: ByteArray): ByteArray {
        ReallyMeCodecRustNativeProvider.requireLoaded()
        return try {
            ReallyMeCodecNative.processOperationJsonNative(requestJson)
        } catch (error: UnsatisfiedLinkError) {
            throw ReallyMeCodecException.ProviderFailure()
        }
    }

    @JvmStatic
    @JvmOverloads
    public fun decodePem(
        pem: ByteArray,
        options: ReallyMePemDecodeOptions = ReallyMePemDecodeOptions(),
    ): ReallyMePemDocument {
        val result = processOperation(pemDecodeRequest(pem, options))
        if (result.resultCase != CodecOperationResult.ResultCase.PEM_DECODE) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        return sdkPemDocument(result.pemDecode)
    }

    @JvmStatic
    @JvmOverloads
    public fun encodePem(
        label: ReallyMePemLabel,
        der: ByteArray,
        options: ReallyMePemEncodeOptions = ReallyMePemEncodeOptions(),
    ): ByteArray {
        val result = processOperation(pemEncodeRequest(label, der, options))
        if (result.resultCase != CodecOperationResult.ResultCase.PEM_ENCODE) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        val pem = result.pemEncode
        if (pem.reallyMeHasUnknownFieldsForValidation()) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        return pem.pem.toByteArray()
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

    /**
     * Executes one generated request through the fully discriminated response
     * boundary used by structured SDK convenience methods.
     *
     * The JNI request and response arrays are SDK-owned transient buffers and
     * are wiped on every path. Each public method must still require its exact
     * generated result case before converting to an SDK domain value.
     */
    private fun processOperation(request: CodecOperationRequest): CodecOperationResult {
        ReallyMeCodecRustNativeProvider.requireLoaded()
        val serializedSize = request.serializedSize
        if (serializedSize < 0) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        if (serializedSize > MAX_CODEC_PROTO_MESSAGE_BYTES) {
            throw ReallyMeCodecException.InvalidInput()
        }
        val requestBytes = try {
            request.toByteArray()
        } catch (_: RuntimeException) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        val responseBytes = try {
            ReallyMeCodecNative.processOperationNative(requestBytes)
        } catch (_: UnsatisfiedLinkError) {
            throw ReallyMeCodecException.ProviderFailure()
        } finally {
            requestBytes.fill(0)
        }
        return try {
            val input = CodedInputStream.newInstance(responseBytes)
            input.setRecursionLimit(MAX_DETERMINISTIC_CBOR_PROTO_MESSAGE_DEPTH)
            val response = CodecOperationResponse.parseFrom(input)
            if (response.reallyMeHasUnknownFieldsForValidation()) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            when (response.outcomeCase) {
                CodecOperationResponse.OutcomeCase.RESULT -> {
                    val result = response.result
                    if (
                        result.reallyMeHasUnknownFieldsForValidation() ||
                        result.resultCase == CodecOperationResult.ResultCase.RESULT_NOT_SET
                    ) {
                        throw ReallyMeCodecException.ProviderFailure()
                    }
                    result
                }
                CodecOperationResponse.OutcomeCase.ERROR ->
                    throw exceptionForCodecError(response.error)
                CodecOperationResponse.OutcomeCase.OUTCOME_NOT_SET,
                null,
                -> throw ReallyMeCodecException.ProviderFailure()
            }
        } catch (_: InvalidProtocolBufferException) {
            throw ReallyMeCodecException.ProviderFailure()
        } catch (error: ReallyMeCodecException) {
            throw error
        } catch (_: RuntimeException) {
            throw ReallyMeCodecException.ProviderFailure()
        } finally {
            responseBytes.fill(0)
        }
    }

    private fun exceptionForCodecError(codecError: CodecError): ReallyMeCodecException {
        if (codecError.reallyMeHasUnknownFieldsForValidation()) {
            return ReallyMeCodecException.ProviderFailure()
        }
        val expectedOrigin = when (codecError.errorCase) {
            CodecError.ErrorCase.BASE_ENCODING -> {
                if (!knownProviderReason(codecError.baseEncoding.reallyMeHasUnknownFieldsForValidation(), codecError.baseEncoding.reason, 100..199)) {
                    return ReallyMeCodecException.ProviderFailure()
                }
                CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER
            }
            CodecError.ErrorCase.PEM -> {
                if (!knownProviderReason(codecError.pem.reallyMeHasUnknownFieldsForValidation(), codecError.pem.reason, 200..299)) {
                    return ReallyMeCodecException.ProviderFailure()
                }
                CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER
            }
            CodecError.ErrorCase.MULTIFORMAT -> {
                if (!knownProviderReason(codecError.multiformat.reallyMeHasUnknownFieldsForValidation(), codecError.multiformat.reason, 300..399)) {
                    return ReallyMeCodecException.ProviderFailure()
                }
                CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER
            }
            CodecError.ErrorCase.CANONICALIZATION -> {
                if (!knownProviderReason(
                        codecError.canonicalization.reallyMeHasUnknownFieldsForValidation(),
                        codecError.canonicalization.reason,
                        400..499,
                    )
                ) {
                    return ReallyMeCodecException.ProviderFailure()
                }
                if (
                    codecError.canonicalization.reason ==
                    CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_INTERNAL
                ) {
                    CodecErrorOrigin.CODEC_ERROR_ORIGIN_PROVIDER
                } else {
                    CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER
                }
            }
            CodecError.ErrorCase.BACKEND -> {
                if (!knownProviderReason(codecError.backend.reallyMeHasUnknownFieldsForValidation(), codecError.backend.reason, 500..599)) {
                    return ReallyMeCodecException.ProviderFailure()
                }
                CodecErrorOrigin.CODEC_ERROR_ORIGIN_PROVIDER
            }
            CodecError.ErrorCase.BOUNDARY -> {
                if (!knownProviderReason(codecError.boundary.reallyMeHasUnknownFieldsForValidation(), codecError.boundary.reason, 600..699)) {
                    return ReallyMeCodecException.ProviderFailure()
                }
                CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER
            }
            CodecError.ErrorCase.ERROR_NOT_SET,
            null,
            -> return ReallyMeCodecException.ProviderFailure()
        }
        if (codecError.origin != expectedOrigin) {
            return ReallyMeCodecException.ProviderFailure()
        }
        return if (expectedOrigin == CodecErrorOrigin.CODEC_ERROR_ORIGIN_CALLER) {
            ReallyMeCodecException.InvalidInput()
        } else {
            ReallyMeCodecException.ProviderFailure()
        }
    }

    private fun knownProviderReason(
        hasUnknownFields: Boolean,
        reason: CodecErrorReason,
        expectedRange: IntRange,
    ): Boolean =
        !hasUnknownFields &&
            reason != CodecErrorReason.UNRECOGNIZED &&
            reason.number in expectedRange

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

    private fun multicodecTableRequest(): CodecOperationRequest =
        CodecOperationRequest.newBuilder()
            .setMulticodecTable(CodecMulticodecTableRequest.getDefaultInstance())
            .build()

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

    private fun pemDecodeRequest(
        pem: ByteArray,
        options: ReallyMePemDecodeOptions,
    ): CodecOperationRequest {
        requireBoundaryAggregate(pem.size)
        val optionBuilder = CodecPemDecodeOptions.newBuilder()
            .setMaxInputLen(options.maxInputLen)
            .setMaxDerLen(options.maxDerLen)
        for (label in options.allowedLabels) {
            optionBuilder.addAllowedLabels(protoPemLabel(label))
        }
        return CodecOperationRequest.newBuilder()
            .setPemDecode(
                CodecPemDecodeRequest.newBuilder()
                    .setPem(borrowedByteString(pem))
                    .setOptions(optionBuilder)
            )
            .build()
    }

    private fun pemEncodeRequest(
        label: ReallyMePemLabel,
        der: ByteArray,
        options: ReallyMePemEncodeOptions,
    ): CodecOperationRequest {
        requireBoundaryAggregate(der.size)
        val optionBuilder = CodecPemEncodeOptions.newBuilder()
            .setMaxDerLen(options.maxDerLen)
            .setLineWidth(options.lineWidth)
            .setLineEnding(
                when (options.lineEnding) {
                    ReallyMePemLineEnding.LF ->
                        CodecPemLineEnding.CODEC_PEM_LINE_ENDING_LF
                    ReallyMePemLineEnding.CRLF ->
                        CodecPemLineEnding.CODEC_PEM_LINE_ENDING_CRLF
                    null -> CodecPemLineEnding.CODEC_PEM_LINE_ENDING_UNSPECIFIED
                }
            )
        return CodecOperationRequest.newBuilder()
            .setPemEncode(
                CodecPemEncodeRequest.newBuilder()
                    .setLabel(protoPemLabel(label))
                    .setDer(borrowedByteString(der))
                    .setOptions(optionBuilder)
            )
            .build()
    }

    private class DeterministicCborValidationState {
        var nodes: Int = 0
        var textBytes: Int = 0
        var byteStringBytes: Int = 0
    }

    private fun validateDeterministicCborValue(value: ReallyMeDeterministicCborValue) {
        val state = DeterministicCborValidationState()
        validateDeterministicCborValue(value, 0, state)
    }

    private fun validateDeterministicCborValue(
        value: ReallyMeDeterministicCborValue,
        depth: Int,
        state: DeterministicCborValidationState,
    ) {
        state.nodes = addDeterministicCborCount(
            state.nodes,
            1,
            MAX_DETERMINISTIC_CBOR_NODES,
        )
        when (value) {
            ReallyMeDeterministicCborValue.Null,
            is ReallyMeDeterministicCborValue.Bool,
            is ReallyMeDeterministicCborValue.Integer,
            -> Unit
            is ReallyMeDeterministicCborValue.Text -> {
                state.textBytes = addDeterministicCborCount(
                    state.textBytes,
                    deterministicCborUtf8Length(value.value),
                    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
                )
            }
            is ReallyMeDeterministicCborValue.Bytes -> {
                state.byteStringBytes = addDeterministicCborCount(
                    state.byteStringBytes,
                    value.borrowedBytes().size,
                    MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
                )
            }
            is ReallyMeDeterministicCborValue.Array -> {
                validateDeterministicCborContainer(value.values.size)
                val childDepth = deterministicCborChildDepth(depth)
                for (child in value.values) {
                    validateDeterministicCborValue(child, childDepth, state)
                }
            }
            is ReallyMeDeterministicCborValue.Map -> {
                validateDeterministicCborContainer(value.entries.size)
                val childDepth = deterministicCborChildDepth(depth)
                for (entry in value.entries) {
                    state.nodes = addDeterministicCborCount(
                        state.nodes,
                        1,
                        MAX_DETERMINISTIC_CBOR_NODES,
                    )
                    val key = entry.key
                    if (key is ReallyMeDeterministicCborMapKey.Text) {
                        state.textBytes = addDeterministicCborCount(
                            state.textBytes,
                            deterministicCborUtf8Length(key.value),
                            MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
                        )
                    }
                    validateDeterministicCborValue(entry.value, childDepth, state)
                }
            }
        }
    }

    private fun validateDeterministicCborContainer(count: Int) {
        if (count > MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES) {
            throw ReallyMeCodecException.InvalidInput()
        }
    }

    private fun deterministicCborChildDepth(depth: Int): Int {
        if (depth >= MAX_DETERMINISTIC_CBOR_NESTING_DEPTH) {
            throw ReallyMeCodecException.InvalidInput()
        }
        return depth + 1
    }

    private fun addDeterministicCborCount(current: Int, increment: Int, maximum: Int): Int {
        if (increment < 0 || current > maximum - increment) {
            throw ReallyMeCodecException.InvalidInput()
        }
        return current + increment
    }

    private fun deterministicCborUtf8Length(text: String): Int {
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
                Character.isSurrogate(character) ->
                    throw ReallyMeCodecException.InvalidInput()
                else -> 3
            }
            length = addDeterministicCborCount(
                length,
                increment,
                MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
            )
            index += 1
        }
        return length
    }

    /**
     * Validates the generated provider tree before creating a second managed
     * owner graph. This is boundary-shape and resource validation only; Rust
     * remains authoritative for deterministic-CBOR semantics and bytes.
     */
    internal fun validateProviderDeterministicCborValue(value: CodecDeterministicCborValue) {
        val state = DeterministicCborValidationState()
        validateProviderDeterministicCborValue(value, 0, state)
    }

    private fun validateProviderDeterministicCborValue(
        value: CodecDeterministicCborValue,
        depth: Int,
        state: DeterministicCborValidationState,
    ) {
        requireNoProviderUnknownFields(value.reallyMeHasUnknownFieldsForValidation())
        state.nodes = addProviderDeterministicCborCount(
            state.nodes,
            1,
            MAX_DETERMINISTIC_CBOR_NODES,
        )
        when (value.valueCase) {
            CodecDeterministicCborValue.ValueCase.NULL_VALUE,
            -> requireNoProviderUnknownFields(
                value.nullValue.reallyMeHasUnknownFieldsForValidation()
            )
            CodecDeterministicCborValue.ValueCase.BOOL_VALUE -> requireNoProviderUnknownFields(
                value.boolValue.reallyMeHasUnknownFieldsForValidation()
            )
            CodecDeterministicCborValue.ValueCase.INTEGER_VALUE ->
                validateProviderDeterministicCborInteger(value.integerValue)
            CodecDeterministicCborValue.ValueCase.TEXT_VALUE -> {
                requireNoProviderUnknownFields(
                    value.textValue.reallyMeHasUnknownFieldsForValidation()
                )
                state.textBytes = addProviderDeterministicCborCount(
                    state.textBytes,
                    providerDeterministicCborUtf8Length(value.textValue.value),
                    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
                )
            }
            CodecDeterministicCborValue.ValueCase.BYTES_VALUE -> {
                requireNoProviderUnknownFields(
                    value.bytesValue.reallyMeHasUnknownFieldsForValidation()
                )
                state.byteStringBytes = addProviderDeterministicCborCount(
                    state.byteStringBytes,
                    value.bytesValue.value.size(),
                    MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
                )
            }
            CodecDeterministicCborValue.ValueCase.ARRAY_VALUE -> {
                requireNoProviderUnknownFields(
                    value.arrayValue.reallyMeHasUnknownFieldsForValidation()
                )
                validateProviderDeterministicCborContainer(value.arrayValue.valuesCount)
                val childDepth = providerDeterministicCborChildDepth(depth)
                for (child in value.arrayValue.valuesList) {
                    validateProviderDeterministicCborValue(child, childDepth, state)
                }
            }
            CodecDeterministicCborValue.ValueCase.MAP_VALUE -> {
                requireNoProviderUnknownFields(
                    value.mapValue.reallyMeHasUnknownFieldsForValidation()
                )
                validateProviderDeterministicCborContainer(value.mapValue.entriesCount)
                val childDepth = providerDeterministicCborChildDepth(depth)
                for (entry in value.mapValue.entriesList) {
                    requireNoProviderUnknownFields(
                        entry.reallyMeHasUnknownFieldsForValidation()
                    )
                    if (!entry.hasKey() || !entry.hasValue()) {
                        throw ReallyMeCodecException.ProviderFailure()
                    }
                    state.nodes = addProviderDeterministicCborCount(
                        state.nodes,
                        1,
                        MAX_DETERMINISTIC_CBOR_NODES,
                    )
                    validateProviderDeterministicCborKey(entry.key, state)
                }
                rejectDuplicateProviderDeterministicCborMapKeys(
                    value.mapValue.entriesList,
                )
                for (entry in value.mapValue.entriesList) {
                    validateProviderDeterministicCborValue(entry.value, childDepth, state)
                }
            }
            CodecDeterministicCborValue.ValueCase.VALUE_NOT_SET,
            null,
            -> throw ReallyMeCodecException.ProviderFailure()
        }
    }

    private fun rejectDuplicateProviderDeterministicCborMapKeys(
        entries: List<CodecDeterministicCborMapEntry>,
    ) {
        val keys = ArrayList<CodecDeterministicCborMapKey>(entries.size)
        for (entry in entries) {
            keys.add(entry.key)
        }
        keys.sortWith(::compareProviderDeterministicCborMapKeys)
        for (index in 1 until keys.size) {
            if (compareProviderDeterministicCborMapKeys(keys[index - 1], keys[index]) == 0) {
                throw ReallyMeCodecException.ProviderFailure()
            }
        }
    }

    private fun compareProviderDeterministicCborMapKeys(
        left: CodecDeterministicCborMapKey,
        right: CodecDeterministicCborMapKey,
    ): Int {
        val leftRank = providerDeterministicCborMapKeyRank(left.keyCase)
        val rightRank = providerDeterministicCborMapKeyRank(right.keyCase)
        if (leftRank != rightRank) {
            return leftRank.compareTo(rightRank)
        }
        return when (left.keyCase) {
            CodecDeterministicCborMapKey.KeyCase.INTEGER_KEY ->
                compareProviderDeterministicCborIntegers(left.integerKey, right.integerKey)
            CodecDeterministicCborMapKey.KeyCase.TEXT_KEY ->
                left.textKey.value.compareTo(right.textKey.value)
            CodecDeterministicCborMapKey.KeyCase.KEY_NOT_SET,
            null,
            -> 0
        }
    }

    private fun providerDeterministicCborMapKeyRank(
        keyCase: CodecDeterministicCborMapKey.KeyCase?,
    ): Int =
        when (keyCase) {
            CodecDeterministicCborMapKey.KeyCase.INTEGER_KEY -> 0
            CodecDeterministicCborMapKey.KeyCase.TEXT_KEY -> 1
            CodecDeterministicCborMapKey.KeyCase.KEY_NOT_SET,
            null,
            -> 2
        }

    private fun compareProviderDeterministicCborIntegers(
        left: CodecDeterministicCborInteger,
        right: CodecDeterministicCborInteger,
    ): Int {
        val leftRank = providerDeterministicCborIntegerRank(left.valueCase)
        val rightRank = providerDeterministicCborIntegerRank(right.valueCase)
        if (leftRank != rightRank) {
            return leftRank.compareTo(rightRank)
        }
        return when (left.valueCase) {
            CodecDeterministicCborInteger.ValueCase.UNSIGNED_VALUE ->
                left.unsignedValue.value.toULong().compareTo(right.unsignedValue.value.toULong())
            CodecDeterministicCborInteger.ValueCase.NEGATIVE_VALUE ->
                left.negativeValue.value.compareTo(right.negativeValue.value)
            CodecDeterministicCborInteger.ValueCase.VALUE_NOT_SET,
            null,
            -> 0
        }
    }

    private fun providerDeterministicCborIntegerRank(
        valueCase: CodecDeterministicCborInteger.ValueCase?,
    ): Int =
        when (valueCase) {
            CodecDeterministicCborInteger.ValueCase.UNSIGNED_VALUE -> 0
            CodecDeterministicCborInteger.ValueCase.NEGATIVE_VALUE -> 1
            CodecDeterministicCborInteger.ValueCase.VALUE_NOT_SET,
            null,
            -> 2
        }

    private fun validateProviderDeterministicCborInteger(
        integer: CodecDeterministicCborInteger,
    ) {
        requireNoProviderUnknownFields(integer.reallyMeHasUnknownFieldsForValidation())
        when (integer.valueCase) {
            CodecDeterministicCborInteger.ValueCase.UNSIGNED_VALUE ->
                requireNoProviderUnknownFields(
                    integer.unsignedValue.reallyMeHasUnknownFieldsForValidation()
                )
            CodecDeterministicCborInteger.ValueCase.NEGATIVE_VALUE -> {
                requireNoProviderUnknownFields(
                    integer.negativeValue.reallyMeHasUnknownFieldsForValidation()
                )
                if (integer.negativeValue.value >= 0) {
                    throw ReallyMeCodecException.ProviderFailure()
                }
            }
            CodecDeterministicCborInteger.ValueCase.VALUE_NOT_SET,
            null,
            -> throw ReallyMeCodecException.ProviderFailure()
        }
    }

    private fun validateProviderDeterministicCborKey(
        key: CodecDeterministicCborMapKey,
        state: DeterministicCborValidationState,
    ) {
        requireNoProviderUnknownFields(key.reallyMeHasUnknownFieldsForValidation())
        when (key.keyCase) {
            CodecDeterministicCborMapKey.KeyCase.INTEGER_KEY ->
                validateProviderDeterministicCborInteger(key.integerKey)
            CodecDeterministicCborMapKey.KeyCase.TEXT_KEY -> {
                requireNoProviderUnknownFields(
                    key.textKey.reallyMeHasUnknownFieldsForValidation()
                )
                state.textBytes = addProviderDeterministicCborCount(
                    state.textBytes,
                    providerDeterministicCborUtf8Length(key.textKey.value),
                    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
                )
            }
            CodecDeterministicCborMapKey.KeyCase.KEY_NOT_SET,
            null,
            -> throw ReallyMeCodecException.ProviderFailure()
        }
    }

    private fun requireNoProviderUnknownFields(hasUnknownFields: Boolean) {
        if (hasUnknownFields) {
            throw ReallyMeCodecException.ProviderFailure()
        }
    }

    private fun validateProviderDeterministicCborContainer(count: Int) {
        if (count > MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES) {
            throw ReallyMeCodecException.ProviderFailure()
        }
    }

    private fun providerDeterministicCborChildDepth(depth: Int): Int {
        if (depth >= MAX_DETERMINISTIC_CBOR_NESTING_DEPTH) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        return depth + 1
    }

    private fun addProviderDeterministicCborCount(
        current: Int,
        increment: Int,
        maximum: Int,
    ): Int {
        if (increment < 0 || current > maximum - increment) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        return current + increment
    }

    private fun providerDeterministicCborUtf8Length(text: String): Int =
        try {
            deterministicCborUtf8Length(text)
        } catch (_: ReallyMeCodecException.InvalidInput) {
            throw ReallyMeCodecException.ProviderFailure()
        }

    private fun deterministicCborEncodeRequest(
        value: ReallyMeDeterministicCborValue,
    ): CodecOperationRequest =
        CodecOperationRequest.newBuilder()
            .setDeterministicCborEncode(
                CodecDeterministicCborEncodeRequest.newBuilder()
                    .setValue(protoValue(value))
            )
            .build()

    private fun dagCborEncodeRequest(
        value: ReallyMeDeterministicCborValue,
    ): CodecOperationRequest =
        CodecOperationRequest.newBuilder()
            .setDagCborEncode(
                CodecDagCborEncodeRequest.newBuilder()
                    .setValue(protoValue(value))
            )
            .build()

    private fun deterministicCborDecodeRequest(bytes: ByteArray): CodecOperationRequest {
        requireBoundaryAggregate(bytes.size)
        return CodecOperationRequest.newBuilder()
            .setDeterministicCborDecode(
                CodecDeterministicCborDecodeRequest.newBuilder()
                    .setEncoded(borrowedByteString(bytes))
            )
            .build()
    }

    private fun dagCborDecodeRequest(bytes: ByteArray): CodecOperationRequest {
        requireBoundaryAggregate(bytes.size)
        return CodecOperationRequest.newBuilder()
            .setDagCborDecode(
                CodecDagCborDecodeRequest.newBuilder()
                    .setEncoded(borrowedByteString(bytes))
            )
            .build()
    }

    private fun protoInteger(
        integer: ReallyMeDeterministicCborInteger,
    ): CodecDeterministicCborInteger =
        when (integer) {
            is ReallyMeDeterministicCborInteger.Unsigned ->
                CodecDeterministicCborInteger.newBuilder()
                    .setUnsignedValue(
                        CodecDeterministicCborUnsignedInteger.newBuilder()
                            .setValue(integer.value.toLong())
                    )
                    .build()
            is ReallyMeDeterministicCborInteger.Negative ->
                CodecDeterministicCborInteger.newBuilder()
                    .setNegativeValue(
                        CodecDeterministicCborNegativeInteger.newBuilder()
                            .setValue(integer.value)
                    )
                    .build()
        }

    private fun protoKey(key: ReallyMeDeterministicCborMapKey): CodecDeterministicCborMapKey =
        when (key) {
            is ReallyMeDeterministicCborMapKey.Integer ->
                CodecDeterministicCborMapKey.newBuilder()
                    .setIntegerKey(protoInteger(key.value))
                    .build()
            is ReallyMeDeterministicCborMapKey.Text ->
                CodecDeterministicCborMapKey.newBuilder()
                    .setTextKey(
                        CodecDeterministicCborText.newBuilder()
                            .setValue(key.value)
                    )
                    .build()
        }

    private fun protoValue(value: ReallyMeDeterministicCborValue): CodecDeterministicCborValue =
        when (value) {
            ReallyMeDeterministicCborValue.Null ->
                CodecDeterministicCborValue.newBuilder()
                    .setNullValue(CodecDeterministicCborNull.getDefaultInstance())
                    .build()
            is ReallyMeDeterministicCborValue.Bool ->
                CodecDeterministicCborValue.newBuilder()
                    .setBoolValue(
                        CodecDeterministicCborBool.newBuilder()
                            .setValue(value.value)
                    )
                    .build()
            is ReallyMeDeterministicCborValue.Integer ->
                CodecDeterministicCborValue.newBuilder()
                    .setIntegerValue(protoInteger(value.value))
                    .build()
            is ReallyMeDeterministicCborValue.Text ->
                CodecDeterministicCborValue.newBuilder()
                    .setTextValue(
                        CodecDeterministicCborText.newBuilder()
                            .setValue(value.value)
                    )
                    .build()
            is ReallyMeDeterministicCborValue.Bytes ->
                CodecDeterministicCborValue.newBuilder()
                    .setBytesValue(
                        CodecDeterministicCborBytes.newBuilder()
                            .setValue(borrowedByteString(value.borrowedBytes()))
                    )
                    .build()
            is ReallyMeDeterministicCborValue.Array ->
                CodecDeterministicCborValue.newBuilder()
                    .setArrayValue(
                        CodecDeterministicCborArray.newBuilder()
                            .addAllValues(value.values.map(::protoValue))
                    )
                    .build()
            is ReallyMeDeterministicCborValue.Map ->
                CodecDeterministicCborValue.newBuilder()
                    .setMapValue(
                        CodecDeterministicCborMap.newBuilder()
                            .addAllEntries(
                                value.entries.map { entry ->
                                    CodecDeterministicCborMapEntry.newBuilder()
                                        .setKey(protoKey(entry.key))
                                        .setValue(protoValue(entry.value))
                                        .build()
                                }
                            )
                    )
                    .build()
        }

    private fun sdkInteger(
        integer: CodecDeterministicCborInteger,
    ): ReallyMeDeterministicCborInteger =
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

    private fun sdkValue(value: CodecDeterministicCborValue): ReallyMeDeterministicCborValue =
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
                    value.bytesValue.value.toByteArray()
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
                    }
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
    private fun borrowedByteString(bytes: ByteArray): ByteString =
        UnsafeByteOperations.unsafeWrap(bytes)

    private fun bytes(text: String): ByteArray {
        utf8Length(text)
        val encoded = text.toByteArray(Charsets.UTF_8)
        if (encoded.size > ffiInputLimit()) {
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
        val maximum = ffiInputLimit()
        if (text.length > maximum) {
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
                Character.isSurrogate(character) ->
                    throw ReallyMeCodecException.InvalidInput()
                else -> 3
            }
            if (length > maximum - increment) {
                throw ReallyMeCodecException.InvalidInput()
            }
            length += increment
            index += 1
        }
        return length
    }

    private fun requireBoundaryAggregate(vararg lengths: Int) {
        val maximum = ffiInputLimit()
        var aggregate = 0L
        for (length in lengths) {
            aggregate += length.toLong()
            if (aggregate > maximum.toLong()) {
                throw ReallyMeCodecException.InvalidInput()
            }
        }
    }

    private fun ffiInputLimit(): Int =
        ReallyMeCodecRustNativeProvider.ffiInputLimit()

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
