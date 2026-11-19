// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import ReallyMeCodec
import XCTest

extension ReallyMeCodecTests {
    func testPublicSdkErrorContextUsesGeneratedReasonAndOrigin() {
        let reason: ReallyMeProtoCodecErrorReason = .canonicalNonCanonicalJson
        let origin: ReallyMeProtoCodecErrorOrigin = .caller
        let error = ReallyMeCodecError.operationFailure(category: .nonCanonical, reason: reason, origin: origin)
        XCTAssertEqual(error.category, .nonCanonical)
        XCTAssertEqual(error.reason, reason)
        XCTAssertEqual(error.origin, origin)
        XCTAssertNil(ReallyMeCodecError.symbolNotFound.reason)
        XCTAssertNil(ReallyMeCodecError.providerFailure.origin)
        XCTAssertEqual(ReallyMeCodecError.invalidInput.category, .invalidInput)
    }
}
