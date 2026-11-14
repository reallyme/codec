// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
import SwiftProtobuf

public struct ReallyMeMulticodecMetadata:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable
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

    public var customMirror: Mirror { Mirror(self, children: []) }
}

public struct ReallyMeMulticodecLookupResult:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable
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

    public var customMirror: Mirror { Mirror(self, children: []) }
}

public struct ReallyMeMulticodecTable:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable
{
    public let entries: [ReallyMeMulticodecMetadata]

    public var description: String {
        "ReallyMeMulticodecTable(<redacted>)"
    }

    public var debugDescription: String {
        description
    }

    public var customMirror: Mirror { Mirror(self, children: []) }
}

public struct ReallyMeParsedMultikey:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable
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

    public var customMirror: Mirror { Mirror(self, children: []) }
}

public struct ReallyMeDagCborCidVerification:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable
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

    public var customMirror: Mirror { Mirror(self, children: []) }
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
    @unchecked Sendable, CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable
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

    public var customMirror: Mirror {
        Mirror(self, children: [])
    }
}

func sdkMulticodecLookupResult(
    from result: ReallyMeProtoCodecMulticodecLookupResult
) throws(ReallyMeCodecError) -> ReallyMeMulticodecLookupResult {
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
) throws(ReallyMeCodecError) -> ReallyMeMulticodecTable {
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
) throws(ReallyMeCodecError) -> ReallyMeParsedMultikey {
    try requireNoStructuredProviderUnknownFields(result.unknownFields)
    // Multikey parsing always has either a nonzero fixed length or a variable
    // rule. Reject contradictory provider metadata before copying key bytes.
    guard result.variablePublicKeyLength == (result.expectedPublicKeyLength == 0),
        result.variablePublicKeyLength || UInt64(result.expectedPublicKeyLength) == UInt64(result.publicKey.count)
    else {
        throw ReallyMeCodecError.providerFailure
    }
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
) throws(ReallyMeCodecError) -> ReallyMeDagCborCidVerification {
    try requireNoStructuredProviderUnknownFields(result.unknownFields)
    return ReallyMeDagCborCidVerification(
        valid: result.valid,
        expectedCid: result.expectedCid,
        actualCid: result.actualCid
    )
}

func sdkPemDocument(
    from result: ReallyMeProtoCodecPemDecodeResult
) throws(ReallyMeCodecError) -> ReallyMePemDocument {
    try requireNoStructuredProviderUnknownFields(result.unknownFields)
    return ReallyMePemDocument(
        label: try sdkPemLabel(from: result.label),
        der: Array(result.der)
    )
}

func sdkMulticodecMetadata(
    from result: ReallyMeProtoCodecMulticodecSpec
) throws(ReallyMeCodecError) -> ReallyMeMulticodecMetadata {
    try requireNoStructuredProviderUnknownFields(result.unknownFields)
    // Zero with a false variable flag means not applicable in this legacy
    // message. A nonzero fixed length and a variable rule cannot coexist.
    guard !result.variableLength || result.fixedLength == 0 else {
        throw ReallyMeCodecError.providerFailure
    }
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

private func sdkPemLabel(from label: String) throws(ReallyMeCodecError) -> ReallyMePemLabel {
    guard let value = ReallyMePemLabel(rawValue: label) else {
        throw ReallyMeCodecError.providerFailure
    }
    return value
}

func requireNoStructuredProviderUnknownFields(
    _ unknownFields: SwiftProtobuf.UnknownStorage
) throws(ReallyMeCodecError) {
    guard unknownFields.data.isEmpty else {
        throw ReallyMeCodecError.providerFailure
    }
}
