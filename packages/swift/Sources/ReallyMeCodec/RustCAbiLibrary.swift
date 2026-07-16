// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#if canImport(Darwin)
import Darwin
#endif
import Foundation

/// Runtime handle for the ReallyMe codec symbols in the Rust C ABI library.
public final class ReallyMeCodecRustCAbiLibrary: @unchecked Sendable {
    private let handle: UnsafeMutableRawPointer

    public init(path: String) throws {
        #if canImport(Darwin)
        guard FileManager.default.fileExists(atPath: path) else {
            throw ReallyMeCodecError.dynamicLibraryNotFound
        }
        guard let loadedHandle = dlopen(path, RTLD_NOW | RTLD_LOCAL) else {
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
