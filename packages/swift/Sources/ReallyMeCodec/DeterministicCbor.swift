// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
import SwiftProtobuf

public struct ReallyMeDeterministicCborNegativeInteger:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable
{
    public let value: Int64

    public init(_ value: Int64) throws(ReallyMeCodecError) {
        guard value < 0 else {
            throw ReallyMeCodecError.invalidInput
        }
        self.value = value
    }

    init(providerValue value: Int64) throws(ReallyMeCodecError) {
        guard value < 0 else {
            throw ReallyMeCodecError.providerFailure
        }
        self.value = value
    }

    public var description: String {
        "ReallyMeDeterministicCborNegativeInteger(<redacted>)"
    }

    public var debugDescription: String {
        description
    }

    public var customMirror: Mirror { Mirror(self, children: []) }
}

public enum ReallyMeDeterministicCborInteger:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable
{
    case unsigned(UInt64)
    case negative(ReallyMeDeterministicCborNegativeInteger)

    public var description: String {
        "ReallyMeDeterministicCborInteger(<redacted>)"
    }

    public var debugDescription: String {
        description
    }

    public var customMirror: Mirror { Mirror(self, children: []) }
}

public enum ReallyMeDeterministicCborMapKey:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable
{
    case integer(ReallyMeDeterministicCborInteger)
    case text(String)

    public var description: String {
        "ReallyMeDeterministicCborMapKey(<redacted>)"
    }

    public var debugDescription: String {
        description
    }

    public var customMirror: Mirror { Mirror(self, children: []) }
}

public struct ReallyMeDeterministicCborMapEntry:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable
{
    public let key: ReallyMeDeterministicCborMapKey
    public let value: ReallyMeDeterministicCborValue

    public init(key: ReallyMeDeterministicCborMapKey, value: ReallyMeDeterministicCborValue) {
        self.key = key
        self.value = value
    }

    public var description: String {
        "ReallyMeDeterministicCborMapEntry(<redacted>)"
    }

    public var debugDescription: String {
        description
    }

    public var customMirror: Mirror { Mirror(self, children: []) }
}

public indirect enum ReallyMeDeterministicCborValue:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable
{
    case null
    case bool(Bool)
    case integer(ReallyMeDeterministicCborInteger)
    case text(String)
    case bytes([UInt8])
    case array([ReallyMeDeterministicCborValue])
    case map([ReallyMeDeterministicCborMapEntry])

    public var description: String {
        "ReallyMeDeterministicCborValue(<redacted>)"
    }

    public var debugDescription: String {
        description
    }

    public var customMirror: Mirror { Mirror(self, children: []) }
}

public enum ReallyMeDeterministicCbor {
    public static func null() -> ReallyMeDeterministicCborValue {
        .null
    }

    public static func bool(_ value: Bool) -> ReallyMeDeterministicCborValue {
        .bool(value)
    }

    public static func unsigned(_ value: UInt64) -> ReallyMeDeterministicCborValue {
        .integer(.unsigned(value))
    }

    public static func negative(_ value: Int64) throws(ReallyMeCodecError) -> ReallyMeDeterministicCborValue {
        .integer(.negative(try ReallyMeDeterministicCborNegativeInteger(value)))
    }

    public static func text(_ value: String) -> ReallyMeDeterministicCborValue {
        .text(value)
    }

    public static func bytes(_ value: [UInt8]) -> ReallyMeDeterministicCborValue {
        .bytes(value)
    }

    public static func bytes(_ value: Data) -> ReallyMeDeterministicCborValue {
        .bytes(Array(value))
    }

    public static func array(
        _ values: [ReallyMeDeterministicCborValue]
    ) -> ReallyMeDeterministicCborValue {
        .array(values)
    }

    public static func mapInt(
        _ entries: [(UInt64, ReallyMeDeterministicCborValue)]
    ) -> ReallyMeDeterministicCborValue {
        .map(
            entries.map { key, value in
                ReallyMeDeterministicCborMapEntry(
                    key: .integer(.unsigned(key)),
                    value: value
                )
            })
    }

    public static func mapText(
        _ entries: [(String, ReallyMeDeterministicCborValue)]
    ) -> ReallyMeDeterministicCborValue {
        .map(
            entries.map { key, value in
                ReallyMeDeterministicCborMapEntry(key: .text(key), value: value)
            })
    }

    public static func intKey(_ value: UInt64) -> ReallyMeDeterministicCborMapKey {
        .integer(.unsigned(value))
    }

    public static func intKey(_ value: Int64) throws(ReallyMeCodecError) -> ReallyMeDeterministicCborMapKey {
        if value >= 0 {
            return .integer(.unsigned(UInt64(value)))
        }
        return .integer(.negative(try ReallyMeDeterministicCborNegativeInteger(value)))
    }

    public static func textKey(_ value: String) -> ReallyMeDeterministicCborMapKey {
        .text(value)
    }

    public static func entry(
        key: ReallyMeDeterministicCborMapKey,
        value: ReallyMeDeterministicCborValue
    ) -> ReallyMeDeterministicCborMapEntry {
        ReallyMeDeterministicCborMapEntry(key: key, value: value)
    }
}

/// Builders for the stricter DAG-CBOR profile.
///
/// DAG-CBOR permits only text map keys. The value representation remains the
/// shared recursive model so the generated protobuf contract stays singular,
/// but this namespace deliberately does not expose deterministic-CBOR-only
/// integer-key map helpers.
public enum ReallyMeDagCbor {
    public static func null() -> ReallyMeDeterministicCborValue {
        .null
    }

    public static func bool(_ value: Bool) -> ReallyMeDeterministicCborValue {
        .bool(value)
    }

    public static func unsigned(_ value: UInt64) -> ReallyMeDeterministicCborValue {
        .integer(.unsigned(value))
    }

    public static func negative(_ value: Int64) throws(ReallyMeCodecError) -> ReallyMeDeterministicCborValue {
        .integer(.negative(try ReallyMeDeterministicCborNegativeInteger(value)))
    }

    public static func text(_ value: String) -> ReallyMeDeterministicCborValue {
        .text(value)
    }

    public static func bytes(_ value: [UInt8]) -> ReallyMeDeterministicCborValue {
        .bytes(value)
    }

    public static func bytes(_ value: Data) -> ReallyMeDeterministicCborValue {
        .bytes(Array(value))
    }

    public static func array(
        _ values: [ReallyMeDeterministicCborValue]
    ) -> ReallyMeDeterministicCborValue {
        .array(values)
    }

    public static func mapText(
        _ entries: [(String, ReallyMeDeterministicCborValue)]
    ) -> ReallyMeDeterministicCborValue {
        .map(
            entries.map { key, value in
                ReallyMeDeterministicCborMapEntry(key: .text(key), value: value)
            })
    }
}
