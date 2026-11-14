// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import java.nio.file.FileSystems
import java.nio.file.Files
import java.nio.file.LinkOption
import java.nio.file.attribute.GroupPrincipal
import java.nio.file.attribute.PosixFileAttributeView
import java.nio.file.attribute.PosixFilePermissions
import java.nio.file.attribute.UserPrincipal
import java.nio.file.attribute.UserPrincipalLookupService
import java.nio.file.attribute.UserPrincipalNotFoundException
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class NativeProviderSafetyTest {
    private companion object {
        private const val MAX_FFI_REQUEST_BYTES = 1_048_576
        private const val MAX_PROTOBUF_REQUEST_BYTES = 16_781_312
        private const val MAX_PROTO_JSON_REQUEST_BYTES = 22_373_720
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
        assertTrue(ReallyMeCodecRustNativeProvider.isCompatibleAbiVersion(6))
        assertFalse(ReallyMeCodecRustNativeProvider.isCompatibleAbiVersion(5))
        assertFalse(ReallyMeCodecRustNativeProvider.isCompatibleAbiVersion(0))

        assertTrue(ReallyMeCodecRustNativeProvider.isCompatiblePackageVersion(0, 4, 0))
        assertFalse(ReallyMeCodecRustNativeProvider.isCompatiblePackageVersion(0, 3, 6))
        assertFalse(ReallyMeCodecRustNativeProvider.isCompatiblePackageVersion(0, 4, 1))
        assertFalse(ReallyMeCodecRustNativeProvider.isCompatiblePackageVersion(0, 3, 5))
        assertFalse(ReallyMeCodecRustNativeProvider.isCompatiblePackageVersion(0, 2, 6))
        assertFalse(ReallyMeCodecRustNativeProvider.isCompatiblePackageVersion(1, 3, 6))
        assertFalse(ReallyMeCodecRustNativeProvider.isCompatiblePackageVersion(-1, -1, -1))

        // A NUL in the configured temporary root makes Path.of throw an
        // unchecked InvalidPathException; it must map to a typed refusal.
        assertNull(ReallyMeCodecRustNativeProvider.createPrivateExtractionDirectory("bad\u0000root"))

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
        val oversizedBase64 = "A".repeat(MAX_FFI_REQUEST_BYTES + 4)
        val oversizedJson = "\"" + "a".repeat(MAX_FFI_REQUEST_BYTES - 1) + "\""

        assertEquals(
            ReallyMeCodecException.InvalidInput::class,
            assertFailsWith<ReallyMeCodecException.InvalidInput> {
                ReallyMeCodec.base64Decode(oversizedBase64)
            }::class,
        )
        assertEquals(
            ReallyMeCodecException.InvalidInput::class,
            assertFailsWith<ReallyMeCodecException.InvalidInput> {
                ReallyMeCodec.canonicalizeJson(oversizedJson)
            }::class,
        )
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
        val expected =
            ReallyMeCodecRustNativeProvider.parseDigestMetadata(
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
        val current =
            java.nio.file.attribute
                .UserPrincipal { "DOMAIN\\codec" }
        val system =
            java.nio.file.attribute
                .UserPrincipal { "NT AUTHORITY\\SYSTEM" }
        val administrators =
            java.nio.file.attribute
                .UserPrincipal { "BUILTIN\\Administrators" }
        val spoofedCurrent =
            java.nio.file.attribute
                .UserPrincipal { "OTHER\\codec" }
        val spoofedSid =
            java.nio.file.attribute
                .UserPrincipal { "Administrators (S-1-5-32-544)" }
        assertTrue(ReallyMeCodecRustNativeProvider.isTrustedAclPrincipal(current, current, system, administrators))
        assertTrue(ReallyMeCodecRustNativeProvider.isTrustedAclPrincipal(system, current, system, administrators))
        assertTrue(ReallyMeCodecRustNativeProvider.isTrustedAclPrincipal(administrators, current, system, administrators))
        assertFalse(ReallyMeCodecRustNativeProvider.isTrustedAclPrincipal(spoofedCurrent, current, system, administrators))
        assertFalse(ReallyMeCodecRustNativeProvider.isTrustedAclPrincipal(spoofedSid, current, system, administrators))
    }

    @Test
    fun nativeExtractionRejectsUnsafePosixRootAndCreatesPrivateDirectory() {
        val root = Files.createTempDirectory("reallyme-codec-loader-test-")
        val posixView =
            Files.getFileAttributeView(
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
            extracted =
                ReallyMeCodecRustNativeProvider.createPrivateExtractionDirectory(
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
    fun windowsAclLookupUsesResolvedIdentityWhenSidTextIsUnavailable() {
        val system = UserPrincipal { "NT AUTHORITY\\SYSTEM" }
        val sidLookup =
            object : UserPrincipalLookupService() {
                override fun lookupPrincipalByName(name: String): UserPrincipal =
                    if (name == "S-1-5-18") {
                        system
                    } else {
                        throw UserPrincipalNotFoundException(name)
                    }

                override fun lookupPrincipalByGroupName(group: String): GroupPrincipal = throw UserPrincipalNotFoundException(group)
            }
        val lookup =
            object : UserPrincipalLookupService() {
                override fun lookupPrincipalByName(name: String): UserPrincipal =
                    if (name == "NT AUTHORITY\\SYSTEM") {
                        system
                    } else {
                        throw UserPrincipalNotFoundException(name)
                    }

                override fun lookupPrincipalByGroupName(group: String): GroupPrincipal = throw UserPrincipalNotFoundException(group)
            }
        assertEquals(
            system,
            ReallyMeCodecRustNativeProvider.lookupTrustedAclPrincipal(
                sidLookup,
                "S-1-5-18",
                "NT AUTHORITY\\SYSTEM",
            ),
        )
        assertEquals(
            system,
            ReallyMeCodecRustNativeProvider.lookupTrustedAclPrincipal(
                lookup,
                "S-1-5-18",
                "NT AUTHORITY\\SYSTEM",
            ),
        )
        assertFailsWith<UserPrincipalNotFoundException> {
            ReallyMeCodecRustNativeProvider.lookupTrustedAclPrincipal(
                lookup,
                "S-1-5-32-544",
                "BUILTIN\\Administrators",
            )
        }
    }

    @Test
    fun windowsHostCreatesPrivateNativeExtractionDirectory() {
        if (ReallyMeCodecRustNativeProvider.normalizedOs(System.getProperty("os.name")) != "windows") {
            return
        }
        val lookup = FileSystems.getDefault().userPrincipalLookupService
        assertNotNull(
            ReallyMeCodecRustNativeProvider.lookupTrustedAclPrincipal(
                lookup,
                "S-1-5-18",
                "NT AUTHORITY\\SYSTEM",
            ),
        )
        assertNotNull(
            ReallyMeCodecRustNativeProvider.lookupTrustedAclPrincipal(
                lookup,
                "S-1-5-32-544",
                "BUILTIN\\Administrators",
            ),
        )
        val extracted = assertNotNull(ReallyMeCodecRustNativeProvider.createPrivateExtractionDirectory())
        Files.delete(extracted)
    }

    private fun configuredCodec(): ReallyMeCodec {
        System
            .getProperty("reallyme.codec.testLibraryPath")
            ?.takeIf { it.isNotEmpty() }
            ?.let { ReallyMeCodecRustNativeProvider.loadLibrary(it) }
        return ReallyMeCodec
    }
}
