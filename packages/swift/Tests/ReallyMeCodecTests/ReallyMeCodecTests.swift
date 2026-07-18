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
    let base64Whitespace: String
    let base64urlUnpadded: String
    let base64urlPadded: String
    let base64urlNonCanonicalTrailingBits: String
    let base64urlInvalidLength: String
    let base64urlWhitespace: String
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
    let multibaseMultibytePrefix: String
    let ed25519Multikey: String
    let nonCanonicalBase64urlMultikey: String
    let multikeyBindingType: String
    let mismatchedBindingType: String
    let mismatchedBindingAlgorithm: String
    let multicodecTableRequiredName: String
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
    let jcsUtf16KeyOrderInputJson: String
    let jcsUtf16KeyOrderCanonicalJson: String
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
            currentDirectory.appendingPathComponent("vectors/codec-vectors.json"),
            currentDirectory.appendingPathComponent("../../vectors/codec-vectors.json"),
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

    private static func dagCborVectorValue() throws -> ReallyMeDeterministicCborValue {
        ReallyMeDagCbor.mapText([
            ("b", ReallyMeDagCbor.unsigned(2)),
            ("a", ReallyMeDagCbor.text("one")),
            ("bytes", ReallyMeDagCbor.bytes([0, 1, 2])),
        ])
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

    func testSwiftDataAndCborBuildersPreserveCanonicalBytes() throws {
        let codec = try Self.configuredCodec()
        let vectors = try Self.codecVectors()
        let baseInput = Data(try Self.hexBytes(vectors.baseInputHex))

        XCTAssertEqual(try codec.base64urlEncode(baseInput), vectors.base64urlUnpadded)
        XCTAssertEqual(try codec.base64urlDecodeData(vectors.base64urlUnpadded), baseInput)

        let deterministicValue = ReallyMeDeterministicCbor.mapText([
            ("b", ReallyMeDeterministicCbor.unsigned(2)),
            ("a", ReallyMeDeterministicCbor.unsigned(1)),
        ])
        let deterministicBytes = try codec.deterministicCborEncodeData(deterministicValue)
        XCTAssertEqual(Array(deterministicBytes), [0xa2, 0x61, 0x61, 0x01, 0x61, 0x62, 0x02])
        XCTAssertEqual(
            try codec.deterministicCborEncode(codec.deterministicCborDecode(deterministicBytes)),
            Array(deterministicBytes)
        )

        let dagValue = try Self.dagCborVectorValue()
        let dagBytes = try codec.dagCborEncodeData(dagValue)
        XCTAssertEqual(Self.hexString(Array(dagBytes)), vectors.dagCborEncodedHex)
        XCTAssertEqual(try codec.dagCborComputeCid(dagBytes), vectors.dagCborCid)
        XCTAssertTrue(try codec.dagCborVerifyCid(cid: vectors.dagCborCid, data: dagBytes).valid)
        XCTAssertEqual(
            try codec.dagCborSha256ContentHashData(dagBytes),
            Data(try Self.hexBytes(vectors.dagCborSha256Hex))
        )
    }

    func testManagedBoundariesRejectOversizedInputsBeforeSerialization() throws {
        let codec = try Self.configuredCodec()
        let oversizedText = String(repeating: "a", count: 1_048_577)

        Self.assertCodecError(.invalidInput, try codec.base64Decode(oversizedText))
        Self.assertCodecError(.invalidInput, try codec.canonicalizeJson(oversizedText))
        Self.assertCodecError(
            .invalidInput,
            try codec.multicodecPrefixForName(oversizedText)
        )
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

        let metadata = try codec.multicodecPrefixForName(vectors.ed25519CodecName)
        XCTAssertEqual(metadata.name, vectors.ed25519CodecName)
        XCTAssertEqual(metadata.tag, .key)
        XCTAssertEqual(metadata.algorithmName, vectors.ed25519AlgorithmName)
        XCTAssertEqual(Self.hexString(metadata.prefix), vectors.ed25519PrefixHex)

        let lookup = try codec.multicodecLookupPrefix(prefixedPublicKey)
        XCTAssertEqual(lookup.name, vectors.ed25519CodecName)
        XCTAssertEqual(try codec.multicodecStripPrefix(prefixedPublicKey), publicKey)
        XCTAssertTrue(try codec.multicodecTable().entries.contains { $0.name == vectors.multicodecTableRequiredName })

        XCTAssertEqual(
            try codec.multikeyEncode(codecName: vectors.ed25519CodecName, publicKey: publicKey),
            vectors.ed25519Multikey
        )
        let parsed = try codec.multikeyParse(vectors.ed25519Multikey)
        XCTAssertEqual(parsed.codecName, vectors.ed25519CodecName)
        XCTAssertEqual(parsed.publicKey, publicKey)
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

        let dagCborBytes = try codec.dagCborEncode(Self.dagCborVectorValue())
        XCTAssertEqual(Self.hexString(dagCborBytes), vectors.dagCborEncodedHex)
        XCTAssertEqual(
            try codec.dagCborEncode(codec.dagCborDecode(dagCborBytes)),
            dagCborBytes
        )
        XCTAssertEqual(try codec.dagCborComputeCid(dagCborBytes), vectors.dagCborCid)
        XCTAssertTrue(try codec.dagCborVerifyCid(cid: vectors.dagCborCid, bytes: dagCborBytes).valid)
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
        let privateLabel = try XCTUnwrap(ReallyMePemLabel(rawValue: vectors.pemPrivateLabel))
        XCTAssertEqual(
            try codec.encodePem(label: privateLabel, der: privateDer),
            Array(vectors.pemPrivatePem.utf8)
        )
        let decodedPem = try codec.decodePem(Array(vectors.pemPrivatePem.utf8))
        XCTAssertEqual(decodedPem.label.rawValue, vectors.pemPrivateLabel)
        XCTAssertEqual(
            try codec.encodePem(
                label: try XCTUnwrap(ReallyMePemLabel(rawValue: vectors.pemPublicLabel)),
                der: Array(vectors.pemWrappedDerText.utf8),
                options: ReallyMePemEncodeOptions(lineWidth: 4)
            ),
            Array(vectors.pemWrappedPem.utf8)
        )

        let binaryResponse = try codec.processOperation(
            Self.hexBytes(vectors.protoMulticodecTableRequestHex)
        )
        let jsonResponse = try codec.processOperationJson(
            Array(vectors.protoMulticodecTableRequestJson.utf8)
        )
        XCTAssertEqual(binaryResponse, jsonResponse)
        let decodedResponse = try ReallyMeProtoCodecOperationResponse(serializedBytes: binaryResponse)
        guard case .result(let result)? = decodedResponse.outcome,
              case .multicodecTable(let decodedTable)? = result.result else {
            XCTFail("expected generated multicodec table result")
            return
        }
        XCTAssertTrue(decodedTable.entries.contains { $0.name == vectors.multicodecTableRequiredName })

        Self.assertCodecError(.invalidInput, try codec.processGeneratedOperation(request: [0xff]))
        let malformedJsonResponse = try ReallyMeProtoCodecOperationResponse(
            serializedBytes: try codec.processOperationJson(Array("{".utf8))
        )
        guard case .error(let malformedJsonError)? = malformedJsonResponse.outcome else {
            XCTFail("expected generated error envelope for malformed ProtoJSON request")
            return
        }
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecError(malformedJsonError),
            .invalidInput
        )
    }

    func testSharedVectorSuiteRejectsNonCanonicalInputs() throws {
        let codec = try Self.configuredCodec()
        let vectors = try Self.codecVectors()

        Self.assertCodecError(.invalidInput, try codec.base64Decode(vectors.base64MissingPadding))
        Self.assertCodecError(
            .invalidInput,
            try codec.base64Decode(vectors.base64NonCanonicalTrailingBits)
        )
        Self.assertCodecError(.invalidInput, try codec.base64Decode(vectors.base64Whitespace))
        Self.assertCodecError(.invalidInput, try codec.base64urlDecode(vectors.base64urlPadded))
        Self.assertCodecError(
            .invalidInput,
            try codec.base64urlDecode(vectors.base64urlNonCanonicalTrailingBits)
        )
        Self.assertCodecError(.invalidInput, try codec.base64urlDecode(vectors.base64urlInvalidLength))
        Self.assertCodecError(.invalidInput, try codec.base64urlDecode(vectors.base64urlWhitespace))
        Self.assertCodecError(.invalidInput, try codec.multibaseDecode(vectors.unsupportedMultibase))
        Self.assertCodecError(.invalidInput, try codec.multibaseDecode(vectors.multibaseMultibytePrefix))
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
        XCTAssertEqual(
            try codec.canonicalizeJson(vectors.jcsUtf16KeyOrderInputJson),
            vectors.jcsUtf16KeyOrderCanonicalJson
        )
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

        let metadata = try codec.multicodecPrefixForName("ed25519-pub")
        XCTAssertEqual(metadata.name, "ed25519-pub")
        XCTAssertEqual(metadata.tag, .key)
        XCTAssertEqual(metadata.algorithmName, "Ed25519")
        XCTAssertEqual(metadata.expectedKeyLength, 32)
        Self.assertCodecError(.invalidInput, try codec.multicodecPrefixForName("not-a-codec"))

        let prefixed = metadata.prefix + publicKey
        let lookup = try codec.multicodecLookupPrefix(prefixed)
        XCTAssertEqual(lookup.name, "ed25519-pub")
        XCTAssertEqual(lookup.prefixLength, UInt32(metadata.prefix.count))
        Self.assertCodecError(.invalidInput, try codec.multicodecLookupPrefix([0, 0, 7]))
        XCTAssertEqual(try codec.multicodecStripPrefix(prefixed), publicKey)
        XCTAssertTrue(try codec.multicodecTable().entries.contains { $0.name == "mlkem-1024-pub" })

        let multikey = try codec.multikeyEncode(codecName: "ed25519-pub", publicKey: publicKey)
        let parsed = try codec.multikeyParse(multikey)
        XCTAssertEqual(parsed.codecName, "ed25519-pub")
        XCTAssertEqual(parsed.algorithmName, "Ed25519")
        XCTAssertEqual(parsed.publicKey, publicKey)
        XCTAssertTrue(try codec.bindingTypeMatchesCodec(bindingType: "Multikey", codecName: "ed25519-pub"))
        try codec.requireSupportedMulticodec("ed25519-pub")
        try codec.validateKeyBinding(bindingType: "Multikey", algorithm: nil, multikey: multikey)

        Self.assertCodecError(.invalidInput, try codec.requireSupportedMulticodec("not-a-codec"))
        Self.assertCodecError(
            .invalidInput,
            try codec.validateKeyBinding(bindingType: "P256Key2024", algorithm: "P-256", multikey: multikey)
        )

        Self.assertCodecError(.invalidInput, try codec.multikeyParse("not-a-key"))
    }

    func testDagCborCidAndJcsOperationsUseRustProvider() throws {
        let codec = try Self.configuredCodec()

        let encoded = try codec.dagCborEncode(Self.dagCborVectorValue())
        XCTAssertFalse(encoded.isEmpty)
        XCTAssertEqual(try codec.dagCborEncode(codec.dagCborDecode(encoded)), encoded)

        let cid = try codec.dagCborComputeCid(encoded)
        XCTAssertTrue(try codec.isValidCidString(cid))
        XCTAssertFalse(try codec.isValidCidString("not-a-cid"))
        XCTAssertEqual(try codec.tryParseCid(cid), cid)
        XCTAssertNil(try codec.tryParseCid("not-a-cid"))

        let verification = try codec.dagCborVerifyCid(cid: cid, bytes: encoded)
        XCTAssertTrue(verification.valid)
        XCTAssertEqual(verification.expectedCid, cid)

        let invalidUpperPayloadCid = String(cid.prefix(1)) + cid.dropFirst().uppercased()
        let invalidVerification = try codec.dagCborVerifyCid(cid: invalidUpperPayloadCid, bytes: encoded)
        XCTAssertFalse(invalidVerification.valid)
        XCTAssertEqual(invalidVerification.actualCid, "")
        let emptyCidVerification = try codec.dagCborVerifyCid(cid: "", bytes: encoded)
        XCTAssertFalse(emptyCidVerification.valid)
        XCTAssertEqual(emptyCidVerification.expectedCid, cid)
        XCTAssertEqual(emptyCidVerification.actualCid, "")

        XCTAssertEqual(try codec.dagCborSha256ContentHash(encoded).count, 32)
        XCTAssertGreaterThan(try codec.dagCborMultihash(encoded).count, 32)
        XCTAssertEqual(try codec.dagCborCodecCode(), 0x71)
        Self.assertCodecError(.invalidInput, try codec.dagCborDecode([0xa2, 0x61, 0x62, 0x01, 0x61, 0x61, 0x02]))
        let oversizedCbor = [UInt8](repeating: 0, count: 1024 * 1024 + 1)
        Self.assertCodecError(.invalidInput, try codec.dagCborDecode(oversizedCbor))
        Self.assertCodecError(.invalidInput, try codec.dagCborComputeCid(oversizedCbor))
        Self.assertCodecError(.invalidInput, try codec.dagCborVerifyCid(cid: cid, bytes: oversizedCbor))
        Self.assertCodecError(.invalidInput, try codec.dagCborSha256ContentHash(oversizedCbor))
        Self.assertCodecError(.invalidInput, try codec.dagCborMultihash(oversizedCbor))

        XCTAssertEqual(try codec.canonicalizeJson("{\"b\":2,\"a\":1}"), "{\"a\":1,\"b\":2}")
        XCTAssertEqual(try codec.canonicalizeJson("333333333.33333329"), "333333333.3333333")
        Self.assertCodecError(.invalidInput, try codec.canonicalizeJson("{"))
    }

    func testDeterministicCborTypedSurfaceUsesGeneratedProto() throws {
        let codec = try Self.configuredCodec()
        let value = ReallyMeDeterministicCborValue.map([
            ReallyMeDeterministicCborMapEntry(
                key: .text("b"),
                value: .integer(.unsigned(2))
            ),
            ReallyMeDeterministicCborMapEntry(
                key: .integer(.unsigned(1)),
                value: .text("i")
            ),
            ReallyMeDeterministicCborMapEntry(
                key: .text("1"),
                value: .text("t")
            ),
        ])

        let encoded = try codec.deterministicCborEncode(value)
        XCTAssertEqual(Self.hexString(encoded), "a301616961316174616202")
        let decoded = try codec.deterministicCborDecode(encoded)
        XCTAssertEqual(try codec.deterministicCborEncode(decoded), encoded)

        var maximumDepth = ReallyMeDeterministicCborValue.null
        for _ in 0..<64 {
            maximumDepth = .map([
                ReallyMeDeterministicCborMapEntry(
                    key: .integer(.unsigned(1)),
                    value: maximumDepth
                )
            ])
        }
        let maximumDepthEncoded = try codec.deterministicCborEncode(maximumDepth)
        let maximumDepthDecoded = try codec.deterministicCborDecode(maximumDepthEncoded)
        XCTAssertEqual(
            try codec.deterministicCborEncode(maximumDepthDecoded),
            maximumDepthEncoded
        )
        XCTAssertEqual(String(describing: value), "ReallyMeDeterministicCborValue(<redacted>)")
        XCTAssertEqual(
            String(describing: ReallyMeDeterministicCborMapKey.text("passportNumber")),
            "ReallyMeDeterministicCborMapKey(<redacted>)"
        )

        XCTAssertThrowsError(try ReallyMeDeterministicCborNegativeInteger(0)) { error in
            XCTAssertEqual(error as? ReallyMeCodecError, .invalidInput)
        }
        Self.assertCodecError(.invalidInput, try codec.deterministicCborDecode([0x18, 0x00]))
    }

    func testDeterministicCborProviderTreeIsValidatedBeforeSdkCopy() throws {
        var nullValue = ReallyMeProtoCodecDeterministicCborValue()
        nullValue.nullValue = ReallyMeProtoCodecDeterministicCborNull()

        var oversizedArray = ReallyMeProtoCodecDeterministicCborArray()
        oversizedArray.values = Array(repeating: nullValue, count: 16_385)
        var oversizedValue = ReallyMeProtoCodecDeterministicCborValue()
        oversizedValue.arrayValue = oversizedArray
        XCTAssertThrowsError(try validateProviderDeterministicCborValue(oversizedValue)) { error in
            XCTAssertEqual(error as? ReallyMeCodecError, .providerFailure)
        }

        let valueWithUnknownField = try ReallyMeProtoCodecDeterministicCborValue(
            serializedBytes: [0x0a, 0x00, 0x98, 0x06, 0x01]
        )
        XCTAssertThrowsError(
            try validateProviderDeterministicCborValue(valueWithUnknownField)
        ) { error in
            XCTAssertEqual(error as? ReallyMeCodecError, .providerFailure)
        }

        func textKey(_ value: String) -> ReallyMeProtoCodecDeterministicCborMapKey {
            var text = ReallyMeProtoCodecDeterministicCborText()
            text.value = value
            var key = ReallyMeProtoCodecDeterministicCborMapKey()
            key.textKey = text
            return key
        }

        func unsignedKey(_ value: UInt64) -> ReallyMeProtoCodecDeterministicCborMapKey {
            var unsigned = ReallyMeProtoCodecDeterministicCborUnsignedInteger()
            unsigned.value = value
            var integer = ReallyMeProtoCodecDeterministicCborInteger()
            integer.unsignedValue = unsigned
            var key = ReallyMeProtoCodecDeterministicCborMapKey()
            key.integerKey = integer
            return key
        }

        func entry(_ key: ReallyMeProtoCodecDeterministicCborMapKey)
            -> ReallyMeProtoCodecDeterministicCborMapEntry
        {
            var entry = ReallyMeProtoCodecDeterministicCborMapEntry()
            entry.key = key
            entry.value = nullValue
            return entry
        }

        var duplicateMap = ReallyMeProtoCodecDeterministicCborMap()
        duplicateMap.entries = [entry(textKey("a")), entry(textKey("a"))]
        var duplicateMapValue = ReallyMeProtoCodecDeterministicCborValue()
        duplicateMapValue.mapValue = duplicateMap
        XCTAssertThrowsError(
            try validateProviderDeterministicCborValue(duplicateMapValue)
        ) { error in
            XCTAssertEqual(error as? ReallyMeCodecError, .providerFailure)
        }

        var duplicateUnsignedMap = ReallyMeProtoCodecDeterministicCborMap()
        duplicateUnsignedMap.entries = [
            entry(unsignedKey(UInt64.max)),
            entry(unsignedKey(UInt64.max)),
        ]
        var duplicateUnsignedMapValue = ReallyMeProtoCodecDeterministicCborValue()
        duplicateUnsignedMapValue.mapValue = duplicateUnsignedMap
        XCTAssertThrowsError(
            try validateProviderDeterministicCborValue(duplicateUnsignedMapValue)
        ) { error in
            XCTAssertEqual(error as? ReallyMeCodecError, .providerFailure)
        }

        var exactUtf8Map = ReallyMeProtoCodecDeterministicCborMap()
        exactUtf8Map.entries = [
            entry(textKey("\u{00e9}")),
            entry(textKey("e\u{0301}")),
        ]
        var exactUtf8MapValue = ReallyMeProtoCodecDeterministicCborValue()
        exactUtf8MapValue.mapValue = exactUtf8Map
        XCTAssertNoThrow(try validateProviderDeterministicCborValue(exactUtf8MapValue))
    }

    func testPemRoundTripAndProtoErrorsUseRustProvider() throws {
        let codec = try Self.configuredCodec()
        let der: [UInt8] = [0x30, 0x03, 0x02, 0x01, 0x01]
        let pem = try codec.encodePem(label: .privateKey, der: der)

        XCTAssertTrue(String(decoding: pem, as: UTF8.self).contains("-----BEGIN PRIVATE KEY-----"))
        let decoded = try codec.decodePem(pem)
        XCTAssertEqual(decoded.label, .privateKey)
        XCTAssertEqual(decoded.der, der)

        let wrapped = try codec.encodePem(
            label: .publicKey,
            der: Array("not real der".utf8),
            options: ReallyMePemEncodeOptions(lineWidth: 4)
        )
        XCTAssertTrue(String(decoding: wrapped, as: UTF8.self).contains("bm90\nIHJl\nYWwg\nZGVy"))

        Self.assertCodecError(
            .invalidInput,
            try codec.encodePem(
                label: .publicKey,
                der: der,
                options: ReallyMePemEncodeOptions(lineWidth: 77)
            )
        )
        Self.assertCodecError(
            .invalidInput,
            try codec.decodePem(
                pem,
                options: ReallyMePemDecodeOptions(allowedLabels: [.publicKey])
            )
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

    func testAbiVersionMismatchFailsClosed() throws {
        try ReallyMeCodecRustCAbiProvider.requireCompatibleAbiVersion(5)
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireCompatibleAbiVersion(0)
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireCompatibleAbiVersion(1)
        )
    }

    func testProviderSuppliedOperationResponseLimitFailsClosed() throws {
        XCTAssertEqual(
            try ReallyMeCodecRustCAbiProvider.requireValidFfiLimit(67_108_864),
            67_108_864
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireValidFfiLimit(0)
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireValidFfiLimit(UInt(Int.max) + 1)
        )
        XCTAssertEqual(
            try ReallyMeCodecRustCAbiProvider.requireValidOperationResponseLimit(
                10_489_856,
                maxFfiOutputLength: 67_108_864
            ),
            10_489_856
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireValidOperationResponseLimit(
                0,
                maxFfiOutputLength: 67_108_864
            )
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireValidOperationResponseLimit(
                67_108_865,
                maxFfiOutputLength: 67_108_864
            )
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireValidOperationResponseLimit(
                UInt(Int.max) + 1,
                maxFfiOutputLength: 67_108_864
            )
        )
    }

    func testProviderErrorOriginsAreAttributedDeterministically() {
        var boundary = ReallyMeProtoCodecBoundaryError()
        boundary.reason = .boundaryMalformedProtobuf
        var callerError = ReallyMeProtoCodecError()
        callerError.boundary = boundary
        callerError.origin = .caller
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecError(callerError),
            .invalidInput
        )

        var backend = ReallyMeProtoCodecBackendError()
        backend.reason = .backendInternal
        var providerError = ReallyMeProtoCodecError()
        providerError.backend = backend
        providerError.origin = .provider
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecError(providerError),
            .providerFailure
        )

        var mismatchedOrigin = callerError
        mismatchedOrigin.origin = .provider
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecError(mismatchedOrigin),
            .providerFailure
        )

        var unrecognizedReason = ReallyMeProtoCodecBoundaryError()
        unrecognizedReason.reason = .UNRECOGNIZED(699)
        var unknownError = ReallyMeProtoCodecError()
        unknownError.boundary = unrecognizedReason
        unknownError.origin = .caller
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecError(unknownError),
            .providerFailure
        )
    }

    func testSensitiveGeneratedMultikeyRequestFormattingIsRedacted() {
        var request = ReallyMeProtoCodecMultikeyParseRequest()
        request.multikey = "zSensitiveMultikey"
        var otherRequest = ReallyMeProtoCodecMultikeyParseRequest()
        otherRequest.multikey = "zDifferentSensitiveMultikey"

        XCTAssertEqual(
            request.debugDescription,
            "ReallyMeProtoCodecMultikeyParseRequest(<redacted>)"
        )
        XCTAssertEqual(
            request.textFormatString(),
            "ReallyMeProtoCodecMultikeyParseRequest(<redacted>)"
        )
        var hasher = Hasher()
        request.hash(into: &hasher)
        let redactedHash = hasher.finalize()
        var otherHasher = Hasher()
        otherRequest.hash(into: &otherHasher)
        XCTAssertEqual(redactedHash, otherHasher.finalize())
    }
}
