// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import Foundation
@testable import ReallyMeCodec
import ReallyMeCodecProto
import XCTest

private struct CodecVectorManifest: Decodable {
    let schemaVersion: Int
    let vectors: CodecVectors
}

private struct CodecVectors: Decodable {
    let baseInputHex: String
    let base64Padded: String
    let base64MissingPadding: String
    let base64NonCanonicalTrailingBits: String
    let base64urlUnpadded: String
    let base64urlPadded: String
    let base64urlNonCanonicalTrailingBits: String
    let lowerHex: String
    let base58btcEncoded: String
    let publicKeyHex: String
    let ed25519CodecName: String
    let ed25519AlgorithmName: String
    let ed25519Tag: String
    let ed25519KeyMaterial: String
    let ed25519ExpectedKeyLength: Int
    let ed25519PrefixHex: String
    let ed25519PrefixedPublicKeyHex: String
    let publicKeyBase58btc: String
    let publicKeyMultibaseBase58btc: String
    let publicKeyMultibaseBase64url: String
    let unsupportedMultibase: String
    let ed25519Multikey: String
    let nonCanonicalBase64urlMultikey: String
    let multikeyBindingType: String
    let mismatchedBindingType: String
    let mismatchedBindingAlgorithm: String
    let multicodecTableRequiredName: String
    let dagCborTaggedJson: String
    let dagCborCanonicalTaggedJson: String
    let dagCborEncodedHex: String
    let dagCborNonCanonicalIntegerHex: String
    let dagCborDuplicateKeyHex: String
    let dagCborOutOfOrderKeyHex: String
    let dagCborCid: String
    let dagCborSha256Hex: String
    let dagCborMultihashHex: String
    let dagCborCodecCode: UInt32
    let invalidCid: String
    let jcsObjectInputJson: String
    let jcsObjectCanonicalJson: String
    let jcsNumberInputJson: String
    let jcsNumberCanonicalJson: String
    let jcsDuplicateMemberJson: String
    let jcsNonInteroperableIntegerJson: String
    let jcsLoneSurrogateJson: String
    let pemPrivateLabel: String
    let pemPrivateDerHex: String
    let pemPrivatePem: String
    let pemPublicLabel: String
    let pemWrappedDerText: String
    let pemWrappedPem: String
    let pemLineWidthOptionsJson: String
    let protoMulticodecTableRequestHex: String
    let protoMulticodecTableRequestJson: String
}

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

    private static func codecVectors() throws -> CodecVectors {
        let currentDirectory = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let candidates = [
            currentDirectory.appendingPathComponent("test-vectors/codec-vectors.json"),
            currentDirectory.appendingPathComponent("../../test-vectors/codec-vectors.json"),
        ]
        for candidate in candidates where FileManager.default.fileExists(atPath: candidate.path) {
            let data = try Data(contentsOf: candidate)
            let manifest = try JSONDecoder().decode(CodecVectorManifest.self, from: data)
            guard manifest.schemaVersion == 2 else {
                throw ReallyMeCodecError.invalidInput
            }
            return manifest.vectors
        }
        throw ReallyMeCodecError.invalidInput
    }

    private static func hexBytes(_ text: String) throws -> [UInt8] {
        guard text.count.isMultiple(of: 2) else {
            throw ReallyMeCodecError.invalidInput
        }
        var bytes: [UInt8] = []
        bytes.reserveCapacity(text.count / 2)
        var index = text.startIndex
        while index < text.endIndex {
            let next = text.index(index, offsetBy: 2)
            guard let byte = UInt8(text[index..<next], radix: 16) else {
                throw ReallyMeCodecError.invalidInput
            }
            bytes.append(byte)
            index = next
        }
        return bytes
    }

    private static func hexString(_ bytes: [UInt8]) -> String {
        bytes.map { String(format: "%02x", $0) }.joined()
    }

    private static func jsonObject(_ text: String) throws -> [String: Any] {
        let data = Data(text.utf8)
        return try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
    }

    private static func jsonObject(_ bytes: [UInt8]) throws -> [String: Any] {
        try XCTUnwrap(JSONSerialization.jsonObject(with: Data(bytes)) as? [String: Any])
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

    func testManagedBoundariesRejectOversizedInputsBeforeSerialization() throws {
        let codec = try Self.configuredCodec()
        let oversizedText = String(repeating: "a", count: 1_048_577)

        Self.assertCodecError(.invalidInput, try codec.base64Decode(oversizedText))
        Self.assertCodecError(.invalidInput, try codec.canonicalizeJson(oversizedText))
        Self.assertCodecError(
            .invalidInput,
            try codec.multicodecPrefixForNameProto(oversizedText)
        )

        let result = try codec.multicodecPrefixForNameProtoResult(oversizedText)
        XCTAssertEqual(result.status, .codecError)
        let error = try ReallyMeProtoCodecError(serializedBytes: result.bytes)
        XCTAssertEqual(error.boundary.reason, .boundaryResourceLimitExceeded)
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

    func testSharedVectorSuiteCoversSwiftPublicMethods() throws {
        let codec = try Self.configuredCodec()
        let vectors = try Self.codecVectors()
        let baseInput = try Self.hexBytes(vectors.baseInputHex)

        XCTAssertEqual(try codec.base64Encode(baseInput), vectors.base64Padded)
        XCTAssertEqual(try codec.base64Decode(vectors.base64Padded), baseInput)
        XCTAssertEqual(try codec.base64urlEncode(baseInput), vectors.base64urlUnpadded)
        XCTAssertEqual(try codec.base64urlDecode(vectors.base64urlUnpadded), baseInput)
        XCTAssertEqual(try codec.bytesToLowerHex(baseInput), vectors.lowerHex)
        XCTAssertEqual(try codec.lowerHexToBytes(vectors.lowerHex), baseInput)
        XCTAssertEqual(try codec.base58btcEncode(baseInput), vectors.base58btcEncoded)
        XCTAssertEqual(try codec.base58btcDecode(vectors.base58btcEncoded), baseInput)

        let publicKey = try Self.hexBytes(vectors.publicKeyHex)
        let prefixedPublicKey = try Self.hexBytes(vectors.ed25519PrefixedPublicKeyHex)
        XCTAssertEqual(try codec.base58btcEncode(publicKey), vectors.publicKeyBase58btc)
        XCTAssertEqual(try codec.multibaseBase58btcEncode(publicKey), vectors.publicKeyMultibaseBase58btc)
        XCTAssertEqual(try codec.multibaseBase64urlEncode(publicKey), vectors.publicKeyMultibaseBase64url)
        XCTAssertEqual(try codec.multibaseDecode(vectors.publicKeyMultibaseBase58btc), publicKey)
        XCTAssertEqual(try codec.multibaseDecode(vectors.publicKeyMultibaseBase64url), publicKey)

        let metadataJson = try Self.jsonObject(try codec.multicodecPrefixForName(vectors.ed25519CodecName))
        XCTAssertEqual(metadataJson["name"] as? String, vectors.ed25519CodecName)
        XCTAssertEqual(metadataJson["tag"] as? String, vectors.ed25519Tag)
        let metadataProto = try ReallyMeProtoCodecMulticodecSpec(
            serializedBytes: try codec.multicodecPrefixForNameProto(vectors.ed25519CodecName)
        )
        XCTAssertEqual(metadataProto.name, vectors.ed25519CodecName)
        XCTAssertEqual(metadataProto.algorithmName, vectors.ed25519AlgorithmName)
        XCTAssertEqual(Self.hexString(Array(metadataProto.prefix)), vectors.ed25519PrefixHex)
        let metadataProtoResult = try codec.multicodecPrefixForNameProtoResult(vectors.ed25519CodecName)
        XCTAssertEqual(metadataProtoResult.status, .result)
        XCTAssertEqual(
            try ReallyMeProtoCodecMulticodecSpec(serializedBytes: metadataProtoResult.bytes).name,
            vectors.ed25519CodecName
        )

        let lookupJson = try Self.jsonObject(try codec.multicodecLookupPrefix(prefixedPublicKey))
        XCTAssertEqual(lookupJson["name"] as? String, vectors.ed25519CodecName)
        let lookupProto = try ReallyMeProtoCodecMulticodecLookupResult(
            serializedBytes: try codec.multicodecLookupPrefixProto(prefixedPublicKey)
        )
        XCTAssertEqual(lookupProto.name, vectors.ed25519CodecName)
        XCTAssertEqual(try codec.multicodecLookupPrefixProtoResult(prefixedPublicKey).status, .result)
        XCTAssertEqual(try codec.multicodecStripPrefix(prefixedPublicKey), publicKey)
        XCTAssertTrue(try codec.multicodecTable().contains(vectors.multicodecTableRequiredName))
        let tableProto = try ReallyMeProtoCodecMulticodecTableResult(
            serializedBytes: try codec.multicodecTableProto()
        )
        XCTAssertTrue(tableProto.entries.contains { $0.name == vectors.multicodecTableRequiredName })
        XCTAssertEqual(try codec.multicodecTableProtoResult().status, .result)

        XCTAssertEqual(
            try codec.multikeyEncode(codecName: vectors.ed25519CodecName, publicKey: publicKey),
            vectors.ed25519Multikey
        )
        let parsedJson = try Self.jsonObject(try codec.multikeyParse(vectors.ed25519Multikey))
        XCTAssertEqual(parsedJson["codecName"] as? String, vectors.ed25519CodecName)
        let parsedProto = try ReallyMeProtoCodecMultikeyParseResult(
            serializedBytes: try codec.multikeyParseProto(vectors.ed25519Multikey)
        )
        XCTAssertEqual(parsedProto.codecName, vectors.ed25519CodecName)
        XCTAssertEqual(Array(parsedProto.publicKey), publicKey)
        XCTAssertEqual(try codec.multikeyParseProtoResult(vectors.ed25519Multikey).status, .result)
        XCTAssertTrue(
            try codec.bindingTypeMatchesCodec(
                bindingType: vectors.multikeyBindingType,
                codecName: vectors.ed25519CodecName
            )
        )
        try codec.requireSupportedMulticodec(vectors.ed25519CodecName)
        try codec.validateKeyBinding(
            bindingType: vectors.multikeyBindingType,
            algorithm: nil,
            multikey: vectors.ed25519Multikey
        )
        Self.assertCodecError(
            .invalidInput,
            try codec.validateKeyBinding(
                bindingType: vectors.mismatchedBindingType,
                algorithm: vectors.mismatchedBindingAlgorithm,
                multikey: vectors.ed25519Multikey
            )
        )

        let dagCborBytes = try codec.dagCborEncode(taggedJson: vectors.dagCborTaggedJson)
        XCTAssertEqual(Self.hexString(dagCborBytes), vectors.dagCborEncodedHex)
        XCTAssertEqual(try codec.dagCborDecode(dagCborBytes), vectors.dagCborCanonicalTaggedJson)
        XCTAssertEqual(try codec.dagCborComputeCid(dagCborBytes), vectors.dagCborCid)
        XCTAssertEqual(try codec.dagCborVerifyCid(cid: vectors.dagCborCid, bytes: dagCborBytes).contains("\"valid\":true"), true)
        let verificationProto = try ReallyMeProtoCodecDagCborVerifyCidResult(
            serializedBytes: try codec.dagCborVerifyCidProto(cid: vectors.dagCborCid, bytes: dagCborBytes)
        )
        XCTAssertTrue(verificationProto.valid)
        XCTAssertEqual(try codec.dagCborVerifyCidProtoResult(cid: vectors.dagCborCid, bytes: dagCborBytes).status, .result)
        XCTAssertEqual(Self.hexString(try codec.dagCborSha256ContentHash(dagCborBytes)), vectors.dagCborSha256Hex)
        XCTAssertEqual(Self.hexString(try codec.dagCborMultihash(dagCborBytes)), vectors.dagCborMultihashHex)
        XCTAssertEqual(try codec.dagCborCodecCode(), vectors.dagCborCodecCode)
        XCTAssertTrue(try codec.isValidCidString(vectors.dagCborCid))
        XCTAssertFalse(try codec.isValidCidString(vectors.invalidCid))
        XCTAssertEqual(try codec.tryParseCid(vectors.dagCborCid), vectors.dagCborCid)
        XCTAssertNil(try codec.tryParseCid(vectors.invalidCid))

        XCTAssertEqual(try codec.canonicalizeJson(vectors.jcsObjectInputJson), vectors.jcsObjectCanonicalJson)
        XCTAssertEqual(try codec.canonicalizeJson(vectors.jcsNumberInputJson), vectors.jcsNumberCanonicalJson)

        let privateDer = try Self.hexBytes(vectors.pemPrivateDerHex)
        XCTAssertEqual(
            try codec.encodePem(label: vectors.pemPrivateLabel, der: privateDer),
            Array(vectors.pemPrivatePem.utf8)
        )
        let decodedPemJson = try Self.jsonObject(try codec.decodePem(Array(vectors.pemPrivatePem.utf8)))
        XCTAssertEqual(decodedPemJson["label"] as? String, vectors.pemPrivateLabel)
        XCTAssertEqual(
            try codec.encodePem(
                label: vectors.pemPublicLabel,
                der: Array(vectors.pemWrappedDerText.utf8),
                optionsJson: vectors.pemLineWidthOptionsJson
            ),
            Array(vectors.pemWrappedPem.utf8)
        )

        let binaryEnvelope = try codec.processProto(
            Self.hexBytes(vectors.protoMulticodecTableRequestHex)
        )
        let jsonEnvelope = try codec.processProtoJson(
            Array(vectors.protoMulticodecTableRequestJson.utf8)
        )
        XCTAssertEqual(binaryEnvelope, jsonEnvelope)
        let decodedEnvelope = try ReallyMeProtoCodecProtoResultEnvelope(serializedBytes: binaryEnvelope)
        XCTAssertEqual(decodedEnvelope.status, .result)
        let decodedTable = try ReallyMeProtoCodecMulticodecTableResult(
            serializedBytes: decodedEnvelope.payload
        )
        XCTAssertTrue(decodedTable.entries.contains { $0.name == vectors.multicodecTableRequiredName })
    }

    func testSharedVectorSuiteRejectsNonCanonicalInputs() throws {
        let codec = try Self.configuredCodec()
        let vectors = try Self.codecVectors()

        Self.assertCodecError(.invalidInput, try codec.base64Decode(vectors.base64MissingPadding))
        Self.assertCodecError(
            .invalidInput,
            try codec.base64Decode(vectors.base64NonCanonicalTrailingBits)
        )
        Self.assertCodecError(.invalidInput, try codec.base64urlDecode(vectors.base64urlPadded))
        Self.assertCodecError(
            .invalidInput,
            try codec.base64urlDecode(vectors.base64urlNonCanonicalTrailingBits)
        )
        Self.assertCodecError(.invalidInput, try codec.multibaseDecode(vectors.unsupportedMultibase))
        Self.assertCodecError(
            .invalidInput,
            try codec.multikeyParse(vectors.nonCanonicalBase64urlMultikey)
        )
        Self.assertCodecError(
            .invalidInput,
            try codec.dagCborDecode(Self.hexBytes(vectors.dagCborNonCanonicalIntegerHex))
        )
        Self.assertCodecError(
            .invalidInput,
            try codec.dagCborDecode(Self.hexBytes(vectors.dagCborDuplicateKeyHex))
        )
        Self.assertCodecError(
            .invalidInput,
            try codec.dagCborDecode(Self.hexBytes(vectors.dagCborOutOfOrderKeyHex))
        )
        Self.assertCodecError(.invalidInput, try codec.canonicalizeJson(vectors.jcsDuplicateMemberJson))
        Self.assertCodecError(
            .invalidInput,
            try codec.canonicalizeJson(vectors.jcsNonInteroperableIntegerJson)
        )
        Self.assertCodecError(.invalidInput, try codec.canonicalizeJson(vectors.jcsLoneSurrogateJson))
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

        XCTAssertTrue(String(decoding: pem, as: UTF8.self).contains("-----BEGIN PRIVATE KEY-----"))
        let decodedJson = try Self.jsonObject(try codec.decodePem(pem))
        XCTAssertEqual(decodedJson["label"] as? String, "PRIVATE KEY")
        XCTAssertEqual(decodedJson["der"] as? String, "MAMCAQE")

        let wrapped = try codec.encodePem(label: "PUBLIC KEY", der: Array("not real der".utf8), optionsJson: "{\"lineWidth\":4}")
        XCTAssertTrue(String(decoding: wrapped, as: UTF8.self).contains("bm90\nIHJl\nYWwg\nZGVy"))

        Self.assertCodecError(.invalidInput, try codec.encodePem(label: "CERTIFICATE", der: der))
        Self.assertCodecError(
            .invalidInput,
            try codec.decodePem(pem, optionsJson: "{\"allowedLabels\":[\"PUBLIC KEY\"]}")
        )

    }

    func testProviderLoadingFailsClosed() throws {
        Self.assertCodecError(
            .dynamicLibraryNotFound,
            try ReallyMeCodecRustCAbiLibrary(path: "/tmp/reallyme-codec-missing-library.dylib")
        )
    }

    func testOwnedMemoryWipesUseTheEntireMutableRegion() {
        var bytes: [UInt8] = [0xA5, 0x5A, 0xFF]
        ReallyMeCodecMemory.clearOwned(&bytes)
        XCTAssertEqual(bytes, [0, 0, 0])

        let source = Data([0x11, 0x22, 0x33, 0x44])
        var slice = source[1..<3]
        ReallyMeCodecMemory.clearOwned(&slice)
        XCTAssertEqual(Array(slice), [0, 0])
        XCTAssertEqual(Array(source), [0x11, 0x22, 0x33, 0x44])
    }

    func testMalformedProviderEnvelopeMapsToTypedFailure() {
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.decodeProtoResultEnvelope([0xFF])
        )
    }

    func testThrowingProtoApisPreserveCallerVersusProviderAttribution() throws {
        var backend = ReallyMeProtoCodecBackendError()
        backend.reason = .backendInternal
        var backendEnvelope = ReallyMeProtoCodecError()
        backendEnvelope.backend = backend
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecErrorPayload(
                try backendEnvelope.serializedBytes()
            ),
            .providerFailure
        )

        var internalError = ReallyMeProtoCodecCanonicalizationError()
        internalError.reason = .canonicalInternal
        var internalEnvelope = ReallyMeProtoCodecError()
        internalEnvelope.canonicalization = internalError
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecErrorPayload(
                try internalEnvelope.serializedBytes()
            ),
            .providerFailure
        )

        var malformedBoundary = ReallyMeProtoCodecBoundaryError()
        malformedBoundary.reason = .boundaryMalformedProtobuf
        var malformedBoundaryEnvelope = ReallyMeProtoCodecError()
        malformedBoundaryEnvelope.boundary = malformedBoundary
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecErrorPayload(
                try malformedBoundaryEnvelope.serializedBytes()
            ),
            .providerFailure
        )

        var resourceBoundary = ReallyMeProtoCodecBoundaryError()
        resourceBoundary.reason = .boundaryResourceLimitExceeded
        var resourceBoundaryEnvelope = ReallyMeProtoCodecError()
        resourceBoundaryEnvelope.boundary = resourceBoundary
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecErrorPayload(
                try resourceBoundaryEnvelope.serializedBytes()
            ),
            .invalidInput
        )

        var mismatched = ReallyMeProtoCodecBackendError()
        mismatched.reason = .multiformatInvalidMultikey
        var mismatchedEnvelope = ReallyMeProtoCodecError()
        mismatchedEnvelope.backend = mismatched
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecErrorPayload(
                try mismatchedEnvelope.serializedBytes()
            ),
            .providerFailure
        )

        var unknownReason = ReallyMeProtoCodecCanonicalizationError()
        unknownReason.reason = .UNRECOGNIZED(450)
        var unknownReasonEnvelope = ReallyMeProtoCodecError()
        unknownReasonEnvelope.canonicalization = unknownReason
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecErrorPayload(
                try unknownReasonEnvelope.serializedBytes()
            ),
            .providerFailure
        )
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecErrorPayload([0xFF]),
            .providerFailure
        )
    }

    func testAbiVersionMismatchFailsClosed() throws {
        try ReallyMeCodecRustCAbiProvider.requireCompatibleAbiVersion(2)
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireCompatibleAbiVersion(0)
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireCompatibleAbiVersion(1)
        )
    }

    func testProviderSuppliedProtoEnvelopeLimitFailsClosed() throws {
        XCTAssertEqual(
            try ReallyMeCodecRustCAbiProvider.requireValidProtoResultEnvelopeLimit(1_048_592),
            1_048_592
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireValidProtoResultEnvelopeLimit(0)
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireValidProtoResultEnvelopeLimit(67_108_865)
        )
    }

    func testSensitiveGeneratedMultikeyRequestFormattingIsRedacted() {
        var request = ReallyMeProtoCodecMultikeyParseRequest()
        request.multikey = "zSensitiveMultikey"

        XCTAssertEqual(
            request.debugDescription,
            "ReallyMeProtoCodecMultikeyParseRequest(<redacted>)"
        )
        XCTAssertEqual(
            request.textFormatString(),
            "ReallyMeProtoCodecMultikeyParseRequest(<redacted>)"
        )
    }
}
