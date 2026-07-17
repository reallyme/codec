// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec

import me.really.codec.v1.CodecDagCborVerifyCidResult
import me.really.codec.v1.CodecBackendError
import me.really.codec.v1.CodecBoundaryError
import me.really.codec.v1.CodecCanonicalizationError
import me.really.codec.v1.CodecError
import me.really.codec.v1.CodecErrorReason
import me.really.codec.v1.CodecMulticodecLookupResult
import me.really.codec.v1.CodecMulticodecSpec
import me.really.codec.v1.CodecMulticodecTableResult
import me.really.codec.v1.CodecMultikeyParseResult
import me.really.codec.v1.CodecProtoResultEnvelope
import me.really.codec.v1.CodecProtoResultStatus
import java.io.File
import java.nio.file.Files
import java.nio.file.LinkOption
import java.nio.file.attribute.PosixFileAttributeView
import java.nio.file.attribute.PosixFilePermissions
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
        private const val MAX_FFI_REQUEST_BYTES = 1_048_576
        private const val MAX_PROTOBUF_REQUEST_BYTES = 1_048_576
        private const val MAX_PROTO_JSON_REQUEST_BYTES = 1_572_864

        private val jsonStringPattern = Regex(
            """"([A-Za-z0-9]+)"\s*:\s*"((?:\\.|[^"\\])*)"""",
        )
        private val jsonNumberPattern = Regex(
            """"([A-Za-z0-9]+)"\s*:\s*([0-9]+)""",
        )
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
    fun managedBoundariesRejectOversizedInputsBeforeSerialization() {
        val oversizedText = "a".repeat(MAX_FFI_REQUEST_BYTES + 1)

        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            ReallyMeCodec.base64Decode(oversizedText)
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            ReallyMeCodec.canonicalizeJson(oversizedText)
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            ReallyMeCodec.multicodecPrefixForNameProto(oversizedText)
        }

        val result = ReallyMeCodec.multicodecPrefixForNameProtoResult(oversizedText)
        assertEquals(ReallyMeCodecProtoStatus.CODEC_ERROR, result.status)
        assertEquals(
            CodecErrorReason.CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
            CodecError.parseFrom(result.bytes).boundary.reason,
        )
    }

    @Test
    fun throwingProtoApisPreserveCallerVersusProviderAttribution() {
        val backend = CodecError.newBuilder()
            .setBackend(
                CodecBackendError.newBuilder()
                    .setReason(CodecErrorReason.CODEC_ERROR_REASON_BACKEND_INTERNAL),
            )
            .build()
        assertTrue(
            ReallyMeCodec.exceptionForCodecErrorPayload(backend.toByteArray()) is
                ReallyMeCodecException.ProviderFailure,
        )

        val internal = CodecError.newBuilder()
            .setCanonicalization(
                CodecCanonicalizationError.newBuilder()
                    .setReason(CodecErrorReason.CODEC_ERROR_REASON_CANONICAL_INTERNAL),
            )
            .build()
        assertTrue(
            ReallyMeCodec.exceptionForCodecErrorPayload(internal.toByteArray()) is
                ReallyMeCodecException.ProviderFailure,
        )

        val malformedBoundary = CodecError.newBuilder()
            .setBoundary(
                CodecBoundaryError.newBuilder()
                    .setReason(CodecErrorReason.CODEC_ERROR_REASON_BOUNDARY_MALFORMED_PROTOBUF),
            )
            .build()
        assertTrue(
            ReallyMeCodec.exceptionForCodecErrorPayload(malformedBoundary.toByteArray()) is
                ReallyMeCodecException.ProviderFailure,
        )

        val resourceBoundary = CodecError.newBuilder()
            .setBoundary(
                CodecBoundaryError.newBuilder()
                    .setReason(CodecErrorReason.CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED),
            )
            .build()
        assertTrue(
            ReallyMeCodec.exceptionForCodecErrorPayload(resourceBoundary.toByteArray()) is
                ReallyMeCodecException.InvalidInput,
        )

        val mismatched = CodecError.newBuilder()
            .setBackend(
                CodecBackendError.newBuilder()
                    .setReason(CodecErrorReason.CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTIKEY),
            )
            .build()
        assertTrue(
            ReallyMeCodec.exceptionForCodecErrorPayload(mismatched.toByteArray()) is
                ReallyMeCodecException.ProviderFailure,
        )
        val unknownReason = CodecError.newBuilder()
            .setCanonicalization(
                CodecCanonicalizationError.newBuilder().setReasonValue(450),
            )
            .build()
        assertTrue(
            ReallyMeCodec.exceptionForCodecErrorPayload(unknownReason.toByteArray()) is
                ReallyMeCodecException.ProviderFailure,
        )
        assertTrue(
            ReallyMeCodec.exceptionForCodecErrorPayload(byteArrayOf(0xff.toByte())) is
                ReallyMeCodecException.ProviderFailure,
        )
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
                digestPath = "/me/really/codec/native/linux-aarch64/libreallyme_codec_ffi.so.sha256",
            ),
            ReallyMeCodecRustNativeProvider.platformNativeResource(
                osName = "Linux",
                osArch = "aarch64",
                androidRuntime = false,
            ),
        )
    }

    @Test
    fun nativeDigestMetadataAndTempPermissionsFailClosed() {
        val digestHex = "ab".repeat(32)
        val expected = ReallyMeCodecRustNativeProvider.parseDigestMetadata(
            "$digestHex 4096\n".toByteArray(Charsets.US_ASCII),
        )
        assertEquals(4096L, expected?.size)
        assertContentEquals(ByteArray(32) { 0xab.toByte() }, expected?.sha256)

        assertNull(
            ReallyMeCodecRustNativeProvider.parseDigestMetadata(
                "${digestHex.uppercase()} 4096\n".toByteArray(Charsets.US_ASCII),
            ),
        )
        assertNull(
            ReallyMeCodecRustNativeProvider.parseDigestMetadata(
                "$digestHex 0\n".toByteArray(Charsets.US_ASCII),
            ),
        )
        assertNull(
            ReallyMeCodecRustNativeProvider.parseDigestMetadata(
                "$digestHex 134217729\n".toByteArray(Charsets.US_ASCII),
            ),
        )

        assertTrue(ReallyMeCodecRustNativeProvider.isSecurePosixTempMode(0x1ff or 0x200))
        assertTrue(ReallyMeCodecRustNativeProvider.isSecurePosixTempMode(0x1c0))
        assertFalse(ReallyMeCodecRustNativeProvider.isSecurePosixTempMode(0x1ff))
        assertTrue(ReallyMeCodecRustNativeProvider.isTrustedPosixTempOwner("root", "codec"))
        assertTrue(ReallyMeCodecRustNativeProvider.isTrustedPosixTempOwner("codec", "codec"))
        assertFalse(ReallyMeCodecRustNativeProvider.isTrustedPosixTempOwner("attacker", "codec"))
        assertTrue(
            ReallyMeCodecRustNativeProvider.isTrustedAclPrincipal(
                "DESKTOP\\codec",
                "codec",
            ),
        )
        assertTrue(
            ReallyMeCodecRustNativeProvider.isTrustedAclPrincipal(
                "NT AUTHORITY\\SYSTEM",
                "codec",
            ),
        )
        assertTrue(
            ReallyMeCodecRustNativeProvider.isTrustedAclPrincipal(
                "Administratoren",
                "codec",
                "Administratoren (S-1-5-32-544)",
            ),
        )
        assertFalse(
            ReallyMeCodecRustNativeProvider.isTrustedAclPrincipal(
                "DESKTOP\\attacker",
                "codec",
            ),
        )
    }

    @Test
    fun nativeExtractionRejectsUnsafePosixRootAndCreatesPrivateDirectory() {
        val root = Files.createTempDirectory("reallyme-codec-loader-test-")
        val posixView = Files.getFileAttributeView(
            root,
            PosixFileAttributeView::class.java,
            LinkOption.NOFOLLOW_LINKS,
        )
        if (posixView == null) {
            Files.deleteIfExists(root)
            return
        }

        var extracted: java.nio.file.Path? = null
        try {
            Files.setPosixFilePermissions(root, PosixFilePermissions.fromString("rwxrwxrwx"))
            assertNull(
                ReallyMeCodecRustNativeProvider.createPrivateExtractionDirectory(root.toString()),
            )

            Files.setPosixFilePermissions(root, PosixFilePermissions.fromString("rwx------"))
            extracted = ReallyMeCodecRustNativeProvider.createPrivateExtractionDirectory(
                root.toString(),
            )
            assertEquals(
                PosixFilePermissions.fromString("rwx------"),
                extracted?.let { Files.getPosixFilePermissions(it, LinkOption.NOFOLLOW_LINKS) },
            )
        } finally {
            extracted?.let { Files.deleteIfExists(it) }
            Files.setPosixFilePermissions(root, PosixFilePermissions.fromString("rwx------"))
            Files.deleteIfExists(root)
        }
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
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.base64Encode(ByteArray(MAX_FFI_REQUEST_BYTES + 1))
        }
    }

    @Test
    fun sharedVectorSuiteCoversKotlinPublicMethods() {
        val codec = configuredCodec()
        val vectors = CodecVectors.load()
        val baseInput = vectors.hexBytes("baseInputHex")

        assertEquals(vectors.string("base64Padded"), codec.base64Encode(baseInput))
        assertContentEquals(baseInput, codec.base64Decode(vectors.string("base64Padded")))
        assertEquals(vectors.string("base64urlUnpadded"), codec.base64urlEncode(baseInput))
        assertContentEquals(baseInput, codec.base64urlDecode(vectors.string("base64urlUnpadded")))
        assertEquals(vectors.string("lowerHex"), codec.bytesToLowerHex(baseInput))
        assertContentEquals(baseInput, codec.lowerHexToBytes(vectors.string("lowerHex")))
        assertEquals(vectors.string("base58btcEncoded"), codec.base58btcEncode(baseInput))
        assertContentEquals(baseInput, codec.base58btcDecode(vectors.string("base58btcEncoded")))

        val publicKey = vectors.hexBytes("publicKeyHex")
        val prefixedPublicKey = vectors.hexBytes("ed25519PrefixedPublicKeyHex")
        assertEquals(vectors.string("publicKeyBase58btc"), codec.base58btcEncode(publicKey))
        assertEquals(vectors.string("publicKeyMultibaseBase58btc"), codec.multibaseBase58btcEncode(publicKey))
        assertEquals(vectors.string("publicKeyMultibaseBase64url"), codec.multibaseBase64urlEncode(publicKey))
        assertContentEquals(publicKey, codec.multibaseDecode(vectors.string("publicKeyMultibaseBase58btc")))
        assertContentEquals(publicKey, codec.multibaseDecode(vectors.string("publicKeyMultibaseBase64url")))

        val metadataJson = codec.multicodecPrefixForName(vectors.string("ed25519CodecName"))
        assertTrue(metadataJson.contains("\"name\":\"${vectors.string("ed25519CodecName")}\""))
        assertTrue(metadataJson.contains("\"tag\":\"${vectors.string("ed25519Tag")}\""))
        val metadataProto = CodecMulticodecSpec.parseFrom(
            codec.multicodecPrefixForNameProto(vectors.string("ed25519CodecName")),
        )
        assertEquals(vectors.string("ed25519CodecName"), metadataProto.name)
        assertEquals(vectors.string("ed25519AlgorithmName"), metadataProto.algorithmName)
        assertEquals(vectors.int("ed25519ExpectedKeyLength"), metadataProto.fixedLength)
        assertEquals(vectors.string("ed25519PrefixHex"), metadataProto.prefix.toByteArray().toLowerHex())
        assertEquals(
            ReallyMeCodecProtoStatus.RESULT,
            codec.multicodecPrefixForNameProtoResult(vectors.string("ed25519CodecName")).status,
        )

        assertTrue(codec.multicodecLookupPrefix(prefixedPublicKey).contains("\"name\":\"${vectors.string("ed25519CodecName")}\""))
        val lookupProto = CodecMulticodecLookupResult.parseFrom(
            codec.multicodecLookupPrefixProto(prefixedPublicKey),
        )
        assertEquals(vectors.string("ed25519CodecName"), lookupProto.name)
        assertEquals(ReallyMeCodecProtoStatus.RESULT, codec.multicodecLookupPrefixProtoResult(prefixedPublicKey).status)
        assertContentEquals(publicKey, codec.multicodecStripPrefix(prefixedPublicKey))
        assertTrue(codec.multicodecTable().contains(vectors.string("multicodecTableRequiredName")))
        assertTrue(
            CodecMulticodecTableResult.parseFrom(codec.multicodecTableProto())
                .entriesList
                .any { it.name == vectors.string("multicodecTableRequiredName") },
        )
        assertEquals(ReallyMeCodecProtoStatus.RESULT, codec.multicodecTableProtoResult().status)

        assertEquals(
            vectors.string("ed25519Multikey"),
            codec.multikeyEncode(vectors.string("ed25519CodecName"), publicKey),
        )
        assertTrue(codec.multikeyParse(vectors.string("ed25519Multikey")).contains("\"codecName\":\"${vectors.string("ed25519CodecName")}\""))
        val parsedProto = CodecMultikeyParseResult.parseFrom(
            codec.multikeyParseProto(vectors.string("ed25519Multikey")),
        )
        assertEquals(vectors.string("ed25519CodecName"), parsedProto.codecName)
        assertEquals(vectors.string("ed25519AlgorithmName"), parsedProto.algorithmName)
        assertContentEquals(publicKey, parsedProto.publicKey.toByteArray())
        assertEquals(ReallyMeCodecProtoStatus.RESULT, codec.multikeyParseProtoResult(vectors.string("ed25519Multikey")).status)
        assertTrue(codec.bindingTypeMatchesCodec(vectors.string("multikeyBindingType"), vectors.string("ed25519CodecName")))
        codec.requireSupportedMulticodec(vectors.string("ed25519CodecName"))
        codec.validateKeyBinding(vectors.string("multikeyBindingType"), null, vectors.string("ed25519Multikey"))
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.validateKeyBinding(
                vectors.string("mismatchedBindingType"),
                vectors.string("mismatchedBindingAlgorithm"),
                vectors.string("ed25519Multikey"),
            )
        }

        val dagCborBytes = codec.dagCborEncode(vectors.string("dagCborTaggedJson"))
        assertEquals(vectors.string("dagCborEncodedHex"), dagCborBytes.toLowerHex())
        assertEquals(vectors.string("dagCborCanonicalTaggedJson"), codec.dagCborDecode(dagCborBytes))
        assertEquals(vectors.string("dagCborCid"), codec.dagCborComputeCid(dagCborBytes))
        assertTrue(codec.dagCborVerifyCid(vectors.string("dagCborCid"), dagCborBytes).contains("\"valid\":true"))
        assertTrue(
            CodecDagCborVerifyCidResult.parseFrom(
                codec.dagCborVerifyCidProto(vectors.string("dagCborCid"), dagCborBytes),
            ).valid,
        )
        assertEquals(
            ReallyMeCodecProtoStatus.RESULT,
            codec.dagCborVerifyCidProtoResult(vectors.string("dagCborCid"), dagCborBytes).status,
        )
        assertEquals(vectors.string("dagCborSha256Hex"), codec.dagCborSha256ContentHash(dagCborBytes).toLowerHex())
        assertEquals(vectors.string("dagCborMultihashHex"), codec.dagCborMultihash(dagCborBytes).toLowerHex())
        assertEquals(vectors.int("dagCborCodecCode"), codec.dagCborCodecCode())
        assertTrue(codec.isValidCidString(vectors.string("dagCborCid")))
        assertFalse(codec.isValidCidString(vectors.string("invalidCid")))
        assertEquals(vectors.string("dagCborCid"), codec.tryParseCid(vectors.string("dagCborCid")))
        assertNull(codec.tryParseCid(vectors.string("invalidCid")))

        assertEquals(vectors.string("jcsObjectCanonicalJson"), codec.canonicalizeJson(vectors.string("jcsObjectInputJson")))
        assertEquals(vectors.string("jcsNumberCanonicalJson"), codec.canonicalizeJson(vectors.string("jcsNumberInputJson")))

        val privateDer = vectors.hexBytes("pemPrivateDerHex")
        assertContentEquals(
            vectors.string("pemPrivatePem").toByteArray(Charsets.UTF_8),
            codec.encodePem(vectors.string("pemPrivateLabel"), privateDer),
        )
        assertTrue(
            codec.decodePem(vectors.string("pemPrivatePem").toByteArray(Charsets.UTF_8))
                .toString(Charsets.UTF_8)
                .contains("\"label\":\"${vectors.string("pemPrivateLabel")}\"")
        )
        assertContentEquals(
            vectors.string("pemWrappedPem").toByteArray(Charsets.UTF_8),
            codec.encodePem(
                vectors.string("pemPublicLabel"),
                vectors.string("pemWrappedDerText").toByteArray(Charsets.UTF_8),
                vectors.string("pemLineWidthOptionsJson"),
            ),
        )

        val binaryEnvelope = codec.processProto(vectors.hexBytes("protoMulticodecTableRequestHex"))
        val jsonEnvelope = codec.processProtoJson(
            vectors.string("protoMulticodecTableRequestJson").toByteArray(Charsets.UTF_8),
        )
        assertContentEquals(binaryEnvelope, jsonEnvelope)
        val decodedEnvelope = CodecProtoResultEnvelope.parseFrom(binaryEnvelope)
        assertEquals(
            CodecProtoResultStatus.CODEC_PROTO_RESULT_STATUS_RESULT,
            decodedEnvelope.status,
        )
        assertTrue(
            CodecMulticodecTableResult.parseFrom(decodedEnvelope.payload)
                .entriesList
                .any { it.name == vectors.string("multicodecTableRequiredName") },
        )

        for (oversizedEnvelope in listOf(
            codec.processProto(ByteArray(MAX_PROTOBUF_REQUEST_BYTES + 1)),
            codec.processProtoJson(ByteArray(MAX_PROTO_JSON_REQUEST_BYTES + 1)),
        )) {
            val envelope = CodecProtoResultEnvelope.parseFrom(oversizedEnvelope)
            assertEquals(
                CodecProtoResultStatus.CODEC_PROTO_RESULT_STATUS_CODEC_ERROR,
                envelope.status,
            )
            assertEquals(
                CodecErrorReason.CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
                CodecError.parseFrom(envelope.payload).boundary.reason,
            )
        }
    }

    @Test
    fun sharedVectorSuiteRejectsNonCanonicalInputs() {
        val codec = configuredCodec()
        val vectors = CodecVectors.load()

        for (value in listOf(
            vectors.string("base64MissingPadding"),
            vectors.string("base64NonCanonicalTrailingBits"),
        )) {
            assertFailsWith<ReallyMeCodecException.InvalidInput> {
                codec.base64Decode(value)
            }
        }
        for (value in listOf(
            vectors.string("base64urlPadded"),
            vectors.string("base64urlNonCanonicalTrailingBits"),
        )) {
            assertFailsWith<ReallyMeCodecException.InvalidInput> {
                codec.base64urlDecode(value)
            }
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.multibaseDecode(vectors.string("unsupportedMultibase"))
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.multikeyParse(vectors.string("nonCanonicalBase64urlMultikey"))
        }
        for (key in listOf(
            "dagCborNonCanonicalIntegerHex",
            "dagCborDuplicateKeyHex",
            "dagCborOutOfOrderKeyHex",
        )) {
            assertFailsWith<ReallyMeCodecException.InvalidInput> {
                codec.dagCborDecode(vectors.hexBytes(key))
            }
        }
        for (key in listOf(
            "jcsDuplicateMemberJson",
            "jcsNonInteroperableIntegerJson",
            "jcsLoneSurrogateJson",
        )) {
            assertFailsWith<ReallyMeCodecException.InvalidInput> {
                codec.canonicalizeJson(vectors.string(key))
            }
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

        assertTrue(pem.toString(Charsets.UTF_8).contains("-----BEGIN PRIVATE KEY-----"))
        val decodedJson = codec.decodePem(pem)
        assertTrue(decodedJson.toString(Charsets.UTF_8).contains("\"label\":\"PRIVATE KEY\""))
        assertTrue(decodedJson.toString(Charsets.UTF_8).contains("\"der\":\"MAMCAQE\""))

        val wrapped = codec.encodePem(
            "PUBLIC KEY",
            "not real der".toByteArray(Charsets.UTF_8),
            """{"lineWidth":4}""",
        )
        assertTrue(wrapped.toString(Charsets.UTF_8).contains("bm90\nIHJl\nYWwg\nZGVy"))

        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.encodePem("CERTIFICATE", der)
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.decodePem(pem, """{"allowedLabels":["PUBLIC KEY"]}""")
        }

    }

    private fun configuredCodec(): ReallyMeCodec {
        System.getProperty(TEST_LIBRARY_PROPERTY)
            ?.takeIf { it.isNotEmpty() }
            ?.let { ReallyMeCodecRustNativeProvider.loadLibrary(it) }
        return ReallyMeCodec
    }

    private class CodecVectors(
        private val strings: Map<String, String>,
        private val numbers: Map<String, Int>,
    ) {
        fun string(key: String): String =
            strings[key] ?: error("missing codec vector string: $key")

        fun int(key: String): Int =
            numbers[key] ?: error("missing codec vector number: $key")

        fun hexBytes(key: String): ByteArray = string(key).hexToBytes()

        companion object {
            fun load(): CodecVectors {
                val root = File(System.getProperty("user.dir"))
                val candidates = listOf(
                    File(root, "test-vectors/codec-vectors.json"),
                    File(root, "../../test-vectors/codec-vectors.json"),
                )
                val file = candidates.firstOrNull { it.isFile }
                    ?: error("missing codec vector manifest")
                val text = file.readText(Charsets.UTF_8)
                val strings = jsonStringPattern.findAll(text).associate { match ->
                    match.groupValues[1] to match.groupValues[2].jsonUnescaped()
                }
                val numbers = jsonNumberPattern.findAll(text).associate { match ->
                    match.groupValues[1] to match.groupValues[2].toInt()
                }
                require(numbers["schemaVersion"] == 2) {
                    "unsupported codec vector manifest schema"
                }
                return CodecVectors(strings, numbers)
            }
        }
    }
}

private fun String.hexToBytes(): ByteArray {
    require(length % 2 == 0)
    return ByteArray(length / 2) { index ->
        substring(index * 2, index * 2 + 2).toInt(16).toByte()
    }
}

private fun ByteArray.toLowerHex(): String =
    joinToString(separator = "") { byte -> "%02x".format(byte.toInt() and 0xff) }

private fun String.jsonUnescaped(): String {
    val output = StringBuilder(length)
    var index = 0
    while (index < length) {
        val ch = this[index]
        if (ch != '\\') {
            output.append(ch)
            index += 1
            continue
        }
        index += 1
        require(index < length)
        when (val escaped = this[index]) {
            '"' -> output.append('"')
            '\\' -> output.append('\\')
            '/' -> output.append('/')
            'b' -> output.append('\b')
            'f' -> output.append('\u000C')
            'n' -> output.append('\n')
            'r' -> output.append('\r')
            't' -> output.append('\t')
            'u' -> {
                require(index + 4 < length)
                val codePoint = substring(index + 1, index + 5).toInt(16)
                output.append(codePoint.toChar())
                index += 4
            }
            else -> error("unsupported JSON escape: $escaped")
        }
        index += 1
    }
    return output.toString()
}
