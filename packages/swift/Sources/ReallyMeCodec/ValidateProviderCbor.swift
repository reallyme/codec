// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
import SwiftProtobuf

func validateProviderDeterministicCborValue(
    _ value: ReallyMeProtoCodecDeterministicCborValue
) throws(ReallyMeCodecError) {
    var state = DeterministicCborValidationState()
    try validateProviderDeterministicCborValue(value, depth: 0, state: &state)
}

private func validateProviderDeterministicCborValue(
    _ value: ReallyMeProtoCodecDeterministicCborValue,
    depth: Int,
    state: inout DeterministicCborValidationState
) throws(ReallyMeCodecError) {
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
) throws(ReallyMeCodecError) {
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
        guard
            compareProviderDeterministicCborMapKeys(
                entries[previous].key,
                entries[current].key
            ) != 0
        else {
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
) throws(ReallyMeCodecError) {
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
) throws(ReallyMeCodecError) {
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
) throws(ReallyMeCodecError) {
    let (remaining, overflow) =
        maxDeterministicCborAggregateTextBytes
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
func boundedDeterministicCborUtf8Length(
    _ text: String,
    maximum: Int,
    error: ReallyMeCodecError
) throws(ReallyMeCodecError) -> Int {
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

private func validateProviderDeterministicCborContainer(_ count: Int) throws(ReallyMeCodecError) {
    guard count <= maxDeterministicCborContainerEntries else {
        throw ReallyMeCodecError.providerFailure
    }
}

private func providerDeterministicCborChildDepth(_ depth: Int) throws(ReallyMeCodecError) -> Int {
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
) throws(ReallyMeCodecError) {
    let (next, overflow) = count.addingReportingOverflow(increment)
    guard !overflow, next <= maximum else {
        throw ReallyMeCodecError.providerFailure
    }
    count = next
}

private func requireNoProviderUnknownFields(
    _ unknownFields: SwiftProtobuf.UnknownStorage
) throws(ReallyMeCodecError) {
    guard unknownFields.data.isEmpty else {
        throw ReallyMeCodecError.providerFailure
    }
}
