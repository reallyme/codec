// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import Foundation
import ReallyMeCodecProto
import SwiftProtobuf

public struct ReallyMeDeterministicCborNegativeInteger:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible
{
    public let value: Int64

    public init(_ value: Int64) throws {
        guard value < 0 else {
            throw ReallyMeCodecError.invalidInput
        }
        self.value = value
    }

    fileprivate init(providerValue value: Int64) throws {
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
}

public enum ReallyMeDeterministicCborInteger:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible
{
    case unsigned(UInt64)
    case negative(ReallyMeDeterministicCborNegativeInteger)

    public var description: String {
        "ReallyMeDeterministicCborInteger(<redacted>)"
    }

    public var debugDescription: String {
        description
    }
}

public enum ReallyMeDeterministicCborMapKey:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible
{
    case integer(ReallyMeDeterministicCborInteger)
    case text(String)

    public var description: String {
        "ReallyMeDeterministicCborMapKey(<redacted>)"
    }

    public var debugDescription: String {
        description
    }
}

public struct ReallyMeDeterministicCborMapEntry:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible
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
}

public indirect enum ReallyMeDeterministicCborValue:
    Sendable, CustomStringConvertible, CustomDebugStringConvertible
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

    public static func negative(_ value: Int64) throws -> ReallyMeDeterministicCborValue {
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
        .map(entries.map { key, value in
            ReallyMeDeterministicCborMapEntry(
                key: .integer(.unsigned(key)),
                value: value
            )
        })
    }

    public static func mapText(
        _ entries: [(String, ReallyMeDeterministicCborValue)]
    ) -> ReallyMeDeterministicCborValue {
        .map(entries.map { key, value in
            ReallyMeDeterministicCborMapEntry(key: .text(key), value: value)
        })
    }

    public static func intKey(_ value: UInt64) -> ReallyMeDeterministicCborMapKey {
        .integer(.unsigned(value))
    }

    public static func intKey(_ value: Int64) throws -> ReallyMeDeterministicCborMapKey {
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

    public static func negative(_ value: Int64) throws -> ReallyMeDeterministicCborValue {
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
        .map(entries.map { key, value in
            ReallyMeDeterministicCborMapEntry(key: .text(key), value: value)
        })
    }
}

private let maxDeterministicCborNestingDepth = 64
private let maxDeterministicCborNodes = 65_536
private let maxDeterministicCborContainerEntries = 16_384
private let maxDeterministicCborAggregateTextBytes = 1_048_576
private let maxDeterministicCborAggregateByteStringBytes = 1_048_576
// The recursive protobuf representation can be materially larger than the
// canonical CBOR it carries. This transport limit is derived from the same
// semantic budgets as the Rust protobuf boundary so every valid semantic
// value remains reachable without making the public scalar FFI cap larger.
private let maxCodecProtoStructuralBytesPerDeterministicCborNode = 128
private let maxCodecProtoFixedDeterministicCborOperationBytes = 4_096
private let maxDeterministicCborProtoMessageBytes =
    maxDeterministicCborAggregateTextBytes
    + maxDeterministicCborAggregateByteStringBytes
    + (maxDeterministicCborNodes * maxCodecProtoStructuralBytesPerDeterministicCborNode)
    + maxCodecProtoFixedDeterministicCborOperationBytes
// One semantic map level expands to Value -> Map -> MapEntry. Five outer/key
// wrappers cover the deepest generated request/result path. SwiftProtobuf's
// default of 100 cannot carry the documented semantic depth of 64. The fully
// discriminated response adds OperationResponse and OperationResult outside
// the operation-specific result, so Swift requires seven outer/key wrappers.
let maxDeterministicCborProtoMessageDepth =
    (maxDeterministicCborNestingDepth * 3) + 7

private struct DeterministicCborValidationState {
    var nodes = 0
    var textBytes = 0
    var byteStringBytes = 0
}

private func validateDeterministicCborValue(
    _ value: ReallyMeDeterministicCborValue
) throws {
    var state = DeterministicCborValidationState()
    try validateDeterministicCborValue(value, depth: 0, state: &state)
}

private func validateDeterministicCborValue(
    _ value: ReallyMeDeterministicCborValue,
    depth: Int,
    state: inout DeterministicCborValidationState
) throws {
    try addDeterministicCborCount(&state.nodes, 1, maximum: maxDeterministicCborNodes)
    switch value {
    case .null, .bool, .integer:
        return
    case .text(let value):
        try addDeterministicCborTextBytes(value, state: &state)
    case .bytes(let value):
        try addDeterministicCborCount(
            &state.byteStringBytes,
            value.count,
            maximum: maxDeterministicCborAggregateByteStringBytes
        )
    case .array(let values):
        try validateDeterministicCborContainer(values.count)
        let childDepth = try deterministicCborChildDepth(depth)
        for child in values {
            try validateDeterministicCborValue(child, depth: childDepth, state: &state)
        }
    case .map(let entries):
        try validateDeterministicCborContainer(entries.count)
        let childDepth = try deterministicCborChildDepth(depth)
        for entry in entries {
            try addDeterministicCborCount(
                &state.nodes,
                1,
                maximum: maxDeterministicCborNodes
            )
            if case .text(let key) = entry.key {
                try addDeterministicCborTextBytes(key, state: &state)
            }
            try validateDeterministicCborValue(
                entry.value,
                depth: childDepth,
                state: &state
            )
        }
    }
}

private func addDeterministicCborTextBytes(
    _ text: String,
    state: inout DeterministicCborValidationState
) throws {
    let (remaining, overflow) = maxDeterministicCborAggregateTextBytes
        .subtractingReportingOverflow(state.textBytes)
    guard !overflow else {
        throw ReallyMeCodecError.invalidInput
    }
    let increment = try boundedDeterministicCborUtf8Length(
        text,
        maximum: remaining,
        error: .invalidInput
    )
    try addDeterministicCborCount(
        &state.textBytes,
        increment,
        maximum: maxDeterministicCborAggregateTextBytes
    )
}

private func validateDeterministicCborContainer(_ count: Int) throws {
    guard count <= maxDeterministicCborContainerEntries else {
        throw ReallyMeCodecError.invalidInput
    }
}

private func deterministicCborChildDepth(_ depth: Int) throws -> Int {
    let (childDepth, overflow) = depth.addingReportingOverflow(1)
    guard !overflow, childDepth <= maxDeterministicCborNestingDepth else {
        throw ReallyMeCodecError.invalidInput
    }
    return childDepth
}

private func addDeterministicCborCount(
    _ count: inout Int,
    _ increment: Int,
    maximum: Int
) throws {
    let (next, overflow) = count.addingReportingOverflow(increment)
    guard !overflow, next <= maximum else {
        throw ReallyMeCodecError.invalidInput
    }
    count = next
}

/// Validates a generated provider tree before creating a second managed owner
/// graph. This is boundary-shape and resource validation only; Rust remains
/// authoritative for deterministic-CBOR semantics and canonical bytes.
func validateProviderDeterministicCborValue(
    _ value: ReallyMeProtoCodecDeterministicCborValue
) throws {
    var state = DeterministicCborValidationState()
    try validateProviderDeterministicCborValue(value, depth: 0, state: &state)
}

private func validateProviderDeterministicCborValue(
    _ value: ReallyMeProtoCodecDeterministicCborValue,
    depth: Int,
    state: inout DeterministicCborValidationState
) throws {
    try requireNoProviderUnknownFields(value.unknownFields)
    try addProviderDeterministicCborCount(
        &state.nodes,
        1,
        maximum: maxDeterministicCborNodes
    )
    switch value.value {
    case .nullValue(let value):
        try requireNoProviderUnknownFields(value.unknownFields)
    case .boolValue(let value):
        try requireNoProviderUnknownFields(value.unknownFields)
    case .integerValue(let value):
        try validateProviderDeterministicCborInteger(value)
    case .textValue(let value):
        try requireNoProviderUnknownFields(value.unknownFields)
        try addProviderDeterministicCborTextBytes(value.value, state: &state)
    case .bytesValue(let value):
        try requireNoProviderUnknownFields(value.unknownFields)
        try addProviderDeterministicCborCount(
            &state.byteStringBytes,
            value.value.count,
            maximum: maxDeterministicCborAggregateByteStringBytes
        )
    case .arrayValue(let value):
        try requireNoProviderUnknownFields(value.unknownFields)
        try validateProviderDeterministicCborContainer(value.values.count)
        let childDepth = try providerDeterministicCborChildDepth(depth)
        for child in value.values {
            try validateProviderDeterministicCborValue(
                child,
                depth: childDepth,
                state: &state
            )
        }
    case .mapValue(let value):
        try requireNoProviderUnknownFields(value.unknownFields)
        try validateProviderDeterministicCborContainer(value.entries.count)
        let childDepth = try providerDeterministicCborChildDepth(depth)
        for entry in value.entries {
            try requireNoProviderUnknownFields(entry.unknownFields)
            guard entry.hasKey, entry.hasValue else {
                throw ReallyMeCodecError.providerFailure
            }
            try addProviderDeterministicCborCount(
                &state.nodes,
                1,
                maximum: maxDeterministicCborNodes
            )
            try validateProviderDeterministicCborKey(entry.key, state: &state)
        }
        try rejectDuplicateProviderDeterministicCborMapKeys(value.entries)
        for entry in value.entries {
            try validateProviderDeterministicCborValue(
                entry.value,
                depth: childDepth,
                state: &state
            )
        }
    case nil:
        throw ReallyMeCodecError.providerFailure
    }
}

private func rejectDuplicateProviderDeterministicCborMapKeys(
    _ entries: [ReallyMeProtoCodecDeterministicCborMapEntry]
) throws {
    var indices = Array(entries.indices)
    indices.sort {
        compareProviderDeterministicCborMapKeys(
            entries[$0].key,
            entries[$1].key
        ) < 0
    }
    guard indices.count > 1 else {
        return
    }
    for index in 1..<indices.count {
        let previous = indices[index - 1]
        let current = indices[index]
        guard compareProviderDeterministicCborMapKeys(
            entries[previous].key,
            entries[current].key
        ) != 0 else {
            throw ReallyMeCodecError.providerFailure
        }
    }
}

private func compareProviderDeterministicCborMapKeys(
    _ left: ReallyMeProtoCodecDeterministicCborMapKey,
    _ right: ReallyMeProtoCodecDeterministicCborMapKey
) -> Int {
    switch (left.key, right.key) {
    case (.integerKey(let left), .integerKey(let right)):
        return compareProviderDeterministicCborIntegers(left, right)
    case (.integerKey, .textKey):
        return -1
    case (.textKey, .integerKey):
        return 1
    case (.textKey(let left), .textKey(let right)):
        return compareDeterministicCborUtf8(left.value, right.value)
    case (nil, nil):
        return 0
    case (nil, _):
        return 1
    case (_, nil):
        return -1
    }
}

private func compareProviderDeterministicCborIntegers(
    _ left: ReallyMeProtoCodecDeterministicCborInteger,
    _ right: ReallyMeProtoCodecDeterministicCborInteger
) -> Int {
    switch (left.value, right.value) {
    case (.unsignedValue(let left), .unsignedValue(let right)):
        return compareDeterministicCborComparable(left.value, right.value)
    case (.unsignedValue, .negativeValue):
        return -1
    case (.negativeValue, .unsignedValue):
        return 1
    case (.negativeValue(let left), .negativeValue(let right)):
        return compareDeterministicCborComparable(left.value, right.value)
    case (nil, nil):
        return 0
    case (nil, _):
        return 1
    case (_, nil):
        return -1
    }
}

private func compareDeterministicCborComparable<T: Comparable>(
    _ left: T,
    _ right: T
) -> Int {
    if left < right {
        return -1
    }
    if left > right {
        return 1
    }
    return 0
}

// Swift String equality applies Unicode canonical equivalence. CBOR text-key
// equality is instead exact UTF-8 byte equality, with no normalization.
private func compareDeterministicCborUtf8(_ left: String, _ right: String) -> Int {
    var leftIterator = left.utf8.makeIterator()
    var rightIterator = right.utf8.makeIterator()
    while true {
        switch (leftIterator.next(), rightIterator.next()) {
        case (nil, nil):
            return 0
        case (nil, _):
            return -1
        case (_, nil):
            return 1
        case (.some(let leftByte), .some(let rightByte)):
            if leftByte < rightByte {
                return -1
            }
            if leftByte > rightByte {
                return 1
            }
        }
    }
}

private func validateProviderDeterministicCborInteger(
    _ integer: ReallyMeProtoCodecDeterministicCborInteger
) throws {
    try requireNoProviderUnknownFields(integer.unknownFields)
    switch integer.value {
    case .unsignedValue(let value):
        try requireNoProviderUnknownFields(value.unknownFields)
    case .negativeValue(let value):
        try requireNoProviderUnknownFields(value.unknownFields)
        guard value.value < 0 else {
            throw ReallyMeCodecError.providerFailure
        }
    case nil:
        throw ReallyMeCodecError.providerFailure
    }
}

private func validateProviderDeterministicCborKey(
    _ key: ReallyMeProtoCodecDeterministicCborMapKey,
    state: inout DeterministicCborValidationState
) throws {
    try requireNoProviderUnknownFields(key.unknownFields)
    switch key.key {
    case .integerKey(let value):
        try validateProviderDeterministicCborInteger(value)
    case .textKey(let value):
        try requireNoProviderUnknownFields(value.unknownFields)
        try addProviderDeterministicCborTextBytes(value.value, state: &state)
    case nil:
        throw ReallyMeCodecError.providerFailure
    }
}

private func addProviderDeterministicCborTextBytes(
    _ text: String,
    state: inout DeterministicCborValidationState
) throws {
    let (remaining, overflow) = maxDeterministicCborAggregateTextBytes
        .subtractingReportingOverflow(state.textBytes)
    guard !overflow else {
        throw ReallyMeCodecError.providerFailure
    }
    let increment = try boundedDeterministicCborUtf8Length(
        text,
        maximum: remaining,
        error: .providerFailure
    )
    try addProviderDeterministicCborCount(
        &state.textBytes,
        increment,
        maximum: maxDeterministicCborAggregateTextBytes
    )
}

// `String.UTF8View.count` must traverse the complete string before the limit
// can be checked. Stop at the first byte beyond the remaining budget so a
// hostile managed string cannot turn a 1 MiB semantic limit into unbounded
// validation work before protobuf construction or provider-tree copying.
private func boundedDeterministicCborUtf8Length(
    _ text: String,
    maximum: Int,
    error: ReallyMeCodecError
) throws -> Int {
    var length = 0
    for _ in text.utf8 {
        let (next, overflow) = length.addingReportingOverflow(1)
        guard !overflow, next <= maximum else {
            throw error
        }
        length = next
    }
    return length
}

private func validateProviderDeterministicCborContainer(_ count: Int) throws {
    guard count <= maxDeterministicCborContainerEntries else {
        throw ReallyMeCodecError.providerFailure
    }
}

private func providerDeterministicCborChildDepth(_ depth: Int) throws -> Int {
    let (childDepth, overflow) = depth.addingReportingOverflow(1)
    guard !overflow, childDepth <= maxDeterministicCborNestingDepth else {
        throw ReallyMeCodecError.providerFailure
    }
    return childDepth
}

private func addProviderDeterministicCborCount(
    _ count: inout Int,
    _ increment: Int,
    maximum: Int
) throws {
    let (next, overflow) = count.addingReportingOverflow(increment)
    guard !overflow, next <= maximum else {
        throw ReallyMeCodecError.providerFailure
    }
    count = next
}

private func requireNoProviderUnknownFields(
    _ unknownFields: SwiftProtobuf.UnknownStorage
) throws {
    guard unknownFields.data.isEmpty else {
        throw ReallyMeCodecError.providerFailure
    }
}

public extension ReallyMeCodec {
    func deterministicCborEncode(_ value: ReallyMeDeterministicCborValue) throws -> [UInt8] {
        try validateDeterministicCborValue(value)
        var request = ReallyMeProtoCodecDeterministicCborEncodeRequest()
        request.value = protoValue(value)
        defer {
            clearProtoEncodeRequest(&request)
        }
        var operationResult = try withOwnedBytes(
            deterministicCborOperationRequest(.deterministicCborEncode(request))
        ) { request in
            try processGeneratedOperation(request: request)
        }
        guard case .deterministicCborEncode(var result)? = operationResult.result else {
            throw ReallyMeCodecError.providerFailure
        }
        operationResult.result = nil
        defer {
            ReallyMeCodecMemory.clearOwned(&result.encoded)
        }
        guard result.unknownFields.data.isEmpty else {
            throw ReallyMeCodecError.providerFailure
        }
        return Array(result.encoded)
    }

    func deterministicCborDecode(_ bytes: [UInt8]) throws -> ReallyMeDeterministicCborValue {
        try requireBoundaryAggregate([bytes.count], maxFfiInputLength: provider.ffiInputLimit)
        var request = ReallyMeProtoCodecDeterministicCborDecodeRequest()
        request.encoded = Data(bytes)
        defer {
            ReallyMeCodecMemory.clearOwned(&request.encoded)
        }
        var operationResult = try withOwnedBytes(
            deterministicCborOperationRequest(.deterministicCborDecode(request))
        ) { request in
            try processGeneratedOperation(request: request)
        }
        guard case .deterministicCborDecode(var result)? = operationResult.result else {
            throw ReallyMeCodecError.providerFailure
        }
        operationResult.result = nil
        defer {
            clearProtoDecodeResult(&result)
        }
        guard result.unknownFields.data.isEmpty, result.hasValue else {
            throw ReallyMeCodecError.providerFailure
        }
        try validateProviderDeterministicCborValue(result.value)
        return try sdkValue(result.value)
    }

    func dagCborEncode(_ value: ReallyMeDeterministicCborValue) throws -> [UInt8] {
        try validateDeterministicCborValue(value)
        var request = ReallyMeProtoCodecDagCborEncodeRequest()
        request.value = protoValue(value)
        defer {
            if request.hasValue {
                var value = request.value
                request.clearValue()
                clearProtoValue(&value)
            }
        }
        var operationResult = try withOwnedBytes(
            deterministicCborOperationRequest(.dagCborEncode(request))
        ) { request in
            try processGeneratedOperation(request: request)
        }
        guard case .dagCborEncode(var result)? = operationResult.result else {
            throw ReallyMeCodecError.providerFailure
        }
        operationResult.result = nil
        defer {
            ReallyMeCodecMemory.clearOwned(&result.encoded)
        }
        guard result.unknownFields.data.isEmpty else {
            throw ReallyMeCodecError.providerFailure
        }
        return Array(result.encoded)
    }

    func dagCborDecode(_ bytes: [UInt8]) throws -> ReallyMeDeterministicCborValue {
        try requireBoundaryAggregate([bytes.count], maxFfiInputLength: provider.ffiInputLimit)
        var request = ReallyMeProtoCodecDagCborDecodeRequest()
        request.encoded = Data(bytes)
        defer {
            ReallyMeCodecMemory.clearOwned(&request.encoded)
        }
        var operationResult = try withOwnedBytes(
            deterministicCborOperationRequest(.dagCborDecode(request))
        ) { request in
            try processGeneratedOperation(request: request)
        }
        guard case .dagCborDecode(var result)? = operationResult.result else {
            throw ReallyMeCodecError.providerFailure
        }
        operationResult.result = nil
        defer {
            if result.hasValue {
                var value = result.value
                result.clearValue()
                clearProtoValue(&value)
            }
        }
        guard result.unknownFields.data.isEmpty, result.hasValue else {
            throw ReallyMeCodecError.providerFailure
        }
        try validateProviderDeterministicCborValue(result.value)
        return try sdkValue(result.value)
    }
}

private func clearProtoEncodeRequest(
    _ request: inout ReallyMeProtoCodecDeterministicCborEncodeRequest
) {
    guard request.hasValue else {
        return
    }
    var value = request.value
    request.clearValue()
    clearProtoValue(&value)
}

private func clearProtoDecodeResult(
    _ result: inout ReallyMeProtoCodecDeterministicCborDecodeResult
) {
    guard result.hasValue else {
        return
    }
    var value = result.value
    result.clearValue()
    clearProtoValue(&value)
}

// SwiftProtobuf recursive messages are value types with copy-on-write storage.
// The reliable cleanup boundary is the serialized request/result byte owner;
// this walk first detaches oneof parents, then wipes the surviving local owner
// so Data/array copy-on-write does not preserve the dropped generated storage.
private func clearProtoValue(_ value: inout ReallyMeProtoCodecDeterministicCborValue) {
    let detachedValue = value.value
    value.value = nil
    switch detachedValue {
    case .bytesValue(var bytes):
        ReallyMeCodecMemory.clearOwned(&bytes.value)
    case .arrayValue(var array):
        for index in array.values.indices {
            clearProtoValue(&array.values[index])
        }
        array.values.removeAll(keepingCapacity: false)
    case .mapValue(var map):
        for index in map.entries.indices {
            clearProtoMapEntry(&map.entries[index])
        }
        map.entries.removeAll(keepingCapacity: false)
    case .integerValue, .textValue, .boolValue, .nullValue, nil:
        break
    }
}

private func clearProtoMapEntry(_ entry: inout ReallyMeProtoCodecDeterministicCborMapEntry) {
    if entry.hasValue {
        var value = entry.value
        entry.clearValue()
        clearProtoValue(&value)
    }
    entry.clearKey()
}

private func deterministicCborOperationRequest(
    _ operation: ReallyMeProtoCodecOperationRequest.OneOf_Operation
) throws -> [UInt8] {
    var request = ReallyMeProtoCodecOperationRequest()
    request.operation = operation
    var serialized: Data
    do {
        serialized = try request.serializedData()
    } catch {
        throw ReallyMeCodecError.providerFailure
    }
    defer {
        ReallyMeCodecMemory.clearOwned(&serialized)
    }
    guard serialized.count <= maxDeterministicCborProtoMessageBytes else {
        throw ReallyMeCodecError.invalidInput
    }
    return Array(serialized)
}

private func protoInteger(
    _ integer: ReallyMeDeterministicCborInteger
) -> ReallyMeProtoCodecDeterministicCborInteger {
    var result = ReallyMeProtoCodecDeterministicCborInteger()
    switch integer {
    case .unsigned(let value):
        var unsigned = ReallyMeProtoCodecDeterministicCborUnsignedInteger()
        unsigned.value = value
        result.unsignedValue = unsigned
    case .negative(let value):
        var negative = ReallyMeProtoCodecDeterministicCborNegativeInteger()
        negative.value = value.value
        result.negativeValue = negative
    }
    return result
}

private func protoKey(
    _ key: ReallyMeDeterministicCborMapKey
) -> ReallyMeProtoCodecDeterministicCborMapKey {
    var result = ReallyMeProtoCodecDeterministicCborMapKey()
    switch key {
    case .integer(let value):
        result.integerKey = protoInteger(value)
    case .text(let value):
        var text = ReallyMeProtoCodecDeterministicCborText()
        text.value = value
        result.textKey = text
    }
    return result
}

private func protoValue(
    _ value: ReallyMeDeterministicCborValue
) -> ReallyMeProtoCodecDeterministicCborValue {
    var result = ReallyMeProtoCodecDeterministicCborValue()
    switch value {
    case .null:
        result.nullValue = ReallyMeProtoCodecDeterministicCborNull()
    case .bool(let value):
        var bool = ReallyMeProtoCodecDeterministicCborBool()
        bool.value = value
        result.boolValue = bool
    case .integer(let value):
        result.integerValue = protoInteger(value)
    case .text(let value):
        var text = ReallyMeProtoCodecDeterministicCborText()
        text.value = value
        result.textValue = text
    case .bytes(let value):
        var bytes = ReallyMeProtoCodecDeterministicCborBytes()
        bytes.value = Data(value)
        result.bytesValue = bytes
    case .array(let values):
        var array = ReallyMeProtoCodecDeterministicCborArray()
        array.values = values.map(protoValue)
        result.arrayValue = array
    case .map(let entries):
        var map = ReallyMeProtoCodecDeterministicCborMap()
        map.entries = entries.map { entry in
            var protoEntry = ReallyMeProtoCodecDeterministicCborMapEntry()
            protoEntry.key = protoKey(entry.key)
            protoEntry.value = protoValue(entry.value)
            return protoEntry
        }
        result.mapValue = map
    }
    return result
}

private func sdkInteger(
    _ integer: ReallyMeProtoCodecDeterministicCborInteger
) throws -> ReallyMeDeterministicCborInteger {
    switch integer.value {
    case .unsignedValue(let value):
        return .unsigned(value.value)
    case .negativeValue(let value):
        return .negative(try ReallyMeDeterministicCborNegativeInteger(providerValue: value.value))
    case nil:
        throw ReallyMeCodecError.providerFailure
    }
}

private func sdkKey(
    _ key: ReallyMeProtoCodecDeterministicCborMapKey
) throws -> ReallyMeDeterministicCborMapKey {
    switch key.key {
    case .integerKey(let value):
        return .integer(try sdkInteger(value))
    case .textKey(let value):
        return .text(value.value)
    case nil:
        throw ReallyMeCodecError.providerFailure
    }
}

private func sdkValue(
    _ value: ReallyMeProtoCodecDeterministicCborValue
) throws -> ReallyMeDeterministicCborValue {
    switch value.value {
    case .nullValue:
        return .null
    case .boolValue(let value):
        return .bool(value.value)
    case .integerValue(let value):
        return .integer(try sdkInteger(value))
    case .textValue(let value):
        return .text(value.value)
    case .bytesValue(let value):
        return .bytes(Array(value.value))
    case .arrayValue(let value):
        return .array(try value.values.map(sdkValue))
    case .mapValue(let value):
        return .map(
            try value.entries.map { entry in
                guard entry.hasKey, entry.hasValue else {
                    throw ReallyMeCodecError.providerFailure
                }
                return ReallyMeDeterministicCborMapEntry(
                    key: try sdkKey(entry.key),
                    value: try sdkValue(entry.value)
                )
            }
        )
    case nil:
        throw ReallyMeCodecError.providerFailure
    }
}
