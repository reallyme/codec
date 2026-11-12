// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#if canImport(CryptoKit)
import CryptoKit
#endif
#if canImport(Darwin)
import Darwin
#endif
import Foundation

/// Runtime handle for the ReallyMe codec symbols in the Rust C ABI library.
public final class ReallyMeCodecRustCAbiLibrary: @unchecked Sendable {
    private let handle: UnsafeMutableRawPointer

    /// Largest library file the digest-pinned initializer will hash.
    public static let maximumVerifiedLibraryBytes = 134_217_728

    /// Load a library only if its bytes match a pinned SHA-256 digest.
    ///
    /// The file is hashed before `dlopen`, so a replaced or tampered library
    /// at `path` is refused. Store the library where only its owner can
    /// write it; this check does not close the window between hashing and
    /// loading if another process can rewrite the file.
    public convenience init(path: String, expectedSHA256: [UInt8]) throws {
        #if canImport(CryptoKit) && canImport(Darwin)
        guard expectedSHA256.count == SHA256.byteCount,
              (path as NSString).isAbsolutePath else {
            throw ReallyMeCodecError.dynamicLibraryNotFound
        }
        let resolvedPath = URL(fileURLWithPath: path).resolvingSymlinksInPath().path
        guard let attributes = try? FileManager.default.attributesOfItem(atPath: resolvedPath),
              let size = attributes[.size] as? Int,
              size > 0,
              size <= Self.maximumVerifiedLibraryBytes,
              let contents = FileManager.default.contents(atPath: resolvedPath),
              contents.count == size else {
            throw ReallyMeCodecError.dynamicLibraryNotFound
        }
        guard Array(SHA256.hash(data: contents)) == expectedSHA256 else {
            throw ReallyMeCodecError.dynamicLibraryLoadFailed
        }
        try self.init(path: resolvedPath)
        #else
        _ = path
        _ = expectedSHA256
        throw ReallyMeCodecError.unsupportedPlatform
        #endif
    }

    /// Load a library from an absolute path without verifying its bytes.
    ///
    /// Use this only for libraries built in the same trusted environment,
    /// such as local tests. Prefer ``init(path:expectedSHA256:)`` whenever
    /// the file could be replaced by another process.
    public init(path: String) throws {
        #if canImport(Darwin)
        guard (path as NSString).isAbsolutePath else {
            throw ReallyMeCodecError.dynamicLibraryNotFound
        }
        let resolvedPath = URL(fileURLWithPath: path).resolvingSymlinksInPath().path
        guard FileManager.default.fileExists(atPath: resolvedPath) else {
            throw ReallyMeCodecError.dynamicLibraryNotFound
        }
        guard let loadedHandle = dlopen(resolvedPath, RTLD_NOW | RTLD_LOCAL) else {
            throw ReallyMeCodecError.dynamicLibraryLoadFailed
        }
        handle = loadedHandle
        #else
        _ = path
        throw ReallyMeCodecError.unsupportedPlatform
        #endif
    }

    deinit {
        #if canImport(Darwin)
        dlclose(handle)
        #endif
    }

    func loadFunction<Function>(_ symbol: StaticString, as _: Function.Type) throws -> Function {
        #if canImport(Darwin)
        guard let rawSymbol = dlsym(handle, symbol.description) else {
            throw ReallyMeCodecError.symbolNotFound
        }
        return unsafeBitCast(rawSymbol, to: Function.self)
        #else
        _ = symbol
        throw ReallyMeCodecError.unsupportedPlatform
        #endif
    }
}
