// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import Foundation
@testable import ReallyMeCodec
import XCTest

private struct DeterministicCborManifest: Decodable {
    let schemaVersion: Int
    let deterministicCbor: DeterministicCborVectors
}

private struct DeterministicCborVectors: Decodable {
    let profile: String
    let fixtureClasses: [String: String]
    let positive: [DeterministicCborPositiveVector]
    let negative: [DeterministicCborNegativeVector]
    let equivalentInputOrders: [DeterministicCborEquivalentVector]
    let resourceRejections: [DeterministicCborResourceVector]
    let interoperability: [DeterministicCborInteroperabilityVector]
}

private struct DeterministicCborPositiveVector: Decodable {
    let name: String
    let value: DeterministicCborFixtureValue
    let hex: String
}

private struct DeterministicCborNegativeVector: Decodable {
    let name: String
    let hex: String
    let reason: String
}

private struct DeterministicCborEquivalentVector: Decodable {
    let name: String
    let inputs: [[DeterministicCborFixtureMapEntry]]
    let hex: String
}

private struct DeterministicCborResourceVector: Decodable {
    let name: String
    let construction: DeterministicCborResourceConstruction
    let reason: String
}

private struct DeterministicCborResourceConstruction: Decodable {
    let kind: String
    let count: Int?
    let fillByteHex: String?
    let branching: Int?
    let levels: Int?
    let depth: Int?
}

private struct DeterministicCborInteroperabilityVector: Decodable {
    let name: String
    let fixtureKind: String
    let sourceRepo: String
    let sourceCommit: String
    let source: String
    let explanation: String
    let sourceFiles: [DeterministicCborSourceFile]
    let entryCount: Int
    let byteLength: Int
    let hex: String
    let sha256: String
}

private struct DeterministicCborSourceFile: Decodable {
    let path: String
    let sha256: String
}

private enum DeterministicCborFixtureInteger: Decodable {
    case unsigned(String)
    case negative(String)

    private enum CodingKeys: String, CodingKey {
        case unsigned
        case negative
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        guard container.allKeys.count == 1, let key = container.allKeys.first else {
            throw DecodingError.dataCorrupted(
                DecodingError.Context(
                    codingPath: decoder.codingPath,
                    debugDescription: "deterministic-CBOR integer must have one branch"
                )
            )
        }
        switch key {
        case .unsigned:
            self = .unsigned(try container.decode(String.self, forKey: key))
        case .negative:
            self = .negative(try container.decode(String.self, forKey: key))
        }
    }
}

private enum DeterministicCborFixtureMapKey: Decodable {
    case integer(DeterministicCborFixtureInteger)
    case text(String)

    private enum CodingKeys: String, CodingKey {
        case integer
        case text
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        guard container.allKeys.count == 1, let key = container.allKeys.first else {
            throw DecodingError.dataCorrupted(
                DecodingError.Context(
                    codingPath: decoder.codingPath,
                    debugDescription: "deterministic-CBOR map key must have one branch"
                )
            )
        }
        switch key {
        case .integer:
            self = .integer(try container.decode(DeterministicCborFixtureInteger.self, forKey: key))
        case .text:
            self = .text(try container.decode(String.self, forKey: key))
        }
    }
}

private struct DeterministicCborFixtureMapEntry: Decodable {
    let key: DeterministicCborFixtureMapKey
    let value: DeterministicCborFixtureValue
}

private indirect enum DeterministicCborFixtureValue: Decodable {
    case unsigned(String)
    case negative(String)
    case bytes(String)
    case text(String)
    case bool(Bool)
    case null
    case array([DeterministicCborFixtureValue])
    case map([DeterministicCborFixtureMapEntry])

    private enum CodingKeys: String, CodingKey {
        case unsigned
        case negative
        case bytes
        case text
        case bool
        case null
        case array
        case map
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        guard container.allKeys.count == 1, let key = container.allKeys.first else {
            throw DecodingError.dataCorrupted(
                DecodingError.Context(
                    codingPath: decoder.codingPath,
                    debugDescription: "deterministic-CBOR value must have one branch"
                )
            )
        }
        switch key {
        case .unsigned:
            self = .unsigned(try container.decode(String.self, forKey: key))
        case .negative:
            self = .negative(try container.decode(String.self, forKey: key))
        case .bytes:
            self = .bytes(try container.decode(String.self, forKey: key))
        case .text:
            self = .text(try container.decode(String.self, forKey: key))
        case .bool:
            self = .bool(try container.decode(Bool.self, forKey: key))
        case .null:
            guard try container.decode(Bool.self, forKey: key) else {
                throw DecodingError.dataCorruptedError(
                    forKey: key,
                    in: container,
                    debugDescription: "deterministic-CBOR null marker must be true"
                )
            }
            self = .null
        case .array:
            self = .array(try container.decode([DeterministicCborFixtureValue].self, forKey: key))
        case .map:
            self = .map(try container.decode([DeterministicCborFixtureMapEntry].self, forKey: key))
        }
    }
}

final class DeterministicCborVectorTests: XCTestCase {
    private static let maximumCborBytes = 1_048_576
    private static let cborU32LengthHeaderBytes = 5

    func testSharedPositiveNegativeAndEquivalentVectors() throws {
        let codec = try Self.configuredCodec()
        let vectors = try Self.vectors()
        XCTAssertEqual(vectors.profile, "rfc8949-core-deterministic-reallyme-0.2.0")
        XCTAssertEqual(vectors.fixtureClasses["positive"], "golden")
        XCTAssertEqual(vectors.fixtureClasses["negative"], "rejection-fixture")
        XCTAssertEqual(vectors.fixtureClasses["resourceRejections"], "construction-recipe")
        XCTAssertEqual(vectors.fixtureClasses["interoperability"], "interop-fixture")

        for vector in vectors.positive {
            let expected = try Self.hexBytes(vector.hex)
            let value = try Self.sdkValue(vector.value)
            XCTAssertEqual(try codec.deterministicCborEncode(value), expected, vector.name)
            let decoded = try codec.deterministicCborDecode(expected)
            XCTAssertEqual(try codec.deterministicCborEncode(decoded), expected, vector.name)
        }

        for vector in vectors.negative {
            Self.assertInvalidInput(
                try codec.deterministicCborDecode(Self.hexBytes(vector.hex)),
                vector.name + ":" + vector.reason
            )
        }

        for vector in vectors.equivalentInputOrders {
            let expected = try Self.hexBytes(vector.hex)
            for entries in vector.inputs {
                let value = ReallyMeDeterministicCborValue.map(
                    try entries.map(Self.sdkMapEntry)
                )
                XCTAssertEqual(try codec.deterministicCborEncode(value), expected, vector.name)
            }
        }
    }

    func testTypedBuildersPreserveDeterministicAndDagProfiles() throws {
        let codec = try Self.configuredCodec()
        let deterministic = ReallyMeDeterministicCbor.mapInt([
            (2, .text("b")),
            (1, .text("a")),
        ])
        XCTAssertEqual(
            try codec.deterministicCborEncode(deterministic),
            try Self.hexBytes("a2016161026162")
        )

        let dag = ReallyMeDagCbor.mapText([
            ("b", ReallyMeDagCbor.unsigned(2)),
            ("a", ReallyMeDagCbor.bytes(Data([0, 1, 2]))),
        ])
        XCTAssertEqual(
            try codec.dagCborEncode(dag),
            try Self.hexBytes("a2616143000102616202")
        )
    }

    func testSharedResourceRecipesAndSemanticMaximum() throws {
        let codec = try Self.configuredCodec()
        let vectors = try Self.vectors()

        Self.assertInvalidInput(
            try codec.deterministicCborEncode(
                .text(String(repeating: "a", count: Self.maximumCborBytes + 1))
            ),
            "aggregate-text-limit-plus-one"
        )

        for vector in vectors.resourceRejections {
            switch vector.construction.kind {
            case "encoded-byte-count":
                let count = try XCTUnwrap(vector.construction.count)
                let fill = try Self.hexBytes(try XCTUnwrap(vector.construction.fillByteHex))
                XCTAssertEqual(fill.count, 1)
                Self.assertInvalidInput(
                    try codec.deterministicCborDecode([UInt8](repeating: fill[0], count: count)),
                    vector.name
                )
            case "byte-string-length":
                let count = try XCTUnwrap(vector.construction.count)
                Self.assertInvalidInput(
                    try codec.deterministicCborEncode(.bytes([UInt8](repeating: 0, count: count))),
                    vector.name
                )
            case "balanced-array-tree":
                let branching = try XCTUnwrap(vector.construction.branching)
                let levels = try XCTUnwrap(vector.construction.levels)
                Self.assertInvalidInput(
                    try codec.deterministicCborEncode(
                        Self.balancedArrayTree(branching: branching, levels: levels)
                    ),
                    vector.name
                )
            case "array-of-null":
                let count = try XCTUnwrap(vector.construction.count)
                Self.assertInvalidInput(
                    try codec.deterministicCborEncode(
                        .array(Array(repeating: .null, count: count))
                    ),
                    vector.name
                )
            case "nested-singleton-arrays":
                let depth = try XCTUnwrap(vector.construction.depth)
                var value = ReallyMeDeterministicCborValue.null
                for _ in 0..<depth {
                    value = .array([value])
                }
                Self.assertInvalidInput(try codec.deterministicCborEncode(value), vector.name)
            default:
                XCTFail("unknown deterministic-CBOR resource construction")
            }
        }

        let payloadCount = Self.maximumCborBytes - Self.cborU32LengthHeaderBytes
        let encoded = try codec.deterministicCborEncode(
            .bytes([UInt8](repeating: 0, count: payloadCount))
        )
        XCTAssertEqual(encoded.count, Self.maximumCborBytes)
        XCTAssertEqual(Array(encoded.prefix(5)), [0x5a, 0x00, 0x0f, 0xff, 0xfb])
        let decoded = try codec.deterministicCborDecode(encoded)
        guard case .bytes(let bytes) = decoded else {
            return XCTFail("maximum deterministic-CBOR value decoded to the wrong branch")
        }
        XCTAssertEqual(bytes.count, payloadCount)
    }

    func testIdkitInteroperabilityFixtureRoundTripsThroughTypedSdk() throws {
        let codec = try Self.configuredCodec()
        let vectors = try Self.vectors()
        let names = Set(vectors.interoperability.map(\.name))
        XCTAssertTrue(names.contains("idkit-ios-synthetic-passport-claims-v1"))
        XCTAssertTrue(names.contains("idkit-ios-synthetic-passport-claims-null-place-of-birth-v1"))
        XCTAssertTrue(names.contains("idkit-ios-synthetic-fingerprint-map-v1"))
        XCTAssertTrue(names.contains("idkit-ios-synthetic-mixed-integer-claim-tags-v1"))

        for fixture in vectors.interoperability {
            let encoded = try Self.hexBytes(fixture.hex)
            XCTAssertEqual(fixture.fixtureKind, "synthetic", fixture.name)
            XCTAssertEqual(fixture.sourceRepo, "reallyme/idkit-ios", fixture.name)
            XCTAssertEqual(fixture.sourceCommit, "content-hash-pinned", fixture.name)
            XCTAssertFalse(fixture.source.isEmpty, fixture.name)
            XCTAssertFalse(fixture.explanation.isEmpty, fixture.name)
            XCTAssertFalse(fixture.sourceFiles.isEmpty, fixture.name)
            for sourceFile in fixture.sourceFiles {
                XCTAssertFalse(sourceFile.path.isEmpty, fixture.name)
                XCTAssertEqual(sourceFile.sha256.count, 64, fixture.name)
            }
            XCTAssertEqual(fixture.sha256.count, 64, fixture.name)
            XCTAssertEqual(encoded.count, fixture.byteLength, fixture.name)
            let decoded = try codec.deterministicCborDecode(encoded)
            guard case .map(let entries) = decoded else {
                return XCTFail("idkit interoperability fixture did not decode as a map: \(fixture.name)")
            }
            XCTAssertEqual(entries.count, fixture.entryCount, fixture.name)
            XCTAssertEqual(try codec.deterministicCborEncode(decoded), encoded, fixture.name)
        }
    }

    private static func configuredCodec() throws -> ReallyMeCodec {
        let environmentPath = ProcessInfo.processInfo.environment["REALLYME_CODEC_FFI_LIBRARY_PATH"]
        let libraryPath: String
        if let environmentPath, !environmentPath.isEmpty {
            libraryPath = environmentPath
        } else {
            #if os(macOS)
                let libraryName = "libreallyme_codec_ffi.dylib"
            #elseif os(Linux)
                let libraryName = "libreallyme_codec_ffi.so"
            #else
                let libraryName = "reallyme_codec_ffi.dll"
            #endif
            libraryPath = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
                .appendingPathComponent("target/debug")
                .appendingPathComponent(libraryName)
                .path
        }
        guard FileManager.default.fileExists(atPath: libraryPath) else {
            throw ReallyMeCodecError.providerFailure
        }
        return try ReallyMeCodec(
            rustCAbiLibrary: ReallyMeCodecRustCAbiLibrary(path: libraryPath)
        )
    }

    private static func vectors() throws -> DeterministicCborVectors {
        let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let candidates = [
            root.appendingPathComponent("vectors/codec-vectors.json"),
            root.appendingPathComponent("../../vectors/codec-vectors.json"),
        ]
        for candidate in candidates where FileManager.default.fileExists(atPath: candidate.path) {
            let manifest = try JSONDecoder().decode(
                DeterministicCborManifest.self,
                from: Data(contentsOf: candidate)
            )
            guard manifest.schemaVersion == 2 else {
                throw ReallyMeCodecError.invalidInput
            }
            return manifest.deterministicCbor
        }
        throw ReallyMeCodecError.invalidInput
    }

    private static func sdkInteger(
        _ value: DeterministicCborFixtureInteger
    ) throws -> ReallyMeDeterministicCborInteger {
        switch value {
        case .unsigned(let text):
            return .unsigned(try XCTUnwrap(UInt64(text)))
        case .negative(let text):
            return .negative(try ReallyMeDeterministicCborNegativeInteger(
                try XCTUnwrap(Int64(text))
            ))
        }
    }

    private static func sdkMapKey(
        _ key: DeterministicCborFixtureMapKey
    ) throws -> ReallyMeDeterministicCborMapKey {
        switch key {
        case .integer(let value):
            return .integer(try sdkInteger(value))
        case .text(let value):
            return .text(value)
        }
    }

    private static func sdkMapEntry(
        _ entry: DeterministicCborFixtureMapEntry
    ) throws -> ReallyMeDeterministicCborMapEntry {
        ReallyMeDeterministicCborMapEntry(
            key: try sdkMapKey(entry.key),
            value: try sdkValue(entry.value)
        )
    }

    private static func sdkValue(
        _ value: DeterministicCborFixtureValue
    ) throws -> ReallyMeDeterministicCborValue {
        switch value {
        case .unsigned(let text):
            return .integer(.unsigned(try XCTUnwrap(UInt64(text))))
        case .negative(let text):
            return .integer(.negative(try ReallyMeDeterministicCborNegativeInteger(
                try XCTUnwrap(Int64(text))
            )))
        case .bytes(let value):
            return .bytes(Array(try XCTUnwrap(Data(base64Encoded: value))))
        case .text(let value):
            return .text(value)
        case .bool(let value):
            return .bool(value)
        case .null:
            return .null
        case .array(let values):
            return .array(try values.map(sdkValue))
        case .map(let entries):
            return .map(try entries.map(sdkMapEntry))
        }
    }

    private static func balancedArrayTree(
        branching: Int,
        levels: Int
    ) -> ReallyMeDeterministicCborValue {
        var value = ReallyMeDeterministicCborValue.null
        for _ in 0..<levels {
            value = .array(Array(repeating: value, count: branching))
        }
        return value
    }

    private static func hexBytes(_ text: String) throws -> [UInt8] {
        guard text.count.isMultiple(of: 2) else {
            throw ReallyMeCodecError.invalidInput
        }
        var result: [UInt8] = []
        result.reserveCapacity(text.count / 2)
        var index = text.startIndex
        while index < text.endIndex {
            let next = text.index(index, offsetBy: 2)
            guard let byte = UInt8(text[index..<next], radix: 16) else {
                throw ReallyMeCodecError.invalidInput
            }
            result.append(byte)
            index = next
        }
        return result
    }

    private static func assertInvalidInput(
        _ operation: @autoclosure () throws -> Any,
        _ context: String
    ) {
        XCTAssertThrowsError(try operation(), context) { error in
            XCTAssertEqual(error as? ReallyMeCodecError, .invalidInput, context)
        }
    }
}
