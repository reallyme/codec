// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

@testable import ReallyMeCodec
import XCTest

extension ReallyMeCodecTests {
    func testSdkConvenienceFailureExposesWireContextWithoutRawResponseDecoding() throws {
        let codec = try Self.configuredCodec()
        XCTAssertThrowsError(try codec.deterministicCborDecode([0x18, 0x00])) { error in
            guard let failure = error as? ReallyMeCodecError else {
                XCTFail("expected typed SDK error")
                return
            }
            XCTAssertEqual(failure.category, .nonCanonical)
            XCTAssertEqual(failure.reason, .canonicalNonMinimalCborInteger)
            XCTAssertEqual(failure.origin, .caller)
        }
    }

    func testSdkErrorMapperRetainsEveryWireErrorFamily() {
        var base = ReallyMeProtoCodecError()
        base.baseEncoding.reason = .baseInvalidBase64
        var pem = ReallyMeProtoCodecError()
        pem.pem.reason = .pemResourceLimitExceeded
        var multiformat = ReallyMeProtoCodecError()
        multiformat.multiformat.reason = .multiformatUnknownMulticodec
        var canonical = ReallyMeProtoCodecError()
        canonical.canonicalization.reason = .canonicalNonCanonicalJson
        var internalCanonical = ReallyMeProtoCodecError()
        internalCanonical.canonicalization.reason = .canonicalInternal
        var backend = ReallyMeProtoCodecError()
        backend.backend.reason = .backendInternal
        var boundary = ReallyMeProtoCodecError()
        boundary.boundary.reason = .boundaryMalformedJson
        let cases:
            [(
                ReallyMeProtoCodecError, ReallyMeProtoCodecErrorReason, ReallyMeProtoCodecSdkErrorCategory,
                ReallyMeProtoCodecErrorOrigin
            )] = [
                (base, .baseInvalidBase64, .invalidInput, .caller),
                (pem, .pemResourceLimitExceeded, .invalidInput, .caller),
                (multiformat, .multiformatUnknownMulticodec, .unsupportedCodec, .caller),
                (canonical, .canonicalNonCanonicalJson, .nonCanonical, .caller),
                (internalCanonical, .canonicalInternal, .providerFailure, .provider),
                (backend, .backendInternal, .providerFailure, .provider),
                (boundary, .boundaryMalformedJson, .invalidInput, .caller),
            ]
        for (var error, reason, category, origin) in cases {
            error.origin = origin
            let mapped = ReallyMeCodecRustCAbiProvider.errorForCodecError(error)
            XCTAssertEqual(mapped.category, category)
            XCTAssertEqual(mapped.reason, reason)
            XCTAssertEqual(mapped.origin, origin)
        }
    }

    func testSdkErrorMapperRejectsUntrustedContext() {
        var error = ReallyMeProtoCodecError()
        error.boundary.reason = .boundaryMalformedJson
        error.origin = .provider
        var unknown = ReallyMeProtoCodecError()
        unknown.boundary.reason = .UNRECOGNIZED(699)
        unknown.origin = .caller
        for invalid in [error, unknown, ReallyMeProtoCodecError()] {
            let mapped = ReallyMeCodecRustCAbiProvider.errorForCodecError(invalid)
            XCTAssertEqual(mapped, .providerFailure)
            XCTAssertNil(mapped.reason)
            XCTAssertNil(mapped.origin)
        }
    }
}
