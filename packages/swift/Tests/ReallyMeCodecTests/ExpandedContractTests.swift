// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
import SwiftProtobuf
import XCTest
@testable import ReallyMeCodec

final class ExpandedContractTests: XCTestCase {
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

    func testGeneratedOperationsPreserveScalarAndIEEEBytes() throws {
        let codec = try self.codec()
        var bytes = ReallyMeProtoCodecBytesRequest()
        bytes.value = Data([0xfb, 0xff])
        var request = ReallyMeProtoCodecApiOperationRequest()
        request.operation = .base64Encode(bytes)
        let scalar = try ReallyMeProtoCodecApiOperationResponse(
            serializedBytes: codec.processOperation(Array(request.serializedData()))
        )
        guard case .result(let scalarResult)? = scalar.outcome,
            case .base64Encode(let text)? = scalarResult.result
        else {
            XCTFail("expected generated base64 result")
            return
        }
        XCTAssertEqual(text.value, "+/8=")
        var float = ReallyMeProtoCodecGeneralCborFloat()
        float.width = .binary16
        float.bits = 0xfe01
        var value = ReallyMeProtoCodecGeneralCborValue()
        value.value = .floatValue(float)
        var encode = ReallyMeProtoCodecGeneralCborEncodeRequest()
        encode.value = value
        request.operation = .generalCborEncode(encode)
        let cbor = try ReallyMeProtoCodecApiOperationResponse(
            serializedBytes: codec.processOperation(Array(request.serializedData()))
        )
        guard case .result(let cborResult)? = cbor.outcome,
            case .generalCborEncode(let encoded)? = cborResult.result
        else {
            XCTFail("expected generated CBOR result")
            return
        }
        XCTAssertEqual(encoded.value, Data([0xf9, 0xfe, 1]))
    }
}
