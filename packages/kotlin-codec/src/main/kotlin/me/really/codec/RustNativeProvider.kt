// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec

import java.io.File
import java.io.FileOutputStream
import java.io.IOException
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

    @Volatile
    private var loaded: Boolean = false

    @JvmStatic
    @Synchronized
    public fun loadLibrary(path: String) {
        if (path.isEmpty()) {
            throw ReallyMeCodecException.InvalidInput()
        }
        val library = File(path)
        if (loaded) {
            if (!library.isFile) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            return
        }
        try {
            System.load(library.absolutePath)
            if (ReallyMeCodecNative.probeNative() != 1) {
                throw ReallyMeCodecException.ProviderFailure()
            }
            loaded = true
        } catch (_: LinkageError) {
            throw ReallyMeCodecException.ProviderFailure()
        } catch (_: SecurityException) {
            throw ReallyMeCodecException.ProviderFailure()
        }
    }

    @Synchronized
    internal fun requireLoaded() {
        if (loaded) {
            return
        }
        if (isAndroidRuntime() && loadAndroidLibrary()) {
            return
        }
        if (!loadBundledLibrary()) {
            throw ReallyMeCodecException.ProviderFailure()
        }
    }

    private fun loadAndroidLibrary(): Boolean {
        return try {
            // Android installs AAR `jniLibs/<abi>/libreallyme_codec_ffi.so`
            // under the app native-library directory. Those files are not
            // classpath resources, so the JVM resource extraction path cannot
            // see them; the platform loader must resolve them by library name.
            System.loadLibrary(ANDROID_LIBRARY_NAME)
            if (ReallyMeCodecNative.probeNative() != 1) {
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
    }

    private fun loadBundledLibrary(): Boolean {
        val resource = platformNativeResource() ?: return false
        val stream = ReallyMeCodecRustNativeProvider::class.java.getResourceAsStream(resource.path)
            ?: return false
        val extracted = try {
            stream.use { source ->
                val target = File.createTempFile(
                    "reallyme-codec-native-",
                    "-${resource.fileName}",
                )
                FileOutputStream(target).use { destination ->
                    source.copyTo(destination)
                }
                target.deleteOnExit()
                target
            }
        } catch (_: IOException) {
            return false
        } catch (_: SecurityException) {
            return false
        }

        return loadExtractedLibrary(extracted)
    }

    private fun loadExtractedLibrary(path: File): Boolean {
        return try {
            System.load(path.absolutePath)
            if (ReallyMeCodecNative.probeNative() != 1) {
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

    internal fun normalizedArch(osArch: String?): String? {
        return when (osArch?.lowercase(Locale.ROOT)) {
            "aarch64", "arm64" -> "aarch64"
            "amd64", "x86_64" -> "x86_64"
            else -> null
        }
    }

    internal fun isAndroidRuntime(
        runtimeName: String? = System.getProperty("java.runtime.name"),
        vmName: String? = System.getProperty("java.vm.name"),
        vmVendor: String? = System.getProperty("java.vm.vendor"),
    ): Boolean {
        return containsAndroidMarker(runtimeName) ||
            containsAndroidMarker(vmName) ||
            containsAndroidMarker(vmVendor)
    }

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

    internal data class NativeResource(
        val fileName: String,
        val path: String,
    )
}
