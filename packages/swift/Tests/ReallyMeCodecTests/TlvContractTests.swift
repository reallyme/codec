// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
import SwiftProtobuf
import XCTest
@testable import ReallyMeCodec

final class TlvContractTests: XCTestCase {
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
