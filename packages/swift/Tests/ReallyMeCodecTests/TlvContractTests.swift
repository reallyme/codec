// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
import SwiftProtobuf
import XCTest
@testable import ReallyMeCodec

final class TlvContractTests: XCTestCase {
    private struct Manifest: Decodable {
        let schemaVersion: Int
        let cases: [Vector]
    }
    private struct Vector: Decodable {
        let name: String
        let operation: String
        let requestJson: String
        let requestHex: String
        let responseHex: String
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

    private func bytes(_ hex: String) throws -> [UInt8] {
        guard hex.utf8.count.isMultiple(of: 2) else { throw ReallyMeCodecError.invalidInput }
        return try stride(from: 0, to: hex.utf8.count, by: 2).map { offset in
            let start = hex.index(hex.startIndex, offsetBy: offset)
            let end = hex.index(start, offsetBy: 2)
            guard let byte = UInt8(hex[start..<end], radix: 16) else { throw ReallyMeCodecError.invalidInput }
            return byte
        }
    }

    func testTlvReceiptsAndTypedFailuresMatchIndependentWireGoldens() throws {
        let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let candidates = [
            root.appendingPathComponent("vectors/tlv-framing-vectors.json"),
            root.appendingPathComponent("../../vectors/tlv-framing-vectors.json"),
        ]
        guard let file = candidates.first(where: { FileManager.default.fileExists(atPath: $0.path) }) else {
            throw ReallyMeCodecError.invalidInput
        }
        let manifest = try JSONDecoder().decode(Manifest.self, from: Data(contentsOf: file))
        XCTAssertEqual(manifest.schemaVersion, 1)
        let codec = try self.codec()
        for vector in manifest.cases {
            var binary = try bytes(vector.requestHex)
            var json = Array(vector.requestJson.utf8)
            var expected = try bytes(vector.responseHex)
            var binaryResponse = try codec.processOperation(binary)
            var jsonResponse = try codec.processOperationJson(json)
            defer {
                _ = binary.withUnsafeMutableBytes { $0.initializeMemory(as: UInt8.self, repeating: 0) }
                _ = json.withUnsafeMutableBytes { $0.initializeMemory(as: UInt8.self, repeating: 0) }
                _ = expected.withUnsafeMutableBytes { $0.initializeMemory(as: UInt8.self, repeating: 0) }
                _ = binaryResponse.withUnsafeMutableBytes { $0.initializeMemory(as: UInt8.self, repeating: 0) }
                _ = jsonResponse.withUnsafeMutableBytes { $0.initializeMemory(as: UInt8.self, repeating: 0) }
            }
            let label = vector.name + " " + vector.operation
            let request = try ReallyMeProtoCodecApiOperationRequest(jsonString: vector.requestJson)
            XCTAssertEqual(try request.serializedData(), Data(binary), label)
            XCTAssertEqual(binaryResponse, expected, label)
            XCTAssertEqual(jsonResponse, expected, label)
            let response = try ReallyMeProtoCodecApiOperationResponse(serializedBytes: expected)
            XCTAssertEqual(try response.serializedData(), Data(expected), label)
        }
    }

    func testTlvAcceptsAnExplicitSixteenMiBSourceThroughTheNativeProvider() throws {
        let maximum = 16 * 1024 * 1024
        var source = ReallyMeProtoCodecTlvSourceRequest()
        source.encoded = Data(count: maximum)
        source.encoded[0] = 4
        source.options.profile = .iso7816DefiniteBer
        source.options.limits.inputBytes = UInt64(maximum)
        var request = ReallyMeProtoCodecApiOperationRequest()
        request.tlvReadHeader = source

        let codec = try self.codec()
        let binary = try codec.processOperation(Array(request.serializedData()))
        let json = try codec.processOperationJson(Array(request.jsonUTF8Data()))
        XCTAssertEqual(binary, json)
        let response = try ReallyMeProtoCodecApiOperationResponse(serializedBytes: binary)
        XCTAssertEqual(response.result.tlvReadHeader.consumedBytes, 2)
        XCTAssertEqual(response.result.tlvReadHeader.declaredValueLength, 0)
    }
}
