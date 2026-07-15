// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import Foundation
import ReallyMeCodec
import ReallyMeCodecProto
import XCTest

final class ReallyMeCodecTests: XCTestCase {
    private static func libraryPath() throws -> String {
        if let value = ProcessInfo.processInfo.environment["REALLYME_CODEC_FFI_LIBRARY_PATH"],
           !value.isEmpty {
            return value
        }

        #if os(macOS)
            let libraryName = "libreallyme_codec_ffi.dylib"
        #elseif os(Linux)
            let libraryName = "libreallyme_codec_ffi.so"
        #else
            let libraryName = "reallyme_codec_ffi.dll"
        #endif
        let localPath = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
            .appendingPathComponent("target/debug")
            .appendingPathComponent(libraryName)
            .path
        return try XCTUnwrap(
            FileManager.default.fileExists(atPath: localPath) ? localPath : nil,
            "REALLYME_CODEC_FFI_LIBRARY_PATH or target/debug/\(libraryName) must point to a built reallyme-codec-ffi library"
        )
    }

    private static func configuredCodec() throws -> ReallyMeCodec {
        try ReallyMeCodec(rustCAbiLibrary: ReallyMeCodecRustCAbiLibrary(path: libraryPath()))
    }

    private static func jsonObject(_ text: String) throws -> [String: Any] {
        let data = Data(text.utf8)
        return try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
    }

    private static func assertCodecError(
        _ expected: ReallyMeCodecError,
        _ operation: @autoclosure () throws -> Any
    ) {
        XCTAssertThrowsError(try operation()) { error in
            XCTAssertEqual(error as? ReallyMeCodecError, expected)
        }
    }

    func testTemporaryLibraryPatternRetainsLoadedImage() throws {
        let codec = try ReallyMeCodec(
            rustCAbiLibrary: ReallyMeCodecRustCAbiLibrary(path: Self.libraryPath())
        )

        for _ in 0..<32 {
            XCTAssertEqual(try codec.base64urlEncode([1, 2, 3]), "AQID")
        }
    }

    func testBaseEncodingsHandleEmptyLargeAndInvalidInput() throws {
        let codec = try Self.configuredCodec()
        let empty: [UInt8] = []
        let large = (0..<4096).map { UInt8($0 % 251) }

        XCTAssertEqual(try codec.base64Encode(empty), "")
        XCTAssertEqual(try codec.base64Decode(""), empty)
        XCTAssertEqual(try codec.base64urlEncode(empty), "")
        XCTAssertEqual(try codec.base64urlDecode(""), empty)
        XCTAssertEqual(try codec.bytesToLowerHex(empty), "")
        XCTAssertEqual(try codec.lowerHexToBytes(""), empty)

        let largeBase64url = try codec.base64urlEncode(large)
        XCTAssertEqual(try codec.base64urlDecode(largeBase64url), large)
        XCTAssertEqual(try codec.lowerHexToBytes(try codec.bytesToLowerHex(large)), large)

        Self.assertCodecError(.invalidInput, try codec.base64Decode("Zh=="))
        Self.assertCodecError(.invalidInput, try codec.base64Decode("AAEC-_8="))
        Self.assertCodecError(.invalidInput, try codec.base64urlDecode("AAEC-_8="))
        Self.assertCodecError(.invalidInput, try codec.lowerHexToBytes("DEADBEEF"))
    }

    func testMultibaseMulticodecAndMultikeyUseRustProvider() throws {
        let codec = try Self.configuredCodec()
        var publicKey = [UInt8](repeating: 0, count: 32)
        publicKey[31] = 7

        let base58 = try codec.base58btcEncode(publicKey)
        XCTAssertEqual(try codec.base58btcDecode(base58), publicKey)
        XCTAssertEqual(try codec.base58btcDecode(""), [])
        let multibase58 = try codec.multibaseBase58btcEncode(publicKey)
        XCTAssertTrue(multibase58.hasPrefix("z"))
        XCTAssertEqual(try codec.multibaseDecode(multibase58), publicKey)
        let multibase64url = try codec.multibaseBase64urlEncode(publicKey)
        XCTAssertTrue(multibase64url.hasPrefix("u"))
        XCTAssertEqual(try codec.multibaseDecode(multibase64url), publicKey)
        XCTAssertEqual(try codec.multibaseBase64urlEncode([]), "u")
        XCTAssertEqual(try codec.multibaseDecode("u"), [])
        let oversizedBase58Input = [UInt8](repeating: 0, count: 8 * 1024 + 1)
        Self.assertCodecError(.invalidInput, try codec.base58btcEncode(oversizedBase58Input))
        Self.assertCodecError(.invalidInput, try codec.multibaseBase58btcEncode(oversizedBase58Input))

        let metadataJson = try Self.jsonObject(try codec.multicodecPrefixForName("ed25519-pub"))
        XCTAssertEqual(metadataJson["name"] as? String, "ed25519-pub")
        XCTAssertEqual(metadataJson["tag"] as? String, "key")
        let metadataProto = try ReallyMeProtoCodecMulticodecSpec(
            serializedBytes: try codec.multicodecPrefixForNameProto("ed25519-pub")
        )
        let metadataProtoResult = try codec.multicodecPrefixForNameProtoResult("ed25519-pub")
        XCTAssertEqual(metadataProtoResult.status, .result)
        XCTAssertFalse(metadataProtoResult.isCodecError)
        XCTAssertEqual(
            try ReallyMeProtoCodecMulticodecSpec(serializedBytes: metadataProtoResult.bytes).name,
            "ed25519-pub"
        )
        XCTAssertEqual(metadataProto.name, "ed25519-pub")
        XCTAssertEqual(metadataProto.algorithmName, "Ed25519")
        XCTAssertEqual(metadataProto.fixedLength, 32)
        Self.assertCodecError(.invalidInput, try codec.multicodecPrefixForNameProto("not-a-codec"))
        let metadataErrorResult = try codec.multicodecPrefixForNameProtoResult("not-a-codec")
        XCTAssertEqual(metadataErrorResult.status, .codecError)

        let prefixed = Array(metadataProto.prefix) + publicKey
        let lookupJson = try Self.jsonObject(try codec.multicodecLookupPrefix(prefixed))
        XCTAssertEqual(lookupJson["name"] as? String, "ed25519-pub")
        let lookupProto = try ReallyMeProtoCodecMulticodecLookupResult(
            serializedBytes: try codec.multicodecLookupPrefixProto(prefixed)
        )
        XCTAssertEqual(try codec.multicodecLookupPrefixProtoResult(prefixed).status, .result)
        XCTAssertEqual(lookupProto.name, "ed25519-pub")
        Self.assertCodecError(.invalidInput, try codec.multicodecLookupPrefixProto([0, 0, 7]))
        let lookupErrorResult = try codec.multicodecLookupPrefixProtoResult([0, 0, 7])
        XCTAssertEqual(lookupErrorResult.status, .codecError)
        XCTAssertEqual(try codec.multicodecStripPrefix(prefixed), publicKey)
        XCTAssertTrue(try codec.multicodecTable().contains("mlkem-1024-pub"))
        let tableProto = try ReallyMeProtoCodecMulticodecTableResult(
            serializedBytes: try codec.multicodecTableProto()
        )
        XCTAssertEqual(try codec.multicodecTableProtoResult().status, .result)
        XCTAssertTrue(tableProto.entries.contains { $0.name == "mlkem-1024-pub" })

        let multikey = try codec.multikeyEncode(codecName: "ed25519-pub", publicKey: publicKey)
        let parsedJson = try Self.jsonObject(try codec.multikeyParse(multikey))
        XCTAssertEqual(parsedJson["codecName"] as? String, "ed25519-pub")
        let parsedProto = try ReallyMeProtoCodecMultikeyParseResult(
            serializedBytes: try codec.multikeyParseProto(multikey)
        )
        let parsedProtoResult = try codec.multikeyParseProtoResult(multikey)
        XCTAssertEqual(parsedProtoResult.status, .result)
        XCTAssertEqual(
            try ReallyMeProtoCodecMultikeyParseResult(serializedBytes: parsedProtoResult.bytes).codecName,
            "ed25519-pub"
        )
        XCTAssertEqual(parsedProto.codecName, "ed25519-pub")
        XCTAssertEqual(parsedProto.algorithmName, "Ed25519")
        XCTAssertEqual(Array(parsedProto.publicKey), publicKey)
        XCTAssertTrue(try codec.bindingTypeMatchesCodec(bindingType: "Multikey", codecName: "ed25519-pub"))
        try codec.requireSupportedMulticodec("ed25519-pub")
        try codec.validateKeyBinding(bindingType: "Multikey", algorithm: nil, multikey: multikey)

        Self.assertCodecError(.invalidInput, try codec.requireSupportedMulticodec("not-a-codec"))
        Self.assertCodecError(
            .invalidInput,
            try codec.validateKeyBinding(bindingType: "P256Key2024", algorithm: "P-256", multikey: multikey)
        )

        Self.assertCodecError(.invalidInput, try codec.multikeyParseProto("not-a-key"))
        let multikeyErrorResult = try codec.multikeyParseProtoResult("not-a-key")
        XCTAssertEqual(multikeyErrorResult.status, .codecError)
        XCTAssertTrue(multikeyErrorResult.isCodecError)
        let multikeyError = try ReallyMeProtoCodecError(serializedBytes: multikeyErrorResult.bytes)
        XCTAssertNotNil(multikeyError.error)
        guard case .multiformat(let multiformatError)? = multikeyError.error else {
            XCTFail("expected multiformat error envelope")
            return
        }
        XCTAssertEqual(multiformatError.reason, .multiformatInvalidMultikey)
    }

    func testDagCborCidAndJcsOperationsUseRustProvider() throws {
        let codec = try Self.configuredCodec()
        let taggedJson = """
        {"type":"map","value":[{"key":"b","value":{"type":"int","value":2}},{"key":"a","value":{"type":"string","value":"one"}},{"key":"bytes","value":{"type":"bytes","value":"AAEC"}}]}
        """

        let encoded = try codec.dagCborEncode(taggedJson: taggedJson)
        XCTAssertFalse(encoded.isEmpty)
        let decodedJson = try Self.jsonObject(try codec.dagCborDecode(encoded))
        XCTAssertEqual(decodedJson["type"] as? String, "map")

        let cid = try codec.dagCborComputeCid(encoded)
        XCTAssertTrue(try codec.isValidCidString(cid))
        XCTAssertFalse(try codec.isValidCidString("not-a-cid"))
        XCTAssertEqual(try codec.tryParseCid(cid), cid)
        XCTAssertNil(try codec.tryParseCid("not-a-cid"))

        let verificationJson = try Self.jsonObject(try codec.dagCborVerifyCid(cid: cid, bytes: encoded))
        XCTAssertEqual(verificationJson["valid"] as? Bool, true)
        let verificationProto = try ReallyMeProtoCodecDagCborVerifyCidResult(
            serializedBytes: try codec.dagCborVerifyCidProto(cid: cid, bytes: encoded)
        )
        XCTAssertEqual(try codec.dagCborVerifyCidProtoResult(cid: cid, bytes: encoded).status, .result)
        XCTAssertTrue(verificationProto.valid)
        XCTAssertEqual(verificationProto.expectedCid, cid)

        let invalidUpperPayloadCid = String(cid.prefix(1)) + cid.dropFirst().uppercased()
        let invalidVerification = try ReallyMeProtoCodecDagCborVerifyCidResult(
            serializedBytes: try codec.dagCborVerifyCidProto(cid: invalidUpperPayloadCid, bytes: encoded)
        )
        XCTAssertFalse(invalidVerification.valid)
        XCTAssertEqual(invalidVerification.actualCid, "")
        let emptyCidVerificationJson = try Self.jsonObject(try codec.dagCborVerifyCid(cid: "", bytes: encoded))
        XCTAssertEqual(emptyCidVerificationJson["valid"] as? Bool, false)
        XCTAssertEqual(emptyCidVerificationJson["expectedCid"] as? String, cid)
        XCTAssertEqual(emptyCidVerificationJson["actualCid"] as? String, "")
        let emptyCidVerificationProto = try ReallyMeProtoCodecDagCborVerifyCidResult(
            serializedBytes: try codec.dagCborVerifyCidProto(cid: "", bytes: encoded)
        )
        XCTAssertFalse(emptyCidVerificationProto.valid)
        XCTAssertEqual(try codec.dagCborVerifyCidProtoResult(cid: "", bytes: encoded).status, .result)

        XCTAssertEqual(try codec.dagCborSha256ContentHash(encoded).count, 32)
        XCTAssertGreaterThan(try codec.dagCborMultihash(encoded).count, 32)
        XCTAssertEqual(try codec.dagCborCodecCode(), 0x71)
        Self.assertCodecError(.invalidInput, try codec.dagCborDecode([0xa2, 0x61, 0x62, 0x01, 0x61, 0x61, 0x02]))
        let oversizedCbor = [UInt8](repeating: 0, count: 1024 * 1024 + 1)
        Self.assertCodecError(.invalidInput, try codec.dagCborDecode(oversizedCbor))
        Self.assertCodecError(.invalidInput, try codec.dagCborComputeCid(oversizedCbor))
        Self.assertCodecError(.invalidInput, try codec.dagCborVerifyCid(cid: cid, bytes: oversizedCbor))
        Self.assertCodecError(.invalidInput, try codec.dagCborVerifyCidProto(cid: cid, bytes: oversizedCbor))
        XCTAssertEqual(try codec.dagCborVerifyCidProtoResult(cid: cid, bytes: oversizedCbor).status, .codecError)
        Self.assertCodecError(.invalidInput, try codec.dagCborSha256ContentHash(oversizedCbor))
        Self.assertCodecError(.invalidInput, try codec.dagCborMultihash(oversizedCbor))

        XCTAssertEqual(try codec.canonicalizeJson("{\"b\":2,\"a\":1}"), "{\"a\":1,\"b\":2}")
        XCTAssertEqual(try codec.canonicalizeJson("333333333.33333329"), "333333333.3333333")
        Self.assertCodecError(.invalidInput, try codec.canonicalizeJson("{"))
    }

    func testPemRoundTripAndProtoErrorsUseRustProvider() throws {
        let codec = try Self.configuredCodec()
        let der: [UInt8] = [0x30, 0x03, 0x02, 0x01, 0x01]
        let pem = try codec.encodePem(label: "PRIVATE KEY", der: der)

        XCTAssertTrue(pem.contains("-----BEGIN PRIVATE KEY-----"))
        let decodedJson = try Self.jsonObject(try codec.decodePem(pem))
        XCTAssertEqual(decodedJson["label"] as? String, "PRIVATE KEY")
        XCTAssertEqual(decodedJson["der"] as? String, "MAMCAQE")

        let decodedProto = try ReallyMeProtoCodecPemDecodeResult(
            serializedBytes: try codec.decodePemProto(pem)
        )
        let decodedProtoResult = try codec.decodePemProtoResult(pem)
        XCTAssertEqual(decodedProtoResult.status, .result)
        XCTAssertEqual(
            try ReallyMeProtoCodecPemDecodeResult(serializedBytes: decodedProtoResult.bytes).label,
            "PRIVATE KEY"
        )
        XCTAssertEqual(decodedProto.label, "PRIVATE KEY")
        XCTAssertEqual(Array(decodedProto.der), der)

        let wrapped = try codec.encodePem(label: "PUBLIC KEY", der: Array("not real der".utf8), optionsJson: "{\"lineWidth\":4}")
        XCTAssertTrue(wrapped.contains("bm90\nIHJl\nYWwg\nZGVy"))

        Self.assertCodecError(.invalidInput, try codec.encodePem(label: "CERTIFICATE", der: der))
        Self.assertCodecError(
            .invalidInput,
            try codec.decodePem(pem, optionsJson: "{\"allowedLabels\":[\"PUBLIC KEY\"]}")
        )

        Self.assertCodecError(
            .invalidInput,
            try codec.decodePemProto(pem, optionsJson: "{\"allowedLabels\":[\"PUBLIC KEY\"]}")
        )
        let pemErrorResult = try codec.decodePemProtoResult(pem, optionsJson: "{\"allowedLabels\":[\"PUBLIC KEY\"]}")
        XCTAssertEqual(pemErrorResult.status, .codecError)
        let pemError = try ReallyMeProtoCodecError(serializedBytes: pemErrorResult.bytes)
        XCTAssertNotNil(pemError.error)
        guard case .pem(let error)? = pemError.error else {
            XCTFail("expected PEM error envelope")
            return
        }
        XCTAssertEqual(error.reason, .pemUnsupportedLabel)
    }

    func testProviderLoadingFailsClosed() throws {
        Self.assertCodecError(
            .dynamicLibraryNotFound,
            try ReallyMeCodecRustCAbiLibrary(path: "/tmp/reallyme-codec-missing-library.dylib")
        )
    }
}
