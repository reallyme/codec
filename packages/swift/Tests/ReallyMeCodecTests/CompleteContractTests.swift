// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
import SwiftProtobuf
import XCTest
@testable import ReallyMeCodec

final class CompleteContractTests: XCTestCase {
    private struct Manifest: Decodable {
        let schemaVersion: Int
        let cases: [Vector]
    }
    private struct Vector: Decodable {
        let operation: String
        let requestJson: String
        let requestHex: String
    }

    private func codec() throws -> ReallyMeCodec {
        #if REALLYME_CODEC_LINKED_FFI
            return try ReallyMeCodec()
        #else
            #if os(macOS)
                let name = "libreallyme_codec_ffi.dylib"
            #elseif os(Linux)
                let name = "libreallyme_codec_ffi.so"
            #else
                let name = "reallyme_codec_ffi.dll"
            #endif
            let path =
                ProcessInfo.processInfo.environment["REALLYME_CODEC_FFI_LIBRARY_PATH"]
                ?? URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
                .appendingPathComponent("target/debug/" + name).path
            return try ReallyMeCodec(rustCAbiLibrary: ReallyMeCodecRustCAbiLibrary(path: path))
        #endif
    }

    private func binary(_ hex: String) throws -> [UInt8] {
        guard hex.utf8.count.isMultiple(of: 2) else { throw ReallyMeCodecError.invalidInput }
        return try stride(from: 0, to: hex.utf8.count, by: 2).map { offset in
            let start = hex.index(hex.startIndex, offsetBy: offset)
            let end = hex.index(start, offsetBy: 2)
            guard let byte = UInt8(hex[start..<end], radix: 16) else { throw ReallyMeCodecError.invalidInput }
            return byte
        }
    }

    func testEverySchemaOperationPreservesBinaryAndJSONResponses() throws {
        let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let candidates = [
            root.appendingPathComponent("vectors/codec-operation-vectors.json"),
            root.appendingPathComponent("../../vectors/codec-operation-vectors.json"),
        ]
        guard let file = candidates.first(where: { FileManager.default.fileExists(atPath: $0.path) }) else {
            throw ReallyMeCodecError.invalidInput
        }
        let manifest = try JSONDecoder().decode(Manifest.self, from: Data(contentsOf: file))
        XCTAssertEqual(manifest.schemaVersion, 1)
        let codec = try self.codec()
        for vector in manifest.cases {
            var binary = try self.binary(vector.requestHex)
            var json = Array(vector.requestJson.utf8)
            var binaryResponse = try codec.processOperation(binary)
            var jsonResponse = try codec.processOperationJson(json)
            defer {
                _ = binary.withUnsafeMutableBytes { $0.initializeMemory(as: UInt8.self, repeating: 0) }
                _ = json.withUnsafeMutableBytes { $0.initializeMemory(as: UInt8.self, repeating: 0) }
                _ = binaryResponse.withUnsafeMutableBytes { $0.initializeMemory(as: UInt8.self, repeating: 0) }
                _ = jsonResponse.withUnsafeMutableBytes { $0.initializeMemory(as: UInt8.self, repeating: 0) }
            }
            let generated = try ReallyMeProtoCodecApiOperationRequest(jsonString: vector.requestJson)
            XCTAssertEqual(try generated.serializedData(), Data(binary), vector.operation)
            XCTAssertEqual(binaryResponse, jsonResponse, vector.operation)
            let response = try ReallyMeProtoCodecApiOperationResponse(serializedBytes: binaryResponse)
            guard case .result? = response.outcome else {
                XCTFail("expected successful operation: " + vector.operation)
                continue
            }
            let invalidJSON =
                String(vector.requestJson.dropLast(2)) + (vector.requestJson.hasSuffix("{}}") ? "" : ",")
                + "\"unexpected\":true}}"
            let invalidResponse = try ReallyMeProtoCodecApiOperationResponse(
                serializedBytes: codec.processOperationJson(Array(invalidJSON.utf8))
            )
            guard case .error(let error)? = invalidResponse.outcome,
                case .boundary(let boundary)? = error.error
            else {
                XCTFail("expected typed boundary error")
                continue
            }
            XCTAssertEqual(error.origin, .caller)
            XCTAssertEqual(boundary.reason, .boundaryMalformedJson)
        }
    }
}
