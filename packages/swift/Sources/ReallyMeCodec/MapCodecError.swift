// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation

extension ReallyMeCodecRustCAbiProvider {
    static func errorForCodecError(_ codecError: ReallyMeProtoCodecError) -> ReallyMeCodecError {
        guard codecError.unknownFields.data.isEmpty else {
            return .providerFailure
        }
        let expectedOrigin: ReallyMeProtoCodecErrorOrigin
        let reason: ReallyMeProtoCodecErrorReason
        switch codecError.error {
        case .baseEncoding(let error):
            guard error.unknownFields.data.isEmpty,
                Self.isKnownReason(error.reason, range: 100...199)
            else { return .providerFailure }
            expectedOrigin = .caller
            reason = error.reason
        case .pem(let error):
            guard error.unknownFields.data.isEmpty,
                Self.isKnownReason(error.reason, range: 200...299)
            else { return .providerFailure }
            expectedOrigin = .caller
            reason = error.reason
        case .multiformat(let error):
            guard error.unknownFields.data.isEmpty,
                Self.isKnownReason(error.reason, range: 300...399)
            else { return .providerFailure }
            expectedOrigin = .caller
            reason = error.reason
        case .canonicalization(let error):
            guard error.unknownFields.data.isEmpty,
                Self.isKnownReason(error.reason, range: 400...499)
            else { return .providerFailure }
            expectedOrigin = error.reason == .canonicalInternal ? .provider : .caller
            reason = error.reason
        case .backend(let error):
            guard error.unknownFields.data.isEmpty,
                Self.isKnownReason(error.reason, range: 500...599)
            else { return .providerFailure }
            expectedOrigin = .provider
            reason = error.reason
        case .boundary(let error):
            guard error.unknownFields.data.isEmpty,
                Self.isKnownReason(error.reason, range: 600...699)
            else { return .providerFailure }
            expectedOrigin = .caller
            reason = error.reason
        case nil:
            return .providerFailure
        }
        guard codecError.origin == expectedOrigin else {
            return .providerFailure
        }
        guard expectedOrigin == .caller else {
            return .operationFailure(category: .providerFailure, reason: reason, origin: expectedOrigin)
        }
        let category: ReallyMeProtoCodecSdkErrorCategory
        switch codecError.error {
        case .baseEncoding(let error) where error.reason == .baseNonCanonicalHex:
            category = .nonCanonical
        case .multiformat(let error)
        where error.reason == .multiformatUnknownMulticodec || error.reason == .multiformatInvalidMulticodecPrefix:
            category = .unsupportedCodec
        case .canonicalization(let error) where error.reason == .canonicalUnsupportedIpldValue:
            category = .unsupportedIpldValue
        case .canonicalization(let error) where Self.isNonCanonicalReason(error.reason):
            category = .nonCanonical
        default:
            category = .invalidInput
        }
        return .operationFailure(category: category, reason: reason, origin: expectedOrigin)
    }

    private static func isNonCanonicalReason(
        _ reason: ReallyMeProtoCodecErrorReason
    ) -> Bool {
        switch reason {
        case .canonicalNonCanonicalCbor,
            .canonicalNonCanonicalJson,
            .canonicalNonMinimalCborInteger,
            .canonicalDuplicateCborMapKey,
            .canonicalCborMapKeysOutOfOrder,
            .canonicalCborTrailingBytes:
            return true
        default:
            return false
        }
    }

    private static func isKnownReason(
        _ reason: ReallyMeProtoCodecErrorReason,
        range: ClosedRange<Int>
    ) -> Bool {
        if case .UNRECOGNIZED = reason {
            return false
        }
        return range.contains(reason.rawValue)
    }
}
