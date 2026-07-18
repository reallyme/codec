// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import Foundation
import ReallyMeCodecProto
import SwiftProtobuf

public enum ReallyMeMulticodecTag: Sendable {
    case encryption
    case hash
    case key
    case multihash
    case multikey
}

public enum ReallyMeKeyMaterialKind: Sendable {
    case notKey
    case publicKey
    case privateKey
    case symmetricKey
}

public struct ReallyMeMulticodecMetadata:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible
{
    public let name: String
    public let algorithmName: String
    public let tag: ReallyMeMulticodecTag
    public let keyMaterialKind: ReallyMeKeyMaterialKind
    public let prefix: [UInt8]
    public let expectedKeyLength: UInt32?

    public var description: String {
        "ReallyMeMulticodecMetadata(<redacted>)"
    }

    public var debugDescription: String {
        description
    }
}

public struct ReallyMeMulticodecLookupResult:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible
{
    public let name: String
    public let prefixLength: UInt32
    public let metadata: ReallyMeMulticodecMetadata

    public var description: String {
        "ReallyMeMulticodecLookupResult(<redacted>)"
    }

    public var debugDescription: String {
        description
    }
}

public struct ReallyMeMulticodecTable:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible
{
    public let entries: [ReallyMeMulticodecMetadata]

    public var description: String {
        "ReallyMeMulticodecTable(<redacted>)"
    }

    public var debugDescription: String {
        description
    }
}

public struct ReallyMeParsedMultikey:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible
{
    public let codecName: String
    public let algorithmName: String
    public let publicKey: [UInt8]
    public let expectedPublicKeyLength: UInt32?

    public var description: String {
        "ReallyMeParsedMultikey(<redacted>)"
    }

    public var debugDescription: String {
        description
    }
}

public struct ReallyMeDagCborCidVerification:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible
{
    public let valid: Bool
    public let expectedCid: String
    public let actualCid: String

    public var description: String {
        "ReallyMeDagCborCidVerification(<redacted>)"
    }

    public var debugDescription: String {
        description
    }
}

public enum ReallyMePemLabel: String, Sendable {
    case privateKey = "PRIVATE KEY"
    case ecPrivateKey = "EC PRIVATE KEY"
    case publicKey = "PUBLIC KEY"
}

public struct ReallyMePemDecodeOptions: Sendable {
    public let allowedLabels: [ReallyMePemLabel]
    public let maxInputLen: UInt32?
    public let maxDerLen: UInt32?

    public init(
        allowedLabels: [ReallyMePemLabel] = [],
        maxInputLen: UInt32? = nil,
        maxDerLen: UInt32? = nil
    ) {
        self.allowedLabels = allowedLabels
        self.maxInputLen = maxInputLen
        self.maxDerLen = maxDerLen
    }
}

public enum ReallyMePemLineEnding: Sendable {
    case lf
    case crlf
}

public struct ReallyMePemEncodeOptions: Sendable {
    public let maxDerLen: UInt32?
    public let lineWidth: UInt32?
    public let lineEnding: ReallyMePemLineEnding?

    public init(
        maxDerLen: UInt32? = nil,
        lineWidth: UInt32? = nil,
        lineEnding: ReallyMePemLineEnding? = nil
    ) {
        self.maxDerLen = maxDerLen
        self.lineWidth = lineWidth
        self.lineEnding = lineEnding
    }
}

public final class ReallyMePemDocument:
    @unchecked Sendable, CustomStringConvertible, CustomDebugStringConvertible
{
    public let label: ReallyMePemLabel
    private var derBytes: [UInt8]

    public var der: [UInt8] {
        derBytes
    }

    fileprivate init(label: ReallyMePemLabel, der: [UInt8]) {
        self.label = label
        derBytes = der
    }

    deinit {
        ReallyMeCodecMemory.clearOwned(&derBytes)
    }

    public var description: String {
        "ReallyMePemDocument(<redacted>)"
    }

    public var debugDescription: String {
        description
    }
}

func sdkMulticodecLookupResult(
    from result: ReallyMeProtoCodecMulticodecLookupResult
) throws -> ReallyMeMulticodecLookupResult {
    try requireNoStructuredProviderUnknownFields(result.unknownFields)
    guard result.hasMetadata else {
        throw ReallyMeCodecError.providerFailure
    }
    return ReallyMeMulticodecLookupResult(
        name: result.name,
        prefixLength: result.prefixLength,
        metadata: try sdkMulticodecMetadata(from: result.metadata)
    )
}

func sdkMulticodecTable(
    from result: ReallyMeProtoCodecMulticodecTableResult
) throws -> ReallyMeMulticodecTable {
    try requireNoStructuredProviderUnknownFields(result.unknownFields)
    var entries: [ReallyMeMulticodecMetadata] = []
    entries.reserveCapacity(result.entries.count)
    for entry in result.entries {
        entries.append(try sdkMulticodecMetadata(from: entry))
    }
    return ReallyMeMulticodecTable(entries: entries)
}

func sdkParsedMultikey(
    from result: ReallyMeProtoCodecMultikeyParseResult
) throws -> ReallyMeParsedMultikey {
    try requireNoStructuredProviderUnknownFields(result.unknownFields)
    return ReallyMeParsedMultikey(
        codecName: result.codecName,
        algorithmName: result.algorithmName,
        publicKey: Array(result.publicKey),
        expectedPublicKeyLength: result.variablePublicKeyLength || result.expectedPublicKeyLength == 0
            ? nil
            : result.expectedPublicKeyLength
    )
}

func sdkDagCborCidVerification(
    from result: ReallyMeProtoCodecDagCborVerifyCidResult
) throws -> ReallyMeDagCborCidVerification {
    try requireNoStructuredProviderUnknownFields(result.unknownFields)
    return ReallyMeDagCborCidVerification(
        valid: result.valid,
        expectedCid: result.expectedCid,
        actualCid: result.actualCid
    )
}

func sdkPemDocument(
    from result: ReallyMeProtoCodecPemDecodeResult
) throws -> ReallyMePemDocument {
    try requireNoStructuredProviderUnknownFields(result.unknownFields)
    return ReallyMePemDocument(
        label: try sdkPemLabel(from: result.label),
        der: Array(result.der)
    )
}

func sdkMulticodecMetadata(
    from result: ReallyMeProtoCodecMulticodecSpec
) throws -> ReallyMeMulticodecMetadata {
    try requireNoStructuredProviderUnknownFields(result.unknownFields)
    return ReallyMeMulticodecMetadata(
        name: result.name,
        algorithmName: result.algorithmName,
        tag: try sdkMulticodecTag(from: result.tag),
        keyMaterialKind: try sdkKeyMaterialKind(from: result.keyMaterialKind),
        prefix: Array(result.prefix),
        expectedKeyLength: result.variableLength || result.fixedLength == 0
            ? nil
            : result.fixedLength
    )
}

private func sdkMulticodecTag(
    from tag: ReallyMeProtoCodecTag
) throws -> ReallyMeMulticodecTag {
    switch tag {
    case .encryption:
        return .encryption
    case .hash:
        return .hash
    case .key:
        return .key
    case .multihash:
        return .multihash
    case .multikey:
        return .multikey
    case .unspecified, .UNRECOGNIZED:
        throw ReallyMeCodecError.providerFailure
    }
}

private func sdkKeyMaterialKind(
    from kind: ReallyMeProtoCodecKeyMaterialKind
) throws -> ReallyMeKeyMaterialKind {
    switch kind {
    case .notKey:
        return .notKey
    case .publicKey:
        return .publicKey
    case .privateKey:
        return .privateKey
    case .symmetricKey:
        return .symmetricKey
    case .unspecified, .UNRECOGNIZED:
        throw ReallyMeCodecError.providerFailure
    }
}

func protoPemLabel(from label: ReallyMePemLabel) -> ReallyMeProtoCodecPemLabel {
    switch label {
    case .privateKey:
        return .privateKey
    case .ecPrivateKey:
        return .ecPrivateKey
    case .publicKey:
        return .publicKey
    }
}

private func sdkPemLabel(from label: String) throws -> ReallyMePemLabel {
    guard let value = ReallyMePemLabel(rawValue: label) else {
        throw ReallyMeCodecError.providerFailure
    }
    return value
}

func requireNoStructuredProviderUnknownFields(
    _ unknownFields: SwiftProtobuf.UnknownStorage
) throws {
    guard unknownFields.data.isEmpty else {
        throw ReallyMeCodecError.providerFailure
    }
}
