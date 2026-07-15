// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec

import me.really.codec.v1.CodecDagCborVerifyCidResult
import me.really.codec.v1.CodecError
import me.really.codec.v1.CodecErrorReason
import me.really.codec.v1.CodecMulticodecLookupResult
import me.really.codec.v1.CodecMulticodecSpec
import me.really.codec.v1.CodecMulticodecTableResult
import me.really.codec.v1.CodecMultikeyParseResult
import me.really.codec.v1.CodecPemDecodeResult
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ReallyMeCodecTest {
    private companion object {
        private const val TEST_LIBRARY_PROPERTY = "reallyme.codec.testLibraryPath"
    }

    @Test
    fun providerLoadingFailsClosed() {
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            ReallyMeCodecRustNativeProvider.loadLibrary("")
        }
        assertFailsWith<ReallyMeCodecException.ProviderFailure> {
            ReallyMeCodecRustNativeProvider.loadLibrary("/tmp/reallyme-codec-missing-library.dylib")
        }
    }

    @Test
    fun androidRuntimeUsesSystemLibraryPathInsteadOfClasspathResource() {
        assertTrue(
            ReallyMeCodecRustNativeProvider.isAndroidRuntime(
                runtimeName = "Android Runtime",
                vmName = "Dalvik",
                vmVendor = "The Android Project",
            ),
        )
        assertEquals(
            null,
            ReallyMeCodecRustNativeProvider.platformNativeResource(
                osName = "Linux",
                osArch = "aarch64",
                androidRuntime = true,
            ),
        )
    }

    @Test
    fun jvmRuntimeUsesBundledPlatformResourcePath() {
        assertFalse(
            ReallyMeCodecRustNativeProvider.isAndroidRuntime(
                runtimeName = "OpenJDK Runtime Environment",
                vmName = "OpenJDK 64-Bit Server VM",
                vmVendor = "Eclipse Adoptium",
            ),
        )
        assertEquals(
            ReallyMeCodecRustNativeProvider.NativeResource(
                fileName = "libreallyme_codec_ffi.so",
                path = "/me/really/codec/native/linux-aarch64/libreallyme_codec_ffi.so",
            ),
            ReallyMeCodecRustNativeProvider.platformNativeResource(
                osName = "Linux",
                osArch = "aarch64",
                androidRuntime = false,
            ),
        )
    }

    @Test
    fun baseEncodingsHandleEmptyLargeAndInvalidInput() {
        val codec = configuredCodec()
        val empty = byteArrayOf()
        val large = ByteArray(4096) { index -> (index % 251).toByte() }

        assertEquals("", codec.base64Encode(empty))
        assertContentEquals(empty, codec.base64Decode(""))
        assertEquals("", codec.base64urlEncode(empty))
        assertContentEquals(empty, codec.base64urlDecode(""))
        assertEquals("", codec.bytesToLowerHex(empty))
        assertContentEquals(empty, codec.lowerHexToBytes(""))

        val largeBase64url = codec.base64urlEncode(large)
        assertContentEquals(large, codec.base64urlDecode(largeBase64url))
        assertContentEquals(large, codec.lowerHexToBytes(codec.bytesToLowerHex(large)))

        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.base64Decode("Zh==")
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.base64Decode("AAEC-_8=")
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.base64urlDecode("AAEC-_8=")
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.lowerHexToBytes("DEADBEEF")
        }
    }

    @Test
    fun multibaseMulticodecAndMultikeyUseRustProvider() {
        val codec = configuredCodec()
        val publicKey = ByteArray(32)
        publicKey[31] = 7

        val base58 = codec.base58btcEncode(publicKey)
        assertContentEquals(publicKey, codec.base58btcDecode(base58))
        assertContentEquals(byteArrayOf(), codec.base58btcDecode(""))
        val multibase58 = codec.multibaseBase58btcEncode(publicKey)
        assertTrue(multibase58.startsWith("z"))
        assertContentEquals(publicKey, codec.multibaseDecode(multibase58))
        val multibase64url = codec.multibaseBase64urlEncode(publicKey)
        assertTrue(multibase64url.startsWith("u"))
        assertContentEquals(publicKey, codec.multibaseDecode(multibase64url))
        assertEquals("u", codec.multibaseBase64urlEncode(byteArrayOf()))
        assertContentEquals(byteArrayOf(), codec.multibaseDecode("u"))
        val oversizedBase58Input = ByteArray(8 * 1024 + 1)
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.base58btcEncode(oversizedBase58Input)
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.multibaseBase58btcEncode(oversizedBase58Input)
        }

        val metadataJson = codec.multicodecPrefixForName("ed25519-pub")
        assertTrue(metadataJson.contains("\"name\":\"ed25519-pub\""))
        assertTrue(metadataJson.contains("\"tag\":\"key\""))
        val metadataProto = CodecMulticodecSpec.parseFrom(
            codec.multicodecPrefixForNameProto("ed25519-pub"),
        )
        val metadataProtoResult = codec.multicodecPrefixForNameProtoResult("ed25519-pub")
        assertEquals(ReallyMeCodecProtoStatus.RESULT, metadataProtoResult.status)
        assertFalse(metadataProtoResult.isCodecError)
        assertEquals(
            "ed25519-pub",
            CodecMulticodecSpec.parseFrom(metadataProtoResult.bytes).name,
        )
        assertEquals("ed25519-pub", metadataProto.name)
        assertEquals("Ed25519", metadataProto.algorithmName)
        assertEquals(32, metadataProto.fixedLength)
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.multicodecPrefixForNameProto("not-a-codec")
        }
        val metadataErrorResult = codec.multicodecPrefixForNameProtoResult("not-a-codec")
        assertEquals(ReallyMeCodecProtoStatus.CODEC_ERROR, metadataErrorResult.status)

        val prefixed = metadataProto.prefix.toByteArray() + publicKey
        assertTrue(codec.multicodecLookupPrefix(prefixed).contains("\"name\":\"ed25519-pub\""))
        val lookupProto = CodecMulticodecLookupResult.parseFrom(
            codec.multicodecLookupPrefixProto(prefixed),
        )
        assertEquals(ReallyMeCodecProtoStatus.RESULT, codec.multicodecLookupPrefixProtoResult(prefixed).status)
        assertEquals("ed25519-pub", lookupProto.name)
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.multicodecLookupPrefixProto(byteArrayOf(0, 0, 7))
        }
        val lookupErrorResult = codec.multicodecLookupPrefixProtoResult(byteArrayOf(0, 0, 7))
        assertEquals(ReallyMeCodecProtoStatus.CODEC_ERROR, lookupErrorResult.status)
        assertContentEquals(publicKey, codec.multicodecStripPrefix(prefixed))
        assertTrue(codec.multicodecTable().contains("mlkem-1024-pub"))
        val tableProto = CodecMulticodecTableResult.parseFrom(codec.multicodecTableProto())
        assertEquals(ReallyMeCodecProtoStatus.RESULT, codec.multicodecTableProtoResult().status)
        assertTrue(tableProto.entriesList.any { it.name == "mlkem-1024-pub" })

        val multikey = codec.multikeyEncode("ed25519-pub", publicKey)
        assertTrue(codec.multikeyParse(multikey).contains("\"codecName\":\"ed25519-pub\""))
        val parsedProto = CodecMultikeyParseResult.parseFrom(codec.multikeyParseProto(multikey))
        val parsedProtoResult = codec.multikeyParseProtoResult(multikey)
        assertEquals(ReallyMeCodecProtoStatus.RESULT, parsedProtoResult.status)
        assertEquals(
            "ed25519-pub",
            CodecMultikeyParseResult.parseFrom(parsedProtoResult.bytes).codecName,
        )
        assertEquals("ed25519-pub", parsedProto.codecName)
        assertEquals("Ed25519", parsedProto.algorithmName)
        assertContentEquals(publicKey, parsedProto.publicKey.toByteArray())
        assertTrue(codec.bindingTypeMatchesCodec("Multikey", "ed25519-pub"))
        codec.requireSupportedMulticodec("ed25519-pub")
        codec.validateKeyBinding("Multikey", null, multikey)

        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.requireSupportedMulticodec("not-a-codec")
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.validateKeyBinding("P256Key2024", "P-256", multikey)
        }

        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.multikeyParseProto("not-a-key")
        }
        val multikeyErrorResult = codec.multikeyParseProtoResult("not-a-key")
        assertEquals(ReallyMeCodecProtoStatus.CODEC_ERROR, multikeyErrorResult.status)
        assertTrue(multikeyErrorResult.isCodecError)
        val multikeyError = CodecError.parseFrom(multikeyErrorResult.bytes)
        assertEquals(
            CodecError.ErrorCase.MULTIFORMAT,
            multikeyError.errorCase,
        )
        assertEquals(
            CodecErrorReason.CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTIKEY,
            multikeyError.multiformat.reason,
        )
    }

    @Test
    fun dagCborCidAndJcsOperationsUseRustProvider() {
        val codec = configuredCodec()
        val taggedJson = """{"type":"map","value":[{"key":"b","value":{"type":"int","value":2}},{"key":"a","value":{"type":"string","value":"one"}},{"key":"bytes","value":{"type":"bytes","value":"AAEC"}}]}"""

        val encoded = codec.dagCborEncode(taggedJson)
        assertTrue(encoded.isNotEmpty())
        assertTrue(codec.dagCborDecode(encoded).contains("\"type\":\"map\""))

        val cid = codec.dagCborComputeCid(encoded)
        assertTrue(codec.isValidCidString(cid))
        assertFalse(codec.isValidCidString("not-a-cid"))
        assertEquals(cid, codec.tryParseCid(cid))
        assertNull(codec.tryParseCid("not-a-cid"))

        assertTrue(codec.dagCborVerifyCid(cid, encoded).contains("\"valid\":true"))
        val verificationProto = CodecDagCborVerifyCidResult.parseFrom(
            codec.dagCborVerifyCidProto(cid, encoded),
        )
        assertEquals(ReallyMeCodecProtoStatus.RESULT, codec.dagCborVerifyCidProtoResult(cid, encoded).status)
        assertTrue(verificationProto.valid)
        assertEquals(cid, verificationProto.expectedCid)

        val invalidUpperPayloadCid = cid.take(1) + cid.drop(1).uppercase()
        val invalidVerification = CodecDagCborVerifyCidResult.parseFrom(
            codec.dagCborVerifyCidProto(invalidUpperPayloadCid, encoded),
        )
        assertFalse(invalidVerification.valid)
        assertEquals("", invalidVerification.actualCid)
        val emptyCidVerification = CodecDagCborVerifyCidResult.parseFrom(
            codec.dagCborVerifyCidProto("", encoded),
        )
        assertTrue(codec.dagCborVerifyCid("", encoded).contains("\"valid\":false"))
        assertEquals(cid, emptyCidVerification.expectedCid)
        assertFalse(emptyCidVerification.valid)
        assertEquals(
            ReallyMeCodecProtoStatus.RESULT,
            codec.dagCborVerifyCidProtoResult("", encoded).status,
        )

        assertEquals(32, codec.dagCborSha256ContentHash(encoded).size)
        assertTrue(codec.dagCborMultihash(encoded).size > 32)
        assertEquals(0x71, codec.dagCborCodecCode())
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.dagCborDecode(byteArrayOf(0xa2.toByte(), 0x61, 0x62, 0x01, 0x61, 0x61, 0x02))
        }
        val oversizedCbor = ByteArray(1024 * 1024 + 1)
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.dagCborDecode(oversizedCbor)
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.dagCborComputeCid(oversizedCbor)
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.dagCborVerifyCid(cid, oversizedCbor)
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.dagCborVerifyCidProto(cid, oversizedCbor)
        }
        assertEquals(
            ReallyMeCodecProtoStatus.CODEC_ERROR,
            codec.dagCborVerifyCidProtoResult(cid, oversizedCbor).status,
        )
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.dagCborSha256ContentHash(oversizedCbor)
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.dagCborMultihash(oversizedCbor)
        }

        assertEquals("""{"a":1,"b":2}""", codec.canonicalizeJson("""{"b":2,"a":1}"""))
        assertEquals("333333333.3333333", codec.canonicalizeJson("333333333.33333329"))
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.canonicalizeJson("{")
        }
    }

    @Test
    fun pemRoundTripAndProtoErrorsUseRustProvider() {
        val codec = configuredCodec()
        val der = byteArrayOf(0x30, 0x03, 0x02, 0x01, 0x01)
        val pem = codec.encodePem("PRIVATE KEY", der)

        assertTrue(pem.contains("-----BEGIN PRIVATE KEY-----"))
        val decodedJson = codec.decodePem(pem)
        assertTrue(decodedJson.contains("\"label\":\"PRIVATE KEY\""))
        assertTrue(decodedJson.contains("\"der\":\"MAMCAQE\""))

        val decodedProto = CodecPemDecodeResult.parseFrom(codec.decodePemProto(pem))
        val decodedProtoResult = codec.decodePemProtoResult(pem)
        assertEquals(ReallyMeCodecProtoStatus.RESULT, decodedProtoResult.status)
        assertEquals("PRIVATE KEY", CodecPemDecodeResult.parseFrom(decodedProtoResult.bytes).label)
        assertEquals("PRIVATE KEY", decodedProto.label)
        assertContentEquals(der, decodedProto.der.toByteArray())

        val wrapped = codec.encodePem(
            "PUBLIC KEY",
            "not real der".toByteArray(Charsets.UTF_8),
            """{"lineWidth":4}""",
        )
        assertTrue(wrapped.contains("bm90\nIHJl\nYWwg\nZGVy"))

        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.encodePem("CERTIFICATE", der)
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.decodePem(pem, """{"allowedLabels":["PUBLIC KEY"]}""")
        }

        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.decodePemProto(pem, """{"allowedLabels":["PUBLIC KEY"]}""")
        }
        val pemErrorResult = codec.decodePemProtoResult(pem, """{"allowedLabels":["PUBLIC KEY"]}""")
        assertEquals(ReallyMeCodecProtoStatus.CODEC_ERROR, pemErrorResult.status)
        val pemError = CodecError.parseFrom(pemErrorResult.bytes)
        assertEquals(CodecError.ErrorCase.PEM, CodecError.parseFrom(pemErrorResult.bytes).errorCase)
        assertEquals(CodecError.ErrorCase.PEM, pemError.errorCase)
        assertEquals(
            CodecErrorReason.CODEC_ERROR_REASON_PEM_UNSUPPORTED_LABEL,
            pemError.pem.reason,
        )
    }

    private fun configuredCodec(): ReallyMeCodec {
        System.getProperty(TEST_LIBRARY_PROPERTY)
            ?.takeIf { it.isNotEmpty() }
            ?.let { ReallyMeCodecRustNativeProvider.loadLibrary(it) }
        return ReallyMeCodec
    }
}
