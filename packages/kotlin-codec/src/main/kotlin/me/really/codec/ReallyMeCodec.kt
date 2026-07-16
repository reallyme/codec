// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec

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
        process(CodecOperation.BASE64_DECODE, bytes(text))

    @JvmStatic
    public fun base64urlEncode(bytes: ByteArray): String =
        text(process(CodecOperation.BASE64URL_ENCODE, bytes))

    @JvmStatic
    public fun base64urlDecode(text: String): ByteArray =
        process(CodecOperation.BASE64URL_DECODE, bytes(text))

    @JvmStatic
    public fun bytesToLowerHex(bytes: ByteArray): String =
        text(process(CodecOperation.LOWER_HEX_ENCODE, bytes))

    @JvmStatic
    public fun lowerHexToBytes(text: String): ByteArray =
        process(CodecOperation.LOWER_HEX_DECODE, bytes(text))

    @JvmStatic
    public fun base58btcEncode(bytes: ByteArray): String =
        text(process(CodecOperation.BASE58BTC_ENCODE, bytes))

    @JvmStatic
    public fun base58btcDecode(text: String): ByteArray =
        process(CodecOperation.BASE58BTC_DECODE, bytes(text))

    @JvmStatic
    public fun multibaseBase58btcEncode(bytes: ByteArray): String =
        text(process(CodecOperation.MULTIBASE_BASE58BTC_ENCODE, bytes))

    @JvmStatic
    public fun multibaseBase64urlEncode(bytes: ByteArray): String =
        text(process(CodecOperation.MULTIBASE_BASE64URL_ENCODE, bytes))

    @JvmStatic
    public fun multibaseDecode(text: String): ByteArray =
        process(CodecOperation.MULTIBASE_DECODE, bytes(text))

    @JvmStatic
    public fun multicodecPrefixForName(name: String): String =
        text(process(CodecOperation.MULTICODEC_PREFIX_FOR_NAME, bytes(name)))

    @JvmStatic
    public fun multicodecPrefixForNameProto(name: String): ByteArray =
        processProto(CodecOperation.MULTICODEC_PREFIX_FOR_NAME, bytes(name))

    @JvmStatic
    public fun multicodecPrefixForNameProtoResult(name: String): ReallyMeCodecProtoResult =
        processProtoResult(CodecOperation.MULTICODEC_PREFIX_FOR_NAME, bytes(name))

    @JvmStatic
    public fun multicodecLookupPrefix(bytes: ByteArray): String =
        text(process(CodecOperation.MULTICODEC_LOOKUP_PREFIX, bytes))

    @JvmStatic
    public fun multicodecLookupPrefixProto(bytes: ByteArray): ByteArray =
        processProto(CodecOperation.MULTICODEC_LOOKUP_PREFIX, bytes)

    @JvmStatic
    public fun multicodecLookupPrefixProtoResult(bytes: ByteArray): ReallyMeCodecProtoResult =
        processProtoResult(CodecOperation.MULTICODEC_LOOKUP_PREFIX, bytes)

    @JvmStatic
    public fun multicodecStripPrefix(bytes: ByteArray): ByteArray =
        process(CodecOperation.MULTICODEC_STRIP_PREFIX, bytes)

    @JvmStatic
    public fun multicodecTable(): String =
        text(process(CodecOperation.MULTICODEC_TABLE, emptyBytes))

    @JvmStatic
    public fun multicodecTableProto(): ByteArray =
        processProto(CodecOperation.MULTICODEC_TABLE, emptyBytes)

    @JvmStatic
    public fun multicodecTableProtoResult(): ReallyMeCodecProtoResult =
        processProtoResult(CodecOperation.MULTICODEC_TABLE, emptyBytes)

    @JvmStatic
    public fun multikeyEncode(codecName: String, publicKey: ByteArray): String =
        text(process(CodecOperation.MULTIKEY_ENCODE, bytes(codecName), publicKey))

    @JvmStatic
    public fun multikeyParse(multikey: String): String =
        text(process(CodecOperation.MULTIKEY_PARSE, bytes(multikey)))

    @JvmStatic
    public fun multikeyParseProto(multikey: String): ByteArray =
        processProto(CodecOperation.MULTIKEY_PARSE, bytes(multikey))

    @JvmStatic
    public fun multikeyParseProtoResult(multikey: String): ReallyMeCodecProtoResult =
        processProtoResult(CodecOperation.MULTIKEY_PARSE, bytes(multikey))

    @JvmStatic
    public fun requireSupportedMulticodec(name: String) {
        process(CodecOperation.REQUIRE_SUPPORTED_MULTICODEC, bytes(name))
    }

    @JvmStatic
    public fun bindingTypeMatchesCodec(bindingType: String, codecName: String): Boolean =
        processBool(CodecBoolOperation.BINDING_TYPE_MATCHES_CODEC, bytes(bindingType), bytes(codecName))

    @JvmStatic
    public fun validateKeyBinding(bindingType: String, algorithm: String?, multikey: String) {
        process(
            CodecOperation.VALIDATE_KEY_BINDING,
            bytes(bindingType),
            bytes(algorithm ?: ""),
            bytes(multikey),
        )
    }

    @JvmStatic
    public fun dagCborEncode(taggedJson: String): ByteArray =
        process(CodecOperation.DAG_CBOR_ENCODE, bytes(taggedJson))

    @JvmStatic
    public fun dagCborDecode(bytes: ByteArray): String =
        text(process(CodecOperation.DAG_CBOR_DECODE, bytes))

    @JvmStatic
    public fun dagCborComputeCid(bytes: ByteArray): String =
        text(process(CodecOperation.DAG_CBOR_COMPUTE_CID, bytes))

    @JvmStatic
    public fun dagCborVerifyCid(cid: String, bytes: ByteArray): String =
        text(process(CodecOperation.DAG_CBOR_VERIFY_CID, bytes(cid), bytes))

    @JvmStatic
    public fun dagCborVerifyCidProto(cid: String, bytes: ByteArray): ByteArray =
        processProto(CodecOperation.DAG_CBOR_VERIFY_CID, bytes(cid), bytes)

    @JvmStatic
    public fun dagCborVerifyCidProtoResult(cid: String, bytes: ByteArray): ReallyMeCodecProtoResult =
        processProtoResult(CodecOperation.DAG_CBOR_VERIFY_CID, bytes(cid), bytes)

    @JvmStatic
    public fun dagCborSha256ContentHash(bytes: ByteArray): ByteArray =
        process(CodecOperation.DAG_CBOR_SHA256_CONTENT_HASH, bytes)

    @JvmStatic
    public fun dagCborMultihash(bytes: ByteArray): ByteArray =
        process(CodecOperation.DAG_CBOR_MULTIHASH, bytes)

    @JvmStatic
    public fun isValidCidString(cid: String): Boolean =
        processBool(CodecBoolOperation.IS_VALID_CID_STRING, bytes(cid), emptyBytes)

    @JvmStatic
    public fun tryParseCid(cid: String): String? =
        try {
            text(process(CodecOperation.TRY_PARSE_CID, bytes(cid)))
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
        text(process(CodecOperation.CANONICALIZE_JSON, bytes(json)))

    @JvmStatic
    @JvmOverloads
    public fun decodePem(pem: String, optionsJson: String = ""): String =
        text(process(CodecOperation.PEM_DECODE, bytes(pem), bytes(optionsJson)))

    @JvmStatic
    @JvmOverloads
    public fun decodePemProto(pem: String, optionsJson: String = ""): ByteArray =
        processProto(CodecOperation.PEM_DECODE, bytes(pem), bytes(optionsJson))

    @JvmStatic
    @JvmOverloads
    public fun decodePemProtoResult(pem: String, optionsJson: String = ""): ReallyMeCodecProtoResult =
        processProtoResult(CodecOperation.PEM_DECODE, bytes(pem), bytes(optionsJson))

    @JvmStatic
    @JvmOverloads
    public fun encodePem(label: String, der: ByteArray, optionsJson: String = ""): String =
        text(process(CodecOperation.PEM_ENCODE, bytes(label), der, bytes(optionsJson)))

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

    private fun processProto(
        operation: Int,
        first: ByteArray,
        second: ByteArray = emptyBytes,
        third: ByteArray = emptyBytes,
    ): ByteArray {
        val result = processProtoResult(operation, first, second, third)
        if (result.status == ReallyMeCodecProtoStatus.CODEC_ERROR) {
            throw ReallyMeCodecException.InvalidInput()
        }
        return result.bytes
    }

    private fun processProtoResult(
        operation: Int,
        first: ByteArray,
        second: ByteArray = emptyBytes,
        third: ByteArray = emptyBytes,
    ): ReallyMeCodecProtoResult {
        ReallyMeCodecRustNativeProvider.requireLoaded()
        return try {
            ReallyMeCodecNative.processProtoResultNative(operation, first, second, third)
        } catch (error: UnsatisfiedLinkError) {
            throw ReallyMeCodecException.ProviderFailure()
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

    private fun bytes(text: String): ByteArray = text.toByteArray(Charsets.UTF_8)

    private fun text(bytes: ByteArray): String = bytes.toString(Charsets.UTF_8)
}
