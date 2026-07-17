// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import Foundation
#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

enum ReallyMeCodecMemory {
    /// Clears a uniquely owned byte array using a platform primitive whose
    /// contract prevents dead-store elimination.
    ///
    /// Swift collections use copy-on-write, so this helper is intentionally
    /// named for its ownership precondition: callers must pass a fresh mutable
    /// owner with no surviving aliases. Public caller-owned inputs are borrowed
    /// and are never represented as wipeable SDK-owned storage.
    static func clearOwned(_ bytes: inout [UInt8]) {
        bytes.withUnsafeMutableBufferPointer { buffer in
            clear(UnsafeMutableRawBufferPointer(buffer))
        }
    }

    /// Clears the mutable region represented by a uniquely owned `Data`,
    /// including slices whose collection indices do not start at zero.
    static func clearOwned(_ bytes: inout Data) {
        bytes.withUnsafeMutableBytes { (buffer: UnsafeMutableRawBufferPointer) in
            clear(buffer)
        }
    }

    private static func clear(_ buffer: UnsafeMutableRawBufferPointer) {
        guard let baseAddress = buffer.baseAddress, !buffer.isEmpty else {
            return
        }
        #if canImport(Darwin)
        _ = memset_s(baseAddress, buffer.count, 0, buffer.count)
        #elseif canImport(Glibc)
        explicit_bzero(baseAddress, buffer.count)
        #else
        #error("ReallyMeCodec requires a non-elidable platform memory wipe primitive")
        #endif
    }
}
