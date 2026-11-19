// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
@testable import ReallyMeCodec
import SwiftProtobuf
import XCTest

private struct Base64LimitVectorManifest: Decodable {
    let schemaVersion: Int
    let base64DecodeWithLimits: Base64LimitVectorSet
}

private struct Base64LimitVectorSet: Decodable {
    let fixtureClass: String
    let cases: [Base64LimitVector]
}

private struct Base64LimitVector: Decodable {
    let name: String
    let requestHex: String
    let responseHex: String
}

final class Base64LimitVectorTests: XCTestCase {
    func testSharedBoundedBase64WireBytesAndTypedOutcomes() throws {
        let codec = try ReallyMeCodecTests.configuredCodec()
        let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let path = root.appendingPathComponent("vectors/codec-vectors.json")
        let manifest = try JSONDecoder().decode(Base64LimitVectorManifest.self, from: Data(contentsOf: path))
        XCTAssertEqual(manifest.schemaVersion, 2)
        XCTAssertEqual(manifest.base64DecodeWithLimits.fixtureClass, "golden-operation")
        XCTAssertEqual(manifest.base64DecodeWithLimits.cases.count, 16)
        for fixture in manifest.base64DecodeWithLimits.cases {
            let requestBytes = try hexBytes(fixture.requestHex)
            let expectedBytes = try hexBytes(fixture.responseHex)
            let request = try ReallyMeProtoCodecApiOperationRequest(serializedBytes: requestBytes)
            guard case .base64DecodeWithLimits(_)? = request.operation else {
                XCTFail("expected bounded Base64 request: \(fixture.name)")
                continue
            }
            XCTAssertEqual(Array(try request.serializedData()), requestBytes, fixture.name)
            let actualBytes = try codec.processOperation(requestBytes)
            XCTAssertEqual(actualBytes, expectedBytes, fixture.name)
            let actual = try ReallyMeProtoCodecApiOperationResponse(serializedBytes: actualBytes)
            let expected = try ReallyMeProtoCodecApiOperationResponse(serializedBytes: expectedBytes)
            XCTAssertEqual(actual, expected, fixture.name)
            if case .error(let error)? = actual.outcome {
                XCTAssertEqual(error.origin, .caller, fixture.name)
                guard case .baseEncoding(let detail)? = error.error else {
                    XCTFail("expected Base64 error: \(fixture.name)")
                    continue
                }
                XCTAssertNotEqual(detail.reason, .unspecified, fixture.name)
            }
        }
    }

    private func hexBytes(_ text: String) throws -> [UInt8] {
        var characters = text.makeIterator()
        var bytes: [UInt8] = []
        while let first = characters.next() {
            let second = try XCTUnwrap(characters.next())
            bytes.append(try XCTUnwrap(UInt8(String([first, second]), radix: 16)))
        }
        return bytes
    }
}
