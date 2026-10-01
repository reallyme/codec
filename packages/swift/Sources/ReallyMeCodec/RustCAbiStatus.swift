// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

enum ReallyMeCodecRustCAbiStatus {
    static let ok: Int32 = 0
    static let invalidArgument: Int32 = -1
    static let nonCanonicalHex: Int32 = -121
    static let nonCanonicalJson: Int32 = -403
    static let invalidMulticodecPrefix: Int32 = -301
    static let unknownMulticodec: Int32 = -302
    static let bufferTooSmall: Int32 = -5
    static let internalError: Int32 = -128

    static func throwIfError(_ status: Int32) throws {
        switch status {
        case ok:
            return
        case invalidArgument:
            throw ReallyMeCodecError.invalidInput
        case nonCanonicalHex, nonCanonicalJson:
            throw ReallyMeCodecError.nonCanonical
        case invalidMulticodecPrefix, unknownMulticodec:
            throw ReallyMeCodecError.unsupportedCodec
        case bufferTooSmall, internalError:
            throw ReallyMeCodecError.providerFailure
        default:
            throw ReallyMeCodecError.providerFailure
        }
    }
}
