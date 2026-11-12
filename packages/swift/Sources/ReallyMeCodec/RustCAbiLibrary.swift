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
    /// at `path` is refused. Loading also fails when a `DYLD_LIBRARY_PATH`
    /// or `DYLD_IMAGE_SUFFIX` candidate would make the dynamic loader map a
    /// different file, and when the loaded image is not the hashed file.
    /// Store the library where only its owner can write it; this check does
    /// not close the window between hashing and loading if another process
    /// can rewrite the file.
    public convenience init(path: String, expectedSHA256: [UInt8]) throws {
        #if canImport(CryptoKit) && canImport(Darwin)
        guard expectedSHA256.count == SHA256.byteCount,
              (path as NSString).isAbsolutePath else {
            throw ReallyMeCodecError.dynamicLibraryNotFound
        }
        let resolvedPath = URL(fileURLWithPath: path).resolvingSymlinksInPath().path
        guard let hashedIdentity = LoadedFileIdentity(path: resolvedPath),
              let attributes = try? FileManager.default.attributesOfItem(atPath: resolvedPath),
              let size = attributes[.size] as? Int,
              size > 0,
              size <= Self.maximumVerifiedLibraryBytes,
              let contents = FileManager.default.contents(atPath: resolvedPath),
              contents.count == size else {
            throw ReallyMeCodecError.dynamicLibraryNotFound
        }
        guard Array(SHA256.hash(data: contents)) == expectedSHA256,
              LoadedFileIdentity(path: resolvedPath) == hashedIdentity else {
            throw ReallyMeCodecError.dynamicLibraryLoadFailed
        }
        try self.init(resolvedPath: resolvedPath, requiredIdentity: hashedIdentity)
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
    ///
    /// Loading fails when the dynamic loader maps a file other than the one
    /// at `path`.
    public convenience init(path: String) throws {
        #if canImport(Darwin)
        guard (path as NSString).isAbsolutePath else {
            throw ReallyMeCodecError.dynamicLibraryNotFound
        }
        let resolvedPath = URL(fileURLWithPath: path).resolvingSymlinksInPath().path
        guard let identity = LoadedFileIdentity(path: resolvedPath) else {
            throw ReallyMeCodecError.dynamicLibraryNotFound
        }
        try self.init(resolvedPath: resolvedPath, requiredIdentity: identity)
        #else
        _ = path
        throw ReallyMeCodecError.unsupportedPlatform
        #endif
    }

    /// Symbol used to locate the image that `dlopen` actually mapped.
    private static let identitySymbol = "rm_codec_abi_version"

    private init(resolvedPath: String, requiredIdentity: LoadedFileIdentity) throws {
        #if canImport(Darwin)
        // dyld consults these variables before the absolute path, so a
        // matching candidate would be mapped, and its initializers run,
        // instead of the requested file.
        guard !Self.dyldOverrideCandidates(for: resolvedPath).contains(where: { candidate in
            guard let candidateIdentity = LoadedFileIdentity(path: candidate) else {
                return false
            }
            return candidateIdentity != requiredIdentity
        }) else {
            throw ReallyMeCodecError.dynamicLibraryLoadFailed
        }
        guard let loadedHandle = dlopen(resolvedPath, RTLD_NOW | RTLD_LOCAL) else {
            throw ReallyMeCodecError.dynamicLibraryLoadFailed
        }
        guard Self.loadedImageIdentity(handle: loadedHandle) == requiredIdentity else {
            dlclose(loadedHandle)
            throw ReallyMeCodecError.dynamicLibraryLoadFailed
        }
        handle = loadedHandle
        #else
        _ = resolvedPath
        _ = requiredIdentity
        throw ReallyMeCodecError.unsupportedPlatform
        #endif
    }

    #if canImport(Darwin)
    private static func loadedImageIdentity(handle: UnsafeMutableRawPointer) -> LoadedFileIdentity? {
        guard let symbol = dlsym(handle, identitySymbol) else {
            return nil
        }
        var info = Dl_info()
        guard dladdr(symbol, &info) != 0, let imagePath = info.dli_fname else {
            return nil
        }
        return LoadedFileIdentity(path: String(cString: imagePath))
    }

    private static func dyldOverrideCandidates(for resolvedPath: String) -> [String] {
        let environment = ProcessInfo.processInfo.environment
        let url = URL(fileURLWithPath: resolvedPath)
        let leafName = url.lastPathComponent
        var leafNames = [leafName]
        if let suffix = environment["DYLD_IMAGE_SUFFIX"], !suffix.isEmpty {
            leafNames.append(leafName + suffix)
            let stem = url.deletingPathExtension().lastPathComponent
            if !url.pathExtension.isEmpty {
                leafNames.append(stem + suffix + "." + url.pathExtension)
            }
        }
        var candidates: [String] = []
        let directory = url.deletingLastPathComponent().path
        for name in leafNames.dropFirst() {
            candidates.append((directory as NSString).appendingPathComponent(name))
        }
        if let searchPath = environment["DYLD_LIBRARY_PATH"] {
            for searchDirectory in searchPath.split(separator: ":") where !searchDirectory.isEmpty {
                for name in leafNames {
                    candidates.append((String(searchDirectory) as NSString).appendingPathComponent(name))
                }
            }
        }
        return candidates
    }
    #endif

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

/// Device and inode pair that identifies one file independent of its path.
private struct LoadedFileIdentity: Equatable {
    let device: Int64
    let inode: UInt64

    init?(path: String) {
        #if canImport(Darwin)
        var status = stat()
        guard stat(path, &status) == 0, (status.st_mode & S_IFMT) == S_IFREG else {
            return nil
        }
        device = Int64(status.st_dev)
        inode = UInt64(status.st_ino)
        #else
        _ = path
        return nil
        #endif
    }
}
