// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import me.really.codec.ReallyMeCodecRustNativeProvider.ExpectedNativeDigest
import me.really.codec.ReallyMeCodecRustNativeProvider.NativeResource
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.io.InputStream
import java.nio.charset.StandardCharsets
import java.nio.file.FileSystems
import java.nio.file.Files
import java.nio.file.LinkOption
import java.nio.file.Path
import java.nio.file.StandardOpenOption
import java.nio.file.attribute.AclEntry
import java.nio.file.attribute.AclEntryPermission
import java.nio.file.attribute.AclEntryType
import java.nio.file.attribute.AclFileAttributeView
import java.nio.file.attribute.PosixFileAttributeView
import java.nio.file.attribute.PosixFilePermissions
import java.nio.file.attribute.UserPrincipal
import java.nio.file.attribute.UserPrincipalLookupService
import java.nio.file.attribute.UserPrincipalNotFoundException
import java.security.MessageDigest
import java.util.EnumSet

private const val MAX_NATIVE_LIBRARY_BYTES: Long = 134_217_728L
private const val DIGEST_BYTE_LENGTH: Int = 32
private const val DIGEST_METADATA_MAX_BYTES: Int = 96
private const val COPY_BUFFER_BYTES: Int = 8_192
private const val POSIX_GROUP_WRITE: Int = 0x10
private const val POSIX_OTHER_WRITE: Int = 0x02
private const val POSIX_STICKY: Int = 0x200
private const val WINDOWS_SYSTEM_SID: String = "S-1-5-18"
private const val WINDOWS_SYSTEM_ACCOUNT: String = "NT AUTHORITY\\SYSTEM"
private const val WINDOWS_ADMINISTRATORS_SID: String = "S-1-5-32-544"
private const val WINDOWS_ADMINISTRATORS_ACCOUNT: String = "BUILTIN\\Administrators"

private val digestMetadataPattern: Regex = Regex("^([0-9a-f]{64}) ([1-9][0-9]{0,11})\\n$")

internal fun readExpectedDigest(path: String): ExpectedNativeDigest? {
    val stream =
        ReallyMeCodecRustNativeProvider::class.java.getResourceAsStream(path)
            ?: return null
    val metadata =
        try {
            stream.use { readBounded(it, DIGEST_METADATA_MAX_BYTES) }
        } catch (_: IOException) {
            return null
        } catch (_: SecurityException) {
            return null
        } ?: return null
    return try {
        parseNativeDigestMetadata(metadata)
    } finally {
        metadata.fill(0)
    }
}

internal fun parseNativeDigestMetadata(metadata: ByteArray): ExpectedNativeDigest? {
    val match =
        digestMetadataPattern.matchEntire(
            metadata.toString(StandardCharsets.US_ASCII),
        ) ?: return null
    val size = match.groupValues[2].toLongOrNull() ?: return null
    if (size > MAX_NATIVE_LIBRARY_BYTES) {
        return null
    }
    val digest = decodeLowerHex(match.groupValues[1]) ?: return null
    return ExpectedNativeDigest(digest, size)
}

internal fun extractVerifiedLibrary(
    resource: NativeResource,
    expected: ExpectedNativeDigest,
): File? {
    val directory = createNativeExtractionDirectory() ?: return null
    val target = directory.resolve(resource.fileName)
    var completed = false
    val buffer = ByteArray(COPY_BUFFER_BYTES)
    try {
        val source =
            ReallyMeCodecRustNativeProvider::class.java
                .getResourceAsStream(resource.path) ?: return null
        val digest = MessageDigest.getInstance("SHA-256")
        var copied = 0L
        source.use { input ->
            FileOutputStream(target.toFile()).use { output ->
                while (true) {
                    val read = input.read(buffer)
                    if (read < 0) {
                        break
                    }
                    if (read == 0) {
                        return null
                    }
                    val remaining = MAX_NATIVE_LIBRARY_BYTES - copied
                    if (read.toLong() > remaining) {
                        return null
                    }
                    output.write(buffer, 0, read)
                    digest.update(buffer, 0, read)
                    copied += read.toLong()
                }
                output.fd.sync()
            }
        }
        if (
            copied != expected.size ||
            !MessageDigest.isEqual(digest.digest(), expected.sha256)
        ) {
            return null
        }
        if (!makeExtractedLibraryReadOnly(target)) {
            return null
        }
        if (!verifyExtractedLibrary(target, expected)) {
            return null
        }
        completed = true
        return target.toFile()
    } catch (_: IOException) {
        return null
    } catch (_: SecurityException) {
        return null
    } finally {
        buffer.fill(0)
        if (!completed) {
            deleteExtractionFiles(target, directory)
        }
    }
}

internal fun createNativeExtractionDirectory(configuredRoot: String? = System.getProperty("java.io.tmpdir")): Path? {
    return try {
        val rootValue = configuredRoot ?: return null
        val root = Path.of(rootValue).toRealPath(LinkOption.NOFOLLOW_LINKS)
        if (!Files.isDirectory(root, LinkOption.NOFOLLOW_LINKS)) {
            return null
        }
        val posixView =
            Files.getFileAttributeView(
                root,
                PosixFileAttributeView::class.java,
                LinkOption.NOFOLLOW_LINKS,
            )
        if (posixView != null) {
            val attributes = posixView.readAttributes()
            val mode =
                Files.getAttribute(
                    root,
                    "unix:mode",
                    LinkOption.NOFOLLOW_LINKS,
                ) as? Int ?: return null
            val currentUser = System.getProperty("user.name") ?: return null
            if (
                !isSecureNativePosixTempMode(mode) ||
                !isTrustedNativePosixTempOwner(attributes.owner().name, currentUser)
            ) {
                return null
            }
            Files.createTempDirectory(
                root,
                "reallyme-codec-native-",
                PosixFilePermissions.asFileAttribute(
                    PosixFilePermissions.fromString("rwx------"),
                ),
            )
        } else {
            val aclView =
                Files.getFileAttributeView(
                    root,
                    AclFileAttributeView::class.java,
                    LinkOption.NOFOLLOW_LINKS,
                ) ?: return null
            val currentUser = System.getProperty("user.name") ?: return null
            if (!isSecureAclTempRoot(aclView, currentUser)) {
                return null
            }
            val directory = Files.createTempDirectory(root, "reallyme-codec-native-")
            if (!restrictAclToOwner(directory, writable = true)) {
                deleteExtractionFiles(directory.resolve("unused"), directory)
                return null
            }
            directory
        }
    } catch (_: IOException) {
        null
    } catch (_: SecurityException) {
        null
    } catch (_: RuntimeException) {
        // InvalidPathException, UnsupportedOperationException, and
        // IllegalArgumentException report an unusable temporary root.
        null
    }
}

internal fun isSecureNativePosixTempMode(mode: Int): Boolean {
    val writableByAnotherPrincipal =
        (mode and (POSIX_GROUP_WRITE or POSIX_OTHER_WRITE)) != 0
    return !writableByAnotherPrincipal || (mode and POSIX_STICKY) != 0
}

internal fun isTrustedNativePosixTempOwner(
    owner: String,
    currentUser: String,
): Boolean = owner == currentUser || owner == "root" || owner == "0"

private fun isSecureAclTempRoot(
    view: AclFileAttributeView,
    currentUser: String,
): Boolean {
    // Windows display names are not identities: another domain can have
    // the same final username, and a name may contain a SID as text.
    // Resolve principals through the filesystem provider and compare the
    // returned identities, which Windows backs with account SIDs.
    val lookup = FileSystems.getDefault().userPrincipalLookupService
    val currentPrincipal = lookup.lookupPrincipalByName(currentUser)
    val systemPrincipal =
        lookupTrustedNativeAclPrincipal(
            lookup,
            WINDOWS_SYSTEM_SID,
            WINDOWS_SYSTEM_ACCOUNT,
        )
    val administratorsPrincipal =
        lookupTrustedNativeAclPrincipal(
            lookup,
            WINDOWS_ADMINISTRATORS_SID,
            WINDOWS_ADMINISTRATORS_ACCOUNT,
        )
    val owner = view.owner
    if (!isTrustedNativeAclPrincipal(owner, currentPrincipal, systemPrincipal, administratorsPrincipal)) {
        return false
    }
    val mutatingPermissions =
        EnumSet.of(
            AclEntryPermission.ADD_FILE,
            AclEntryPermission.ADD_SUBDIRECTORY,
            AclEntryPermission.APPEND_DATA,
            AclEntryPermission.DELETE,
            AclEntryPermission.DELETE_CHILD,
            AclEntryPermission.WRITE_ACL,
            AclEntryPermission.WRITE_ATTRIBUTES,
            AclEntryPermission.WRITE_DATA,
            AclEntryPermission.WRITE_NAMED_ATTRS,
            AclEntryPermission.WRITE_OWNER,
        )
    return view.acl.none { entry ->
        entry.type() == AclEntryType.ALLOW &&
            !isTrustedNativeAclPrincipal(
                entry.principal(),
                currentPrincipal,
                systemPrincipal,
                administratorsPrincipal,
            ) &&
            entry.permissions().any { it in mutatingPermissions }
    }
}

internal fun lookupTrustedNativeAclPrincipal(
    lookup: UserPrincipalLookupService,
    sid: String,
    qualifiedAccountName: String,
): UserPrincipal =
    try {
        lookup.lookupPrincipalByName(sid)
    } catch (_: UserPrincipalNotFoundException) {
        // Java's Windows provider resolves account names through
        // LookupAccountName. Some hosts do not accept textual SIDs there.
        // The fully qualified fallback is still resolved to a principal and
        // compared by identity, never by its display name.
        lookup.lookupPrincipalByName(qualifiedAccountName)
    }

internal fun isTrustedNativeAclPrincipal(
    principal: UserPrincipal,
    currentUser: UserPrincipal,
    system: UserPrincipal,
    administrators: UserPrincipal,
): Boolean = principal == currentUser || principal == system || principal == administrators

private fun restrictAclToOwner(
    path: Path,
    writable: Boolean,
): Boolean {
    return try {
        val view =
            Files.getFileAttributeView(
                path,
                AclFileAttributeView::class.java,
                LinkOption.NOFOLLOW_LINKS,
            ) ?: return false
        val permissions =
            if (writable) {
                EnumSet.allOf(AclEntryPermission::class.java)
            } else {
                EnumSet.of(
                    AclEntryPermission.EXECUTE,
                    AclEntryPermission.READ_ACL,
                    AclEntryPermission.READ_ATTRIBUTES,
                    AclEntryPermission.READ_DATA,
                    AclEntryPermission.READ_NAMED_ATTRS,
                    AclEntryPermission.SYNCHRONIZE,
                )
            }
        val ownerEntry =
            AclEntry
                .newBuilder()
                .setType(AclEntryType.ALLOW)
                .setPrincipal(view.owner)
                .setPermissions(permissions)
                .build()
        view.acl = listOf(ownerEntry)
        true
    } catch (_: IOException) {
        false
    } catch (_: SecurityException) {
        false
    }
}

private fun makeExtractedLibraryReadOnly(path: Path): Boolean =
    try {
        val posixView =
            Files.getFileAttributeView(
                path,
                PosixFileAttributeView::class.java,
                LinkOption.NOFOLLOW_LINKS,
            )
        if (posixView != null) {
            Files.setPosixFilePermissions(
                path,
                PosixFilePermissions.fromString("r--------"),
            )
            true
        } else {
            restrictAclToOwner(path, writable = false)
        }
    } catch (_: IOException) {
        false
    } catch (_: SecurityException) {
        false
    }

private fun verifyExtractedLibrary(
    path: Path,
    expected: ExpectedNativeDigest,
): Boolean {
    return try {
        if (Files.size(path) != expected.size || Files.isSymbolicLink(path)) {
            return false
        }
        val digest = MessageDigest.getInstance("SHA-256")
        val buffer = ByteArray(COPY_BUFFER_BYTES)
        var readTotal = 0L
        try {
            Files.newInputStream(path, StandardOpenOption.READ).use { input ->
                while (true) {
                    val read = input.read(buffer)
                    if (read < 0) {
                        break
                    }
                    if (read == 0) {
                        return false
                    }
                    val remaining = expected.size - readTotal
                    if (read.toLong() > remaining) {
                        return false
                    }
                    digest.update(buffer, 0, read)
                    readTotal += read.toLong()
                }
            }
        } finally {
            buffer.fill(0)
        }
        readTotal == expected.size &&
            MessageDigest.isEqual(digest.digest(), expected.sha256)
    } catch (_: IOException) {
        false
    } catch (_: SecurityException) {
        false
    }
}

private fun readBounded(
    input: InputStream,
    maximum: Int,
): ByteArray? {
    val result = ByteArray(maximum + 1)
    var offset = 0
    while (offset < result.size) {
        val read = input.read(result, offset, result.size - offset)
        if (read < 0) {
            return result.copyOf(offset).also { result.fill(0) }
        }
        if (read == 0) {
            result.fill(0)
            return null
        }
        offset += read
    }
    result.fill(0)
    return null
}

private fun decodeLowerHex(value: String): ByteArray? {
    if (value.length != DIGEST_BYTE_LENGTH * 2) {
        return null
    }
    val output = ByteArray(DIGEST_BYTE_LENGTH)
    for (index in output.indices) {
        val high = Character.digit(value[index * 2], 16)
        val low = Character.digit(value[index * 2 + 1], 16)
        if (high < 0 || low < 0) {
            output.fill(0)
            return null
        }
        output[index] = ((high shl 4) or low).toByte()
    }
    return output
}

internal fun deleteExtractionFiles(
    target: Path,
    directory: Path,
) {
    try {
        Files.deleteIfExists(target)
    } catch (_: IOException) {
        // Cleanup is best effort; a failed extraction is never loaded.
    } catch (_: SecurityException) {
        // Cleanup is best effort; a failed extraction is never loaded.
    }
    try {
        Files.deleteIfExists(directory)
    } catch (_: IOException) {
        // Cleanup is best effort; a failed extraction is never loaded.
    } catch (_: SecurityException) {
        // Cleanup is best effort; a failed extraction is never loaded.
    }
}
