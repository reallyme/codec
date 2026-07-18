// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec

import com.google.protobuf.ByteString
import me.really.codec.v1.CodecDeterministicCborArray
import me.really.codec.v1.CodecDeterministicCborInteger
import me.really.codec.v1.CodecDeterministicCborMap
import me.really.codec.v1.CodecDeterministicCborMapEntry
import me.really.codec.v1.CodecDeterministicCborMapKey
import me.really.codec.v1.CodecDeterministicCborNull
import me.really.codec.v1.CodecDeterministicCborText
import me.really.codec.v1.CodecDeterministicCborUnsignedInteger
import me.really.codec.v1.CodecDeterministicCborValue
import me.really.codec.v1.CodecErrorReason
import me.really.codec.v1.CodecOperationResponse
import me.really.codec.v1.CodecPemDecodeResult
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
        private const val MAX_PROTOBUF_REQUEST_BYTES = 10_489_856
        private const val MAX_PROTO_JSON_REQUEST_BYTES = 16_082_264

        private fun pemLabel(label: String): ReallyMePemLabel =
            ReallyMePemLabel.entries.single { it.label == label }

        private val jsonStringPattern = Regex(
            """"([A-Za-z0-9]+)"\s*:\s*"((?:\\.|[^"\\])*)"""",
        )
        private val jsonNumberPattern = Regex(
            """"([A-Za-z0-9]+)"\s*:\s*([0-9]+)""",
        )

        private fun dagCborVectorValue(): ReallyMeDeterministicCborValue =
            ReallyMeDeterministicCborValue.Map(
                listOf(
                    ReallyMeDeterministicCborMapEntry(
                        key = ReallyMeDeterministicCborMapKey.Text("b"),
                        value = ReallyMeDeterministicCborValue.Integer(
                            ReallyMeDeterministicCborInteger.Unsigned(2u),
                        ),
                    ),
                    ReallyMeDeterministicCborMapEntry(
                        key = ReallyMeDeterministicCborMapKey.Text("a"),
                        value = ReallyMeDeterministicCborValue.Text("one"),
                    ),
                    ReallyMeDeterministicCborMapEntry(
                        key = ReallyMeDeterministicCborMapKey.Text("bytes"),
                        value = ReallyMeDeterministicCborValue.Bytes(
                            byteArrayOf(0, 1, 2),
                        ),
                    ),
                ),
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
    fun nativeAbiVersionAndLimitValidationFailsClosed() {
        assertTrue(ReallyMeCodecRustNativeProvider.isCompatibleAbiVersion(5))
        assertFalse(ReallyMeCodecRustNativeProvider.isCompatibleAbiVersion(4))
        assertFalse(ReallyMeCodecRustNativeProvider.isCompatibleAbiVersion(0))

        assertTrue(ReallyMeCodecRustNativeProvider.isValidNativeLimit(1))
        assertTrue(ReallyMeCodecRustNativeProvider.isValidNativeLimit(Int.MAX_VALUE.toLong()))
        assertFalse(ReallyMeCodecRustNativeProvider.isValidNativeLimit(0))
        assertFalse(ReallyMeCodecRustNativeProvider.isValidNativeLimit(-1))
        assertFalse(
            ReallyMeCodecRustNativeProvider.isValidNativeLimit(Int.MAX_VALUE.toLong() + 1),
        )
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
            ReallyMeCodec.multicodecPrefixForName(oversizedText)
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            ReallyMeCodec.canonicalizeJson("\uD800")
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
    fun deterministicAndDagCborBuildersPreserveCanonicalBytes() {
        val codec = configuredCodec()
        val vectors = CodecVectors.load()

        val deterministicValue = ReallyMeDeterministicCbor.mapText(
            linkedMapOf(
                "b" to ReallyMeDeterministicCbor.unsigned(2u),
                "a" to ReallyMeDeterministicCbor.unsigned(1u),
            ),
        )
        assertContentEquals(
            byteArrayOf(0xa2.toByte(), 0x61, 0x61, 0x01, 0x61, 0x62, 0x02),
            codec.deterministicCborEncode(deterministicValue),
        )

        val dagValue = ReallyMeDagCbor.mapText(
            linkedMapOf(
                "b" to ReallyMeDagCbor.unsignedLong(2),
                "a" to ReallyMeDagCbor.text("one"),
                "bytes" to ReallyMeDagCbor.bytes(byteArrayOf(0, 1, 2)),
            ),
        )
        val dagBytes = codec.dagCborEncode(dagValue)
        assertEquals(vectors.string("dagCborEncodedHex"), dagBytes.toLowerHex())
        assertEquals(vectors.string("dagCborCid"), codec.dagCborComputeCid(dagBytes))
        assertTrue(codec.dagCborVerifyCid(vectors.string("dagCborCid"), dagBytes).valid)
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

        val metadata = codec.multicodecPrefixForName(vectors.string("ed25519CodecName"))
        assertEquals(vectors.string("ed25519CodecName"), metadata.name)
        assertEquals(ReallyMeMulticodecTag.KEY, metadata.tag)
        assertEquals(vectors.string("ed25519AlgorithmName"), metadata.algorithmName)
        assertEquals(vectors.int("ed25519ExpectedKeyLength").toLong(), metadata.expectedKeyLength)
        assertEquals(vectors.string("ed25519PrefixHex"), metadata.prefix().toLowerHex())

        val lookup = codec.multicodecLookupPrefix(prefixedPublicKey)
        assertEquals(vectors.string("ed25519CodecName"), lookup.name)
        assertContentEquals(publicKey, codec.multicodecStripPrefix(prefixedPublicKey))
        assertTrue(codec.multicodecTable().entries.any { it.name == vectors.string("multicodecTableRequiredName") })

        assertEquals(
            vectors.string("ed25519Multikey"),
            codec.multikeyEncode(vectors.string("ed25519CodecName"), publicKey),
        )
        val parsed = codec.multikeyParse(vectors.string("ed25519Multikey"))
        assertEquals(vectors.string("ed25519CodecName"), parsed.codecName)
        assertEquals(vectors.string("ed25519AlgorithmName"), parsed.algorithmName)
        assertContentEquals(publicKey, parsed.publicKey())
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

        val dagCborBytes = codec.dagCborEncode(dagCborVectorValue())
        assertEquals(vectors.string("dagCborEncodedHex"), dagCborBytes.toLowerHex())
        assertContentEquals(dagCborBytes, codec.dagCborEncode(codec.dagCborDecode(dagCborBytes)))
        assertEquals(vectors.string("dagCborCid"), codec.dagCborComputeCid(dagCborBytes))
        assertTrue(codec.dagCborVerifyCid(vectors.string("dagCborCid"), dagCborBytes).valid)
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
            codec.encodePem(pemLabel(vectors.string("pemPrivateLabel")), privateDer),
        )
        val decodedPem = codec.decodePem(vectors.string("pemPrivatePem").toByteArray(Charsets.UTF_8))
        assertEquals(ReallyMePemLabel.PRIVATE_KEY, decodedPem.label)
        assertContentEquals(privateDer, decodedPem.der())
        assertContentEquals(
            vectors.string("pemWrappedPem").toByteArray(Charsets.UTF_8),
            codec.encodePem(
                pemLabel(vectors.string("pemPublicLabel")),
                vectors.string("pemWrappedDerText").toByteArray(Charsets.UTF_8),
                ReallyMePemEncodeOptions(lineWidth = 4),
            ),
        )

        val binaryResponse = codec.processOperation(vectors.hexBytes("protoMulticodecTableRequestHex"))
        val jsonResponse = codec.processOperationJson(
            vectors.string("protoMulticodecTableRequestJson").toByteArray(Charsets.UTF_8),
        )
        assertContentEquals(binaryResponse, jsonResponse)
        val decodedResponse = CodecOperationResponse.parseFrom(binaryResponse)
        assertEquals(CodecOperationResponse.OutcomeCase.RESULT, decodedResponse.outcomeCase)
        assertEquals(
            me.really.codec.v1.CodecOperationResult.ResultCase.MULTICODEC_TABLE,
            decodedResponse.result.resultCase,
        )
        assertTrue(
            decodedResponse.result.multicodecTable
                .entriesList
                .any { it.name == vectors.string("multicodecTableRequiredName") },
        )

        for (oversizedResponse in listOf(
            codec.processOperation(ByteArray(MAX_PROTOBUF_REQUEST_BYTES + 1)),
            codec.processOperationJson(ByteArray(MAX_PROTO_JSON_REQUEST_BYTES + 1)),
        )) {
            val response = CodecOperationResponse.parseFrom(oversizedResponse)
            assertEquals(CodecOperationResponse.OutcomeCase.ERROR, response.outcomeCase)
            assertEquals(
                CodecErrorReason.CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
                response.error.boundary.reason,
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
            vectors.string("base64Whitespace"),
        )) {
            assertFailsWith<ReallyMeCodecException.InvalidInput> {
                codec.base64Decode(value)
            }
        }
        for (value in listOf(
            vectors.string("base64urlPadded"),
            vectors.string("base64urlNonCanonicalTrailingBits"),
            vectors.string("base64urlInvalidLength"),
            vectors.string("base64urlWhitespace"),
        )) {
            assertFailsWith<ReallyMeCodecException.InvalidInput> {
                codec.base64urlDecode(value)
            }
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.multibaseDecode(vectors.string("unsupportedMultibase"))
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.multibaseDecode(vectors.string("multibaseMultibytePrefix"))
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
        assertEquals(
            vectors.string("jcsUtf16KeyOrderCanonicalJson"),
            codec.canonicalizeJson(vectors.string("jcsUtf16KeyOrderInputJson")),
        )
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

        val metadata = codec.multicodecPrefixForName("ed25519-pub")
        assertEquals("ed25519-pub", metadata.name)
        assertEquals(ReallyMeMulticodecTag.KEY, metadata.tag)
        assertEquals("Ed25519", metadata.algorithmName)
        assertEquals(32L, metadata.expectedKeyLength)
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.multicodecPrefixForName("not-a-codec")
        }

        val prefixed = metadata.prefix() + publicKey
        val lookup = codec.multicodecLookupPrefix(prefixed)
        assertEquals("ed25519-pub", lookup.name)
        assertEquals(metadata.prefix().size.toLong(), lookup.prefixLength)
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.multicodecLookupPrefix(byteArrayOf(0, 0, 7))
        }
        assertContentEquals(publicKey, codec.multicodecStripPrefix(prefixed))
        assertTrue(codec.multicodecTable().entries.any { it.name == "mlkem-1024-pub" })

        val multikey = codec.multikeyEncode("ed25519-pub", publicKey)
        val parsed = codec.multikeyParse(multikey)
        assertEquals("ed25519-pub", parsed.codecName)
        assertEquals("Ed25519", parsed.algorithmName)
        assertContentEquals(publicKey, parsed.publicKey())
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
            codec.multikeyParse("not-a-key")
        }
    }

    @Test
    fun dagCborCidAndJcsOperationsUseRustProvider() {
        val codec = configuredCodec()
        val encoded = codec.dagCborEncode(dagCborVectorValue())
        assertTrue(encoded.isNotEmpty())
        assertContentEquals(encoded, codec.dagCborEncode(codec.dagCborDecode(encoded)))

        val cid = codec.dagCborComputeCid(encoded)
        assertTrue(codec.isValidCidString(cid))
        assertFalse(codec.isValidCidString("not-a-cid"))
        assertEquals(cid, codec.tryParseCid(cid))
        assertNull(codec.tryParseCid("not-a-cid"))

        assertTrue(codec.dagCborVerifyCid(cid, encoded).valid)

        val invalidUpperPayloadCid = cid.take(1) + cid.drop(1).uppercase()
        val invalidVerification = codec.dagCborVerifyCid(invalidUpperPayloadCid, encoded)
        assertFalse(invalidVerification.valid)
        assertEquals("", invalidVerification.actualCid)
        val emptyCidResult = codec.dagCborVerifyCid("", encoded)
        assertFalse(emptyCidResult.valid)
        assertEquals(cid, emptyCidResult.expectedCid)

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
    fun deterministicCborTypedSurfaceUsesGeneratedProto() {
        val codec = configuredCodec()
        val value = ReallyMeDeterministicCborValue.Map(
            listOf(
                ReallyMeDeterministicCborMapEntry(
                    key = ReallyMeDeterministicCborMapKey.Text("b"),
                    value = ReallyMeDeterministicCborValue.Integer(
                        ReallyMeDeterministicCborInteger.Unsigned(2u),
                    ),
                ),
                ReallyMeDeterministicCborMapEntry(
                    key = ReallyMeDeterministicCborMapKey.Integer(
                        ReallyMeDeterministicCborInteger.Unsigned(1u),
                    ),
                    value = ReallyMeDeterministicCborValue.Text("i"),
                ),
                ReallyMeDeterministicCborMapEntry(
                    key = ReallyMeDeterministicCborMapKey.Text("1"),
                    value = ReallyMeDeterministicCborValue.Text("t"),
                ),
            ),
        )

        val encoded = codec.deterministicCborEncode(value)
        assertEquals("a301616961316174616202", encoded.toLowerHex())
        val decoded = codec.deterministicCborDecode(encoded)
        assertContentEquals(encoded, codec.deterministicCborEncode(decoded))

        var maximumDepth: ReallyMeDeterministicCborValue = ReallyMeDeterministicCborValue.Null
        repeat(64) {
            maximumDepth = ReallyMeDeterministicCborValue.Map(
                listOf(
                    ReallyMeDeterministicCborMapEntry(
                        key = ReallyMeDeterministicCborMapKey.Integer(
                            ReallyMeDeterministicCborInteger.Unsigned(1u),
                        ),
                        value = maximumDepth,
                    ),
                ),
            )
        }
        val maximumDepthEncoded = codec.deterministicCborEncode(maximumDepth)
        val maximumDepthDecoded = codec.deterministicCborDecode(maximumDepthEncoded)
        assertContentEquals(
            maximumDepthEncoded,
            codec.deterministicCborEncode(maximumDepthDecoded),
        )
        assertEquals("ReallyMeDeterministicCborValue(<redacted>)", value.toString())
        assertEquals(
            "ReallyMeDeterministicCborMapKey(<redacted>)",
            ReallyMeDeterministicCborMapKey.Text("passportNumber").toString(),
        )
        val mutableValues = mutableListOf<ReallyMeDeterministicCborValue>(
            ReallyMeDeterministicCborValue.Null,
        )
        val snapshot = ReallyMeDeterministicCborValue.Array(mutableValues)
        mutableValues.clear()
        assertEquals(1, snapshot.values.size)

        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            ReallyMeDeterministicCborInteger.Negative.of(0)
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.deterministicCborDecode(byteArrayOf(0x18, 0x00))
        }
    }

    @Test
    fun deterministicCborProviderTreeIsValidatedBeforeSdkCopy() {
        val nullValue = CodecDeterministicCborValue.newBuilder()
            .setNullValue(CodecDeterministicCborNull.getDefaultInstance())
            .build()
        assertFalse(nullValue.reallyMeHasUnknownFieldsForValidation())
        val oversizedValue = CodecDeterministicCborValue.newBuilder()
            .setArrayValue(
                CodecDeterministicCborArray.newBuilder()
                    .addAllValues(List(16_385) { nullValue })
            )
            .build()

        assertFailsWith<ReallyMeCodecException.ProviderFailure> {
            ReallyMeCodec.validateProviderDeterministicCborValue(oversizedValue)
        }

        val unknownFieldValue = CodecDeterministicCborValue.parseFrom(
            byteArrayOf(0x0a, 0x00, 0x98.toByte(), 0x06, 0x01),
        )
        assertTrue(unknownFieldValue.reallyMeHasUnknownFieldsForValidation())
        assertFailsWith<ReallyMeCodecException.ProviderFailure> {
            ReallyMeCodec.validateProviderDeterministicCborValue(unknownFieldValue)
        }

        fun textKey(value: String): CodecDeterministicCborMapKey =
            CodecDeterministicCborMapKey.newBuilder()
                .setTextKey(CodecDeterministicCborText.newBuilder().setValue(value))
                .build()

        fun unsignedKey(value: ULong): CodecDeterministicCborMapKey =
            CodecDeterministicCborMapKey.newBuilder()
                .setIntegerKey(
                    CodecDeterministicCborInteger.newBuilder()
                        .setUnsignedValue(
                            CodecDeterministicCborUnsignedInteger.newBuilder()
                                .setValue(value.toLong())
                        )
                )
                .build()

        fun entry(key: CodecDeterministicCborMapKey): CodecDeterministicCborMapEntry =
            CodecDeterministicCborMapEntry.newBuilder()
                .setKey(key)
                .setValue(nullValue)
                .build()

        val duplicateMap = CodecDeterministicCborValue.newBuilder()
            .setMapValue(
                CodecDeterministicCborMap.newBuilder()
                    .addEntries(entry(textKey("a")))
                    .addEntries(entry(textKey("a")))
            )
            .build()
        assertFailsWith<ReallyMeCodecException.ProviderFailure> {
            ReallyMeCodec.validateProviderDeterministicCborValue(duplicateMap)
        }

        val duplicateUnsignedMap = CodecDeterministicCborValue.newBuilder()
            .setMapValue(
                CodecDeterministicCborMap.newBuilder()
                    .addEntries(entry(unsignedKey(ULong.MAX_VALUE)))
                    .addEntries(entry(unsignedKey(ULong.MAX_VALUE)))
            )
            .build()
        assertFailsWith<ReallyMeCodecException.ProviderFailure> {
            ReallyMeCodec.validateProviderDeterministicCborValue(duplicateUnsignedMap)
        }

        val exactUtf8Map = CodecDeterministicCborValue.newBuilder()
            .setMapValue(
                CodecDeterministicCborMap.newBuilder()
                    .addEntries(entry(textKey("\u00e9")))
                    .addEntries(entry(textKey("e\u0301")))
            )
            .build()
        ReallyMeCodec.validateProviderDeterministicCborValue(exactUtf8Map)
    }

    @Test
    fun generatedProtobufSensitiveFormattingAndHashingAreRedacted() {
        val text = CodecDeterministicCborText.newBuilder()
            .setValue("passport-number")
            .build()
        val pem = CodecPemDecodeResult.newBuilder()
            .setLabel("PRIVATE KEY")
            .setDer(ByteString.copyFrom(byteArrayOf(0x30, 0x03, 0x02, 0x01, 0x01)))
            .build()
        val otherPem = pem.toBuilder()
            .setDer(ByteString.copyFrom(byteArrayOf(0x30, 0x03, 0x02, 0x01, 0x02)))
            .build()

        assertEquals("CodecDeterministicCborText{<redacted>}", text.toString())
        assertEquals("CodecPemDecodeResult{<redacted>}", pem.toString())
        assertEquals(0x524d, pem.hashCode())
        assertEquals(pem.hashCode(), otherPem.hashCode())
    }

    @Test
    fun pemRoundTripAndProtoErrorsUseRustProvider() {
        val codec = configuredCodec()
        val der = byteArrayOf(0x30, 0x03, 0x02, 0x01, 0x01)
        val pem = codec.encodePem(ReallyMePemLabel.PRIVATE_KEY, der)

        assertTrue(pem.toString(Charsets.UTF_8).contains("-----BEGIN PRIVATE KEY-----"))
        val decoded = codec.decodePem(pem)
        assertEquals(ReallyMePemLabel.PRIVATE_KEY, decoded.label)
        assertContentEquals(der, decoded.der())
        decoded.close()
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            decoded.der()
        }

        val wrapped = codec.encodePem(
            ReallyMePemLabel.PUBLIC_KEY,
            "not real der".toByteArray(Charsets.UTF_8),
            ReallyMePemEncodeOptions(lineWidth = 4),
        )
        assertTrue(wrapped.toString(Charsets.UTF_8).contains("bm90\nIHJl\nYWwg\nZGVy"))

        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.encodePem(
                ReallyMePemLabel.PUBLIC_KEY,
                der,
                ReallyMePemEncodeOptions(lineWidth = 77),
            )
        }
        assertFailsWith<ReallyMeCodecException.InvalidInput> {
            codec.decodePem(
                pem,
                ReallyMePemDecodeOptions(allowedLabels = listOf(ReallyMePemLabel.PUBLIC_KEY)),
            )
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
                    File(root, "vectors/codec-vectors.json"),
                    File(root, "../../vectors/codec-vectors.json"),
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
