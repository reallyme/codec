// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

enum ReallyMeCodecRustCAbiStatus {
    static let ok: Int32 = 0
    static let invalidArgument: Int32 = -1
    static let bufferTooSmall: Int32 = -5
    static let internalError: Int32 = -128

    static func throwIfError(_ status: Int32) throws {
        switch status {
        case ok:
            return
        case invalidArgument:
            throw ReallyMeCodecError.invalidInput
        case bufferTooSmall, internalError:
            throw ReallyMeCodecError.providerFailure
        default:
            throw ReallyMeCodecError.providerFailure
        }
    }
}
