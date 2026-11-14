// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

enum ReallyMeCodecRustCAbiStatus {
    static let ok: Int32 = Int32(ReallyMeProtoCodecLegacyAbiStatus.unspecified.rawValue)
    static let invalidArgument: Int32 = Int32(ReallyMeProtoCodecLegacyAbiStatus.invalidArgument.rawValue)
    static let nonCanonicalHex: Int32 = -Int32(ReallyMeProtoCodecErrorReason.baseNonCanonicalHex.rawValue)
    static let nonCanonicalJson: Int32 = -Int32(ReallyMeProtoCodecErrorReason.canonicalNonCanonicalJson.rawValue)
    static let invalidMulticodecPrefix: Int32 = -Int32(
        ReallyMeProtoCodecErrorReason.multiformatInvalidMulticodecPrefix.rawValue)
    static let unknownMulticodec: Int32 = -Int32(ReallyMeProtoCodecErrorReason.multiformatUnknownMulticodec.rawValue)
    static let bufferTooSmall: Int32 = Int32(ReallyMeProtoCodecLegacyAbiStatus.bufferTooSmall.rawValue)
    static let internalError: Int32 = Int32(ReallyMeProtoCodecLegacyAbiStatus.internalError.rawValue)

    static func throwIfError(_ status: Int32) throws(ReallyMeCodecError) {
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
