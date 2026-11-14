// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
import XCTest
@testable import ReallyMeCodec

final class FieldPresenceTests: XCTestCase {
    private func metadata(_ fixed: UInt32, _ variable: Bool) -> ReallyMeProtoCodecMulticodecSpec {
        var value = ReallyMeProtoCodecMulticodecSpec()
        value.tag = .key
        value.keyMaterialKind = .publicKey
        value.fixedLength = fixed
        value.variableLength = variable
        return value
    }

    private func assertProviderFailure<T>(_ action: @autoclosure () throws -> T) {
        XCTAssertThrowsError(try action()) { error in
            XCTAssertEqual(error as? ReallyMeCodecError, .providerFailure)
        }
    }

    func testMetadataLengthStates() throws {
        assertProviderFailure(try sdkMulticodecMetadata(from: metadata(32, true)))
        XCTAssertEqual(try sdkMulticodecMetadata(from: metadata(32, false)).expectedKeyLength, 32)
        XCTAssertNil(try sdkMulticodecMetadata(from: metadata(0, true)).expectedKeyLength)
        XCTAssertNil(try sdkMulticodecMetadata(from: metadata(0, false)).expectedKeyLength)
    }

    func testParsedKeyLengthStates() throws {
        var value = ReallyMeProtoCodecMultikeyParseResult()
        value.publicKey = Data(repeating: 0, count: 32)
        let invalid: [(UInt32, Bool)] = [(32, true), (0, false), (31, false)]
        for (expected, variable) in invalid {
            value.expectedPublicKeyLength = expected
            value.variablePublicKeyLength = variable
            assertProviderFailure(try sdkParsedMultikey(from: value))
        }
        value.expectedPublicKeyLength = 32
        value.variablePublicKeyLength = false
        XCTAssertEqual(try sdkParsedMultikey(from: value).expectedPublicKeyLength, 32)
        value.expectedPublicKeyLength = 0
        value.variablePublicKeyLength = true
        XCTAssertNil(try sdkParsedMultikey(from: value).expectedPublicKeyLength)
    }

    func testGeneratedPresencePreservesDefaultsInBinaryAndJSON() throws {
        var algorithm = ReallyMeProtoCodecValidateKeyBindingRequest()
        XCTAssertFalse(algorithm.hasAlgorithm)
        algorithm.algorithm = ""
        XCTAssertTrue(algorithm.hasAlgorithm)
        XCTAssertEqual(try algorithm.serializedData(), Data([0x12, 0]))
        XCTAssertTrue(
            try ReallyMeProtoCodecValidateKeyBindingRequest(serializedBytes: algorithm.serializedData()).hasAlgorithm)
        XCTAssertTrue(try ReallyMeProtoCodecValidateKeyBindingRequest(jsonString: algorithm.jsonString()).hasAlgorithm)

        var limits = ReallyMeProtoCodecGeneralCborOptions()
        XCTAssertFalse(limits.hasInputBytes)
        limits.inputBytes = 0
        XCTAssertEqual(try limits.serializedData(), Data([0x28, 0]))
        XCTAssertTrue(try ReallyMeProtoCodecGeneralCborOptions(serializedBytes: limits.serializedData()).hasInputBytes)
        XCTAssertTrue(try ReallyMeProtoCodecGeneralCborOptions(jsonString: limits.jsonString()).hasInputBytes)

        var der = ReallyMeProtoCodecDerValue()
        XCTAssertFalse(der.hasBoolValue)
        der.boolValue = false
        XCTAssertEqual(try der.serializedData(), Data([0x28, 0]))
        XCTAssertTrue(try ReallyMeProtoCodecDerValue(serializedBytes: der.serializedData()).hasBoolValue)
        XCTAssertTrue(try ReallyMeProtoCodecDerValue(jsonString: der.jsonString()).hasBoolValue)
    }
}
