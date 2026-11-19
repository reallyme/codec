// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
@testable import ReallyMeCodec
import SwiftProtobuf
import XCTest

private struct JsonVectorManifest: Decodable {
    let schemaVersion: Int
    let jsonDecode: JsonVectorSet
}

private struct JsonVectorSet: Decodable {
    let fixtureClass: String
    let cases: [JsonVector]
}

private struct JsonVector: Decodable {
    let name: String
    let requestHex: String
    let responseHex: String
}

final class JsonVectorTests: XCTestCase {
    func testSharedJsonWireBytesAndTypedOutcomes() throws {
        let codec = try ReallyMeCodecTests.configuredCodec()
        let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let path = root.appendingPathComponent("vectors/codec-vectors.json")
        let manifest = try JSONDecoder().decode(JsonVectorManifest.self, from: Data(contentsOf: path))
        XCTAssertEqual(manifest.schemaVersion, 2)
        XCTAssertEqual(manifest.jsonDecode.fixtureClass, "golden-operation")
        XCTAssertEqual(manifest.jsonDecode.cases.count, 16)
        for fixture in manifest.jsonDecode.cases {
            let requestBytes = try hexBytes(fixture.requestHex)
            let expectedBytes = try hexBytes(fixture.responseHex)
            let request = try ReallyMeProtoCodecApiOperationRequest(serializedBytes: requestBytes)
            guard case .jsonDecode(_)? = request.operation else {
                XCTFail("expected JSON decode request: \(fixture.name)")
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
                guard case .canonicalization(let detail)? = error.error else {
                    XCTFail("expected canonical JSON error: \(fixture.name)")
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
