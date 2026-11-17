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
    func testGeneratedPEMBundlePreservesBytesAndResourceFailure() throws {
        let codec = try self.codec()
        let block = "-----BEGIN CERTIFICATE-----\rAQID\r-----END CERTIFICATE-----\r"
        var pem = ReallyMeProtoCodecPemBundleDecodeRequest()
        pem.pem = Data((block + block).utf8)
        var request = ReallyMeProtoCodecApiOperationRequest()
        request.operation = .pemBundleDecode(pem)
        let response = try ReallyMeProtoCodecApiOperationResponse(
            serializedBytes: codec.processOperation(Array(request.serializedData()))
        )
        guard case .result(let result)? = response.outcome,
            case .pemBundleDecode(let bundle)? = result.result
        else { XCTFail("expected PEM bundle result"); return }
        XCTAssertEqual(bundle.blocks.count, 2)
        XCTAssertEqual(bundle.blocks[0].label, "CERTIFICATE")
        XCTAssertEqual(bundle.blocks[0].der, Data([1, 2, 3]))
        var options = ReallyMeProtoCodecPemBundleDecodeOptions()
        options.maxBlocks = 1
        pem.options = options
        request.operation = .pemBundleDecode(pem)
        let rejected = try ReallyMeProtoCodecApiOperationResponse(
            serializedBytes: codec.processOperation(Array(request.serializedData()))
        )
        guard case .error(let error)? = rejected.outcome,
            case .pem(let reason)? = error.error
        else { XCTFail("expected typed PEM resource failure"); return }
        XCTAssertEqual(reason.reason, .pemResourceLimitExceeded)
    }

    func testGeneratedGuardedJSONPreservesUnicodeAndDuplicateReason() throws {
        let codec = try self.codec()
        var json = ReallyMeProtoCodecJsonDecodeRequest()
        json.encoded = Data(#"{"hello":"world\u0021"}"#.utf8)
        var request = ReallyMeProtoCodecApiOperationRequest()
        request.operation = .jsonDecode(json)
        let response = try ReallyMeProtoCodecApiOperationResponse(
            serializedBytes: codec.processOperation(Array(request.serializedData()))
        )
        guard case .result(let result)? = response.outcome,
            case .jsonDecode(let document)? = result.result,
            case .objectValue(let object)? = document.value.kind,
            let member = object.members.first,
            case .stringValue(let text)? = member.value.kind
        else {
            XCTFail("expected generated JSON result")
            return
        }
        XCTAssertEqual(member.name, "hello")
        XCTAssertEqual(text, "world!")
        json.encoded = Data(#"{"x":1,"\u0078":2}"#.utf8)
        request.operation = .jsonDecode(json)
        let rejected = try ReallyMeProtoCodecApiOperationResponse(
            serializedBytes: codec.processOperation(Array(request.serializedData()))
        )
        guard case .error(let error)? = rejected.outcome,
            case .canonicalization(let detail)? = error.error
        else {
            XCTFail("expected generated JSON failure")
            return
        }
        XCTAssertEqual(detail.reason, .canonicalJsonDuplicateProperty)
    }
}
