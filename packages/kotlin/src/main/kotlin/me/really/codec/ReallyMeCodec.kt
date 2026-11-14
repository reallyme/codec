// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import me.really.codec.v1.CodecDeterministicCborValue
import me.really.codec.v1.CodecLegacyBoolOperation
import me.really.codec.v1.CodecLegacyScalarOperation
import me.really.codec.v1.CodecOperationRequest
import me.really.codec.v1.CodecOperationResult

private object CodecOperation {
    val BASE64_ENCODE: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_BASE64_ENCODE.number
    val BASE64_DECODE: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_BASE64_DECODE.number
    val BASE64URL_ENCODE: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_BASE64URL_ENCODE.number
    val BASE64URL_DECODE: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_BASE64URL_DECODE.number
    val LOWER_HEX_ENCODE: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_LOWER_HEX_ENCODE.number
    val LOWER_HEX_DECODE: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_LOWER_HEX_DECODE.number
    val BASE58BTC_ENCODE: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_BASE58BTC_ENCODE.number
    val BASE58BTC_DECODE: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_BASE58BTC_DECODE.number
    val MULTIBASE_BASE58BTC_ENCODE: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_MULTIBASE_BASE58BTC_ENCODE.number
    val MULTIBASE_BASE64URL_ENCODE: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_MULTIBASE_BASE64URL_ENCODE.number
    val MULTIBASE_DECODE: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_MULTIBASE_DECODE.number
    val MULTICODEC_STRIP_PREFIX: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_MULTICODEC_STRIP_PREFIX.number
    val MULTIKEY_ENCODE: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_MULTIKEY_ENCODE.number
    val REQUIRE_SUPPORTED_MULTICODEC: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_REQUIRE_SUPPORTED_MULTICODEC.number
    val DAG_CBOR_COMPUTE_CID: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_DAG_CBOR_COMPUTE_CID.number
    val DAG_CBOR_SHA256_CONTENT_HASH: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_DAG_CBOR_SHA256_CONTENT_HASH.number
    val DAG_CBOR_MULTIHASH: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_DAG_CBOR_MULTIHASH.number
    val TRY_PARSE_CID: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_TRY_PARSE_CID.number
    val DAG_CBOR_CODEC_CODE: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_DAG_CBOR_CODEC_CODE.number
    val CANONICALIZE_JSON: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_CANONICALIZE_JSON.number
    val VALIDATE_KEY_BINDING: Int = CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_VALIDATE_KEY_BINDING.number
    val VALIDATE_KEY_BINDING_NO_ALGORITHM: Int =
        CodecLegacyScalarOperation.CODEC_LEGACY_SCALAR_OPERATION_VALIDATE_KEY_BINDING_NO_ALGORITHM.number
}

private object CodecBoolOperation {
    val BINDING_TYPE_MATCHES_CODEC: Int = CodecLegacyBoolOperation.CODEC_LEGACY_BOOL_OPERATION_BINDING_TYPE_MATCHES_CODEC.number
    val IS_VALID_CID_STRING: Int = CodecLegacyBoolOperation.CODEC_LEGACY_BOOL_OPERATION_IS_VALID_CID_STRING.number
}

/**
 * Kotlin facade for ReallyMe codec operations backed by the Rust codec crates.
 */
public object ReallyMeCodec {
    @JvmStatic
    public fun base64Encode(bytes: ByteArray): String = text(process(CodecOperation.BASE64_ENCODE, bytes))

    @JvmStatic
    public fun base64Decode(text: String): ByteArray =
        withTextBytes(text) { encoded ->
            process(CodecOperation.BASE64_DECODE, encoded)
        }

    @JvmStatic
    public fun base64urlEncode(bytes: ByteArray): String = text(process(CodecOperation.BASE64URL_ENCODE, bytes))

    @JvmStatic
    public fun base64urlDecode(text: String): ByteArray =
        withTextBytes(text) { encoded ->
            process(CodecOperation.BASE64URL_DECODE, encoded)
        }

    @JvmStatic
    public fun bytesToLowerHex(bytes: ByteArray): String = text(process(CodecOperation.LOWER_HEX_ENCODE, bytes))

    @JvmStatic
    public fun lowerHexToBytes(text: String): ByteArray =
        withTextBytes(text) { encoded ->
            process(CodecOperation.LOWER_HEX_DECODE, encoded)
        }

    @JvmStatic
    public fun base58btcEncode(bytes: ByteArray): String = text(process(CodecOperation.BASE58BTC_ENCODE, bytes))

    @JvmStatic
    public fun base58btcDecode(text: String): ByteArray =
        withTextBytes(text) { encoded ->
            process(CodecOperation.BASE58BTC_DECODE, encoded)
        }

    @JvmStatic
    public fun multibaseBase58btcEncode(bytes: ByteArray): String = text(process(CodecOperation.MULTIBASE_BASE58BTC_ENCODE, bytes))

    @JvmStatic
    public fun multibaseBase64urlEncode(bytes: ByteArray): String = text(process(CodecOperation.MULTIBASE_BASE64URL_ENCODE, bytes))

    @JvmStatic
    public fun multibaseDecode(text: String): ByteArray =
        withTextBytes(text) { encoded ->
            process(CodecOperation.MULTIBASE_DECODE, encoded)
        }

    @JvmStatic
    public fun multicodecPrefixForName(name: String): ReallyMeMulticodecMetadata =
        processOperation(multicodecPrefixForNameRequest(name)) { result ->
            if (result.resultCase != CodecOperationResult.ResultCase.MULTICODEC_PREFIX_FOR_NAME) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            sdkMulticodecMetadata(result.multicodecPrefixForName)
        }

    @JvmStatic
    public fun multicodecLookupPrefix(bytes: ByteArray): ReallyMeMulticodecLookupResult =
        processOperation(multicodecLookupPrefixRequest(bytes)) { result ->
            if (result.resultCase != CodecOperationResult.ResultCase.MULTICODEC_LOOKUP_PREFIX) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            sdkMulticodecLookupResult(result.multicodecLookupPrefix)
        }

    @JvmStatic
    public fun multicodecStripPrefix(bytes: ByteArray): ByteArray = process(CodecOperation.MULTICODEC_STRIP_PREFIX, bytes)

    @JvmStatic
    public fun multicodecTable(): ReallyMeMulticodecTable =
        processOperation(multicodecTableRequest()) { result ->
            if (result.resultCase != CodecOperationResult.ResultCase.MULTICODEC_TABLE) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            sdkMulticodecTable(result.multicodecTable)
        }

    @JvmStatic
    public fun multikeyEncode(
        codecName: String,
        publicKey: ByteArray,
    ): String =
        withTextBytes(codecName) { encodedCodecName ->
            text(process(CodecOperation.MULTIKEY_ENCODE, encodedCodecName, publicKey))
        }

    @JvmStatic
    public fun multikeyParse(multikey: String): ReallyMeParsedMultikey =
        processOperation(multikeyParseRequest(multikey)) { result ->
            if (result.resultCase != CodecOperationResult.ResultCase.MULTIKEY_PARSE) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            sdkParsedMultikey(result.multikeyParse)
        }

    @JvmStatic
    public fun requireSupportedMulticodec(name: String) {
        withTextBytes(name) { encoded ->
            process(CodecOperation.REQUIRE_SUPPORTED_MULTICODEC, encoded)
        }
    }

    @JvmStatic
    public fun bindingTypeMatchesCodec(
        bindingType: String,
        codecName: String,
    ): Boolean =
        withTextBytes(bindingType, codecName) { encodedBindingType, encodedCodecName ->
            processBool(
                CodecBoolOperation.BINDING_TYPE_MATCHES_CODEC,
                encodedBindingType,
                encodedCodecName,
            )
        }

    @JvmStatic
    public fun validateKeyBinding(
        bindingType: String,
        algorithm: String?,
        multikey: String,
    ) {
        if (algorithm != null && algorithm.isEmpty()) {
            throw ReallyMeCodecException.InvalidInput()
        }
        withTextBytes(bindingType, algorithm ?: "", multikey) { encodedBindingType, encodedAlgorithm, encodedMultikey ->
            process(
                if (algorithm == null) CodecOperation.VALIDATE_KEY_BINDING_NO_ALGORITHM else CodecOperation.VALIDATE_KEY_BINDING,
                encodedBindingType,
                encodedAlgorithm,
                encodedMultikey,
            )
        }
    }

    @JvmStatic
    public fun dagCborEncode(value: ReallyMeDeterministicCborValue): ByteArray {
        validateDeterministicCborValue(value)
        return processOperation(dagCborEncodeRequest(value)) { operationResult ->
            if (operationResult.resultCase != CodecOperationResult.ResultCase.DAG_CBOR_ENCODE) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            val result = operationResult.dagCborEncode
            if (result.reallyMeHasUnknownFieldsForValidation()) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            result.encoded.toByteArray()
        }
    }

    @JvmStatic
    public fun dagCborDecode(bytes: ByteArray): ReallyMeDeterministicCborValue {
        requireBoundaryAggregate(bytes.size)
        val ownedBytes = bytes.copyOf()
        return try {
            processOperation(dagCborDecodeRequest(ownedBytes)) { operationResult ->
                if (operationResult.resultCase != CodecOperationResult.ResultCase.DAG_CBOR_DECODE) {
                    throw ReallyMeCodecException.ProviderFailure()
                }
                val result = operationResult.dagCborDecode
                if (result.reallyMeHasUnknownFieldsForValidation() || !result.hasValue()) {
                    throw ReallyMeCodecException.ProviderFailure()
                }
                validateProviderDeterministicCborValue(result.value)
                sdkValue(result.value)
            }
        } finally {
            ownedBytes.fill(0)
        }
    }

    @JvmStatic
    public fun dagCborComputeCid(bytes: ByteArray): String = text(process(CodecOperation.DAG_CBOR_COMPUTE_CID, bytes))

    @JvmStatic
    public fun dagCborVerifyCid(
        cid: String,
        bytes: ByteArray,
    ): ReallyMeDagCborCidVerification =
        processOperation(dagCborVerifyCidRequest(cid, bytes)) { result ->
            if (result.resultCase != CodecOperationResult.ResultCase.DAG_CBOR_VERIFY_CID) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            sdkDagCborCidVerification(result.dagCborVerifyCid)
        }

    @JvmStatic
    public fun dagCborSha256ContentHash(bytes: ByteArray): ByteArray = process(CodecOperation.DAG_CBOR_SHA256_CONTENT_HASH, bytes)

    @JvmStatic
    public fun dagCborMultihash(bytes: ByteArray): ByteArray = process(CodecOperation.DAG_CBOR_MULTIHASH, bytes)

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
        return processOperation(deterministicCborEncodeRequest(value)) { operationResult ->
            if (operationResult.resultCase != CodecOperationResult.ResultCase.DETERMINISTIC_CBOR_ENCODE) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            val result = operationResult.deterministicCborEncode
            if (result.reallyMeHasUnknownFieldsForValidation()) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            result.encoded.toByteArray()
        }
    }

    @JvmStatic
    public fun deterministicCborDecode(bytes: ByteArray): ReallyMeDeterministicCborValue {
        requireBoundaryAggregate(bytes.size)
        // ByteArray is mutable and may be shared with another thread. Validate
        // and serialize one SDK-owned snapshot, then wipe that same owner.
        val ownedBytes = bytes.copyOf()
        return try {
            processOperation(deterministicCborDecodeRequest(ownedBytes)) { operationResult ->
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
            }
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
    ): ReallyMePemDocument =
        processOperation(pemDecodeRequest(pem, options)) { result ->
            if (result.resultCase != CodecOperationResult.ResultCase.PEM_DECODE) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            sdkPemDocument(result.pemDecode)
        }

    @JvmStatic
    @JvmOverloads
    public fun encodePem(
        label: ReallyMePemLabel,
        der: ByteArray,
        options: ReallyMePemEncodeOptions = ReallyMePemEncodeOptions(),
    ): ByteArray =
        processOperation(pemEncodeRequest(label, der, options)) { result ->
            if (result.resultCase != CodecOperationResult.ResultCase.PEM_ENCODE) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            val pem = result.pemEncode
            if (pem.reallyMeHasUnknownFieldsForValidation()) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            pem.pem.toByteArray()
        }

    internal fun validateProviderDeterministicCborValue(value: CodecDeterministicCborValue): Unit = validateProviderCborTree(value)
}
