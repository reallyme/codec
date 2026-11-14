// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Generated from reallyme/codec/v1/codec.proto. Do not edit.

/// Typed SDK failures contain no raw input or backend diagnostics.
public enum ReallyMeCodecError: Error, Equatable, Sendable {
    case unsupportedPlatform
    case dynamicLibraryNotFound
    case dynamicLibraryLoadFailed
    case providerUnavailable
    case symbolNotFound
    case invalidInput
    case unsupportedCodec
    case unsupportedIpldValue
    case nonCanonical
    case providerFailure
}

public enum ReallyMeMulticodecTag: Sendable {
    case encryption
    case hash
    case key
    case multihash
    case multikey
}

func sdkMulticodecTag(from tag: ReallyMeProtoCodecTag) throws(ReallyMeCodecError) -> ReallyMeMulticodecTag {
    switch tag {
    case .encryption: return .encryption
    case .hash: return .hash
    case .key: return .key
    case .multihash: return .multihash
    case .multikey: return .multikey
    case .unspecified, .UNRECOGNIZED: throw .providerFailure
    }
}

public enum ReallyMeKeyMaterialKind: Sendable {
    case notKey
    case publicKey
    case privateKey
    case symmetricKey
}

func sdkKeyMaterialKind(from kind: ReallyMeProtoCodecKeyMaterialKind) throws(ReallyMeCodecError) -> ReallyMeKeyMaterialKind {
    switch kind {
    case .notKey: return .notKey
    case .publicKey: return .publicKey
    case .privateKey: return .privateKey
    case .symmetricKey: return .symmetricKey
    case .unspecified, .UNRECOGNIZED: throw .providerFailure
    }
}

public enum ReallyMePemLabel: String, Sendable {
    case privateKey = "PRIVATE KEY"
    case ecPrivateKey = "EC PRIVATE KEY"
    case publicKey = "PUBLIC KEY"
}

func protoPemLabel(from value: ReallyMePemLabel) -> ReallyMeProtoCodecPemLabel {
    switch value {
    case .privateKey: return .privateKey
    case .ecPrivateKey: return .ecPrivateKey
    case .publicKey: return .publicKey
    }
}

public enum ReallyMePemLineEnding: Sendable {
    case lf
    case crlf
}

func protoPemLineEnding(from value: ReallyMePemLineEnding?) -> ReallyMeProtoCodecPemLineEnding {
    switch value {
    case .lf: return .lf
    case .crlf: return .crlf
    case nil: return .unspecified
    }
}
