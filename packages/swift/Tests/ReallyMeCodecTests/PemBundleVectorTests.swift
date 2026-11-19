// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
@testable import ReallyMeCodec
import SwiftProtobuf
import XCTest

private struct PemBundleVectorManifest: Decodable {
    let schemaVersion: Int
    let pemBundleDecode: PemBundleVectorSet
}

private struct PemBundleVectorSet: Decodable {
    let fixtureClass: String
    let cases: [PemBundleVector]
}

private struct PemBundleVector: Decodable {
    let name: String
    let requestHex: String
    let responseHex: String
}

final class PemBundleVectorTests: XCTestCase {
    func testSharedPemBundleWireBytesAndTypedOutcomes() throws {
        let codec = try ReallyMeCodecTests.configuredCodec()
        let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let candidates = [
            root.appendingPathComponent("vectors/codec-vectors.json"),
            root.appendingPathComponent("../../vectors/codec-vectors.json"),
        ]
        let path = try XCTUnwrap(candidates.first { FileManager.default.fileExists(atPath: $0.path) })
        let manifest = try JSONDecoder().decode(PemBundleVectorManifest.self, from: Data(contentsOf: path))
        XCTAssertEqual(manifest.schemaVersion, 2)
        XCTAssertEqual(manifest.pemBundleDecode.fixtureClass, "golden-operation")
        XCTAssertFalse(manifest.pemBundleDecode.cases.isEmpty)

        for fixture in manifest.pemBundleDecode.cases {
            let requestBytes = try hexBytes(fixture.requestHex)
            let expectedBytes = try hexBytes(fixture.responseHex)
            let request = try ReallyMeProtoCodecApiOperationRequest(serializedBytes: requestBytes)
            guard case .pemBundleDecode(_)? = request.operation else {
                XCTFail("expected PEM bundle request: \(fixture.name)")
                continue
            }
            XCTAssertEqual(Array(try request.serializedData()), requestBytes, fixture.name)
            let actualBytes = try codec.processOperation(requestBytes)
            XCTAssertEqual(actualBytes, expectedBytes, fixture.name)
            let actual = try ReallyMeProtoCodecApiOperationResponse(serializedBytes: actualBytes)
            let expected = try ReallyMeProtoCodecApiOperationResponse(serializedBytes: expectedBytes)
            switch (actual.outcome, expected.outcome) {
            case (.result(let result)?, .result(let expectedResult)?):
                guard case .pemBundleDecode(let bundle)? = result.result,
                    case .pemBundleDecode(let expectedBundle)? = expectedResult.result
                else {
                    XCTFail("expected PEM bundle result: \(fixture.name)")
                    continue
                }
                XCTAssertEqual(bundle.blocks.count, expectedBundle.blocks.count, fixture.name)
                for (block, expectedBlock) in zip(bundle.blocks, expectedBundle.blocks) {
                    XCTAssertEqual(block.label, expectedBlock.label, fixture.name)
                    XCTAssertEqual(block.der, expectedBlock.der, fixture.name)
                }
            case (.error(let error)?, .error(let expectedError)?):
                XCTAssertEqual(error.origin, .caller, fixture.name)
                switch (error.error, expectedError.error) {
                case (.pem(let detail)?, .pem(let expectedDetail)?):
                    XCTAssertEqual(detail.reason, expectedDetail.reason, fixture.name)
                    XCTAssertNotEqual(detail.reason, .unspecified, fixture.name)
                case (.baseEncoding(let detail)?, .baseEncoding(let expectedDetail)?):
                    XCTAssertEqual(detail.reason, expectedDetail.reason, fixture.name)
                    XCTAssertEqual(detail.reason, .baseInputTooLarge, fixture.name)
                default:
                    XCTFail("unexpected error family: \(fixture.name)")
                }
            default:
                XCTFail("unexpected response outcome: \(fixture.name)")
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
