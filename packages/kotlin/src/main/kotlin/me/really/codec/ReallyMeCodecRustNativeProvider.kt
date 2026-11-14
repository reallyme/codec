// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import me.really.codec.v1.CodecLibraryLoadRequest
import java.io.File
import java.io.IOException
import java.nio.file.Path
import java.nio.file.attribute.UserPrincipal
import java.nio.file.attribute.UserPrincipalLookupService
import java.util.Locale

/**
 * Explicit loader for the ReallyMe Rust codec provider.
 *
 * Kotlin does not implement codec primitives locally. The Maven artifact ships
 * platform native libraries as resources and loads the matching one on first
 * use; `loadLibrary(path)` remains available for local development and tests
 * against a freshly built Rust cdylib.
 */
public object ReallyMeCodecRustNativeProvider {
    private const val RESOURCE_ROOT: String = "/me/really/codec/native"
    private const val ANDROID_LIBRARY_NAME: String = "reallyme_codec_ffi"
    private const val EXPECTED_CODEC_ABI_VERSION: Int = 6
    private const val EXPECTED_CODEC_PACKAGE_MAJOR: Int = 0
    private const val EXPECTED_CODEC_PACKAGE_MINOR: Int = 4
    private const val EXPECTED_CODEC_PACKAGE_PATCH: Int = 0
    private const val MAX_MANAGED_FFI_LIMIT_BYTES: Long = Int.MAX_VALUE.toLong()
    private const val ALLOW_UNVERIFIED_NATIVE_PROPERTY: String = "reallyme.codec.allowUnverifiedNative"

    @Volatile
    private var loaded: Boolean = false

    @Volatile
    private var explicitlyLoadedPath: String? = null

    @Volatile
    private var loadFailed: Boolean = false

    @Volatile
    private var maxFfiInputBytes: Int = 0

    @Volatile
    private var maxFfiOutputBytes: Int = 0

    @Volatile
    private var maxOperationResponseBytes: Int = 0

    @JvmStatic
    @Synchronized
    public fun loadLibrary(path: String) {
        loadLibraryRequest(CodecLibraryLoadRequest.newBuilder().setAbsolutePath(path).build())
    }

    private fun loadLibraryRequest(request: CodecLibraryLoadRequest) {
        val path = request.absolutePath
        if (path.isEmpty()) {
            throw ReallyMeCodecException.InvalidInput()
        }
        val library = File(path)
        if (!library.isAbsolute || !library.isFile) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        // An external path bypasses the bundled resource digest. Require an
        // explicit local-development opt-in before loading unmanaged code.
        val explicitlyAllowed =
            try {
                System.getProperty(ALLOW_UNVERIFIED_NATIVE_PROPERTY) == "true"
            } catch (_: SecurityException) {
                false
            }
        if (!explicitlyAllowed) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        val canonicalPath =
            try {
                library.canonicalPath
            } catch (_: IOException) {
                throw ReallyMeCodecException.ProviderFailure()
            } catch (_: SecurityException) {
                throw ReallyMeCodecException.ProviderFailure()
            }
        if (loaded) {
            if (canonicalPath != explicitlyLoadedPath) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            return
        }
        if (loadFailed) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        try {
            System.load(canonicalPath)
            if (!validateLoadedNativeContract()) {
                loadFailed = true
                throw ReallyMeCodecException.ProviderFailure()
            }
            explicitlyLoadedPath = canonicalPath
            loaded = true
        } catch (_: LinkageError) {
            loadFailed = true
            throw ReallyMeCodecException.ProviderFailure()
        } catch (_: SecurityException) {
            loadFailed = true
            throw ReallyMeCodecException.ProviderFailure()
        }
    }

    @Synchronized
    internal fun requireLoaded() {
        if (loaded) {
            return
        }
        if (loadFailed) {
            throw ReallyMeCodecException.ProviderFailure()
        }
        if (isAndroidRuntime()) {
            if (loadAndroidLibrary()) {
                return
            }
            // Never fall through to the desktop JVM extraction path on
            // Android. Besides being inapplicable to packaged jniLibs, that
            // path uses java.nio.file APIs that are unavailable on API 24-25.
            loadFailed = true
            throw ReallyMeCodecException.ProviderFailure()
        }
        // Path and attribute APIs can throw unchecked exceptions for unusual
        // temporary-directory configurations; any of them is a typed,
        // latched provider failure rather than an escaping JDK exception.
        val bundledLoaded =
            try {
                loadBundledLibrary()
            } catch (_: RuntimeException) {
                false
            }
        if (!bundledLoaded) {
            // A failed native load cannot be undone in this classloader. Keep
            // one terminal failure so repeated calls cannot fill temporary
            // storage with verified but unloadable native libraries.
            loadFailed = true
            throw ReallyMeCodecException.ProviderFailure()
        }
    }

    private fun loadAndroidLibrary(): Boolean =
        try {
            // Android installs AAR `jniLibs/<abi>/libreallyme_codec_ffi.so`
            // under the app native-library directory. Those files are not
            // classpath resources, so the JVM resource extraction path cannot
            // see them; the platform loader must resolve them by library name.
            System.loadLibrary(ANDROID_LIBRARY_NAME)
            if (!validateLoadedNativeContract()) {
                false
            } else {
                loaded = true
                true
            }
        } catch (_: LinkageError) {
            false
        } catch (_: SecurityException) {
            false
        }

    private fun loadBundledLibrary(): Boolean {
        val resource = platformNativeResource() ?: return false
        val expected = readExpectedDigest(resource.digestPath) ?: return false
        val extracted = extractVerifiedLibrary(resource, expected) ?: return false
        val loadedSuccessfully = loadExtractedLibrary(extracted)
        if (loadedSuccessfully) {
            // The JVM deletes in reverse registration order. Register the
            // directory first so the library is removed before its parent.
            extracted.parentFile?.deleteOnExit()
            extracted.deleteOnExit()
        } else {
            val extractedPath = extracted.toPath()
            val directory = extractedPath.parent
            if (directory != null) {
                deleteExtractionFiles(extractedPath, directory)
            }
        }
        return loadedSuccessfully
    }

    internal fun parseDigestMetadata(metadata: ByteArray): ExpectedNativeDigest? = parseNativeDigestMetadata(metadata)

    internal fun createPrivateExtractionDirectory(configuredRoot: String? = System.getProperty("java.io.tmpdir")): Path? =
        createNativeExtractionDirectory(configuredRoot)

    internal fun isSecurePosixTempMode(mode: Int): Boolean = isSecureNativePosixTempMode(mode)

    internal fun isTrustedPosixTempOwner(
        owner: String,
        currentUser: String,
    ): Boolean = isTrustedNativePosixTempOwner(owner, currentUser)

    internal fun lookupTrustedAclPrincipal(
        lookup: UserPrincipalLookupService,
        sid: String,
        account: String,
    ): UserPrincipal? = lookupTrustedNativeAclPrincipal(lookup, sid, account)

    internal fun isTrustedAclPrincipal(
        owner: UserPrincipal,
        current: UserPrincipal,
        system: UserPrincipal,
        administrators: UserPrincipal,
    ): Boolean = isTrustedNativeAclPrincipal(owner, current, system, administrators)

    private fun loadExtractedLibrary(path: File): Boolean =
        try {
            System.load(path.absolutePath)
            if (!validateLoadedNativeContract()) {
                false
            } else {
                loaded = true
                true
            }
        } catch (_: LinkageError) {
            false
        } catch (_: SecurityException) {
            false
        }

    internal fun platformNativeResource(
        osName: String? = System.getProperty("os.name"),
        osArch: String? = System.getProperty("os.arch"),
        androidRuntime: Boolean = isAndroidRuntime(),
    ): NativeResource? {
        if (androidRuntime) {
            return null
        }
        val os = normalizedOs(osName) ?: return null
        val arch = normalizedArch(osArch) ?: return null
        val fileName = nativeLibraryFileName(os)
        return NativeResource(
            fileName = fileName,
            path = "$RESOURCE_ROOT/$os-$arch/$fileName",
            digestPath = "$RESOURCE_ROOT/$os-$arch/$fileName.sha256",
        )
    }

    internal fun normalizedOs(osName: String?): String? {
        val value = osName?.lowercase(Locale.ROOT) ?: return null
        return when {
            value.contains("mac") || value.contains("darwin") -> "macos"
            value.contains("linux") -> "linux"
            value.contains("windows") -> "windows"
            else -> null
        }
    }

    internal fun normalizedArch(osArch: String?): String? =
        when (osArch?.lowercase(Locale.ROOT)) {
            "aarch64", "arm64" -> "aarch64"
            "amd64", "x86_64" -> "x86_64"
            else -> null
        }

    internal fun isAndroidRuntime(
        runtimeName: String? = System.getProperty("java.runtime.name"),
        vmName: String? = System.getProperty("java.vm.name"),
        vmVendor: String? = System.getProperty("java.vm.vendor"),
    ): Boolean =
        containsAndroidMarker(runtimeName) ||
            containsAndroidMarker(vmName) ||
            containsAndroidMarker(vmVendor)

    private fun containsAndroidMarker(value: String?): Boolean {
        val normalized = value?.lowercase(Locale.ROOT) ?: return false
        return normalized.contains("android") || normalized.contains("dalvik")
    }

    private fun nativeLibraryFileName(os: String): String =
        when (os) {
            "macos" -> "libreallyme_codec_ffi.dylib"
            "windows" -> "reallyme_codec_ffi.dll"
            else -> "libreallyme_codec_ffi.so"
        }

    internal fun ffiInputLimit(): Int {
        requireLoaded()
        return maxFfiInputBytes
    }

    internal fun ffiOutputLimit(): Int {
        requireLoaded()
        return maxFfiOutputBytes
    }

    internal fun operationResponseLimit(): Int {
        requireLoaded()
        return maxOperationResponseBytes
    }

    internal fun isCompatibleAbiVersion(actualVersion: Int): Boolean = actualVersion == EXPECTED_CODEC_ABI_VERSION

    internal fun isCompatiblePackageVersion(
        major: Int,
        minor: Int,
        patch: Int,
    ): Boolean =
        major == EXPECTED_CODEC_PACKAGE_MAJOR &&
            minor == EXPECTED_CODEC_PACKAGE_MINOR &&
            patch == EXPECTED_CODEC_PACKAGE_PATCH

    internal fun isValidNativeLimit(limit: Long): Boolean = limit in 1..MAX_MANAGED_FFI_LIMIT_BYTES

    private fun validateLoadedNativeContract(): Boolean {
        if (ReallyMeCodecNative.probeNative() != 1) {
            return false
        }
        if (!isCompatibleAbiVersion(ReallyMeCodecNative.abiVersionNative())) {
            return false
        }
        // Android resolves the library by name, so another dependency can
        // supply an older build with the same ABI. Pin the exact release too.
        if (
            !isCompatiblePackageVersion(
                ReallyMeCodecNative.packageVersionMajorNative(),
                ReallyMeCodecNative.packageVersionMinorNative(),
                ReallyMeCodecNative.packageVersionPatchNative(),
            )
        ) {
            return false
        }
        val inputLimit = ReallyMeCodecNative.maxFfiInputBytesNative()
        val outputLimit = ReallyMeCodecNative.maxFfiOutputBytesNative()
        val operationLimit = ReallyMeCodecNative.maxOperationResponseBytesNative()
        if (
            !isValidNativeLimit(inputLimit) ||
            !isValidNativeLimit(outputLimit) ||
            !isValidNativeLimit(operationLimit) ||
            operationLimit > outputLimit
        ) {
            return false
        }
        maxFfiInputBytes = inputLimit.toInt()
        maxFfiOutputBytes = outputLimit.toInt()
        maxOperationResponseBytes = operationLimit.toInt()
        return true
    }

    internal data class NativeResource(
        val fileName: String,
        val path: String,
        val digestPath: String,
    )

    internal data class ExpectedNativeDigest(
        val sha256: ByteArray,
        val size: Long,
    )
}
