// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
import SwiftProtobuf

public extension ReallyMeCodec {
    func deterministicCborEncode(_ value: ReallyMeDeterministicCborValue) throws(ReallyMeCodecError) -> [UInt8] {
        try validateDeterministicCborValue(value)
        var request = ReallyMeProtoCodecDeterministicCborEncodeRequest()
        request.value = protoValue(value)
        defer {
            clearProtoEncodeRequest(&request)
        }
        var operationResult = try withOwnedBytes(
            deterministicCborOperationRequest(.deterministicCborEncode(request))
        ) { (request) throws(ReallyMeCodecError) in
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

    func deterministicCborDecode(_ bytes: [UInt8]) throws(ReallyMeCodecError) -> ReallyMeDeterministicCborValue {
        try requireBoundaryAggregate([bytes.count], maxFfiInputLength: provider.ffiInputLimit)
        var request = ReallyMeProtoCodecDeterministicCborDecodeRequest()
        request.encoded = Data(bytes)
        defer {
            ReallyMeCodecMemory.clearOwned(&request.encoded)
        }
        var operationResult = try withOwnedBytes(
            deterministicCborOperationRequest(.deterministicCborDecode(request))
        ) { (request) throws(ReallyMeCodecError) in
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

    func dagCborEncode(_ value: ReallyMeDeterministicCborValue) throws(ReallyMeCodecError) -> [UInt8] {
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
        ) { (request) throws(ReallyMeCodecError) in
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

    func dagCborDecode(_ bytes: [UInt8]) throws(ReallyMeCodecError) -> ReallyMeDeterministicCborValue {
        try requireBoundaryAggregate([bytes.count], maxFfiInputLength: provider.ffiInputLimit)
        var request = ReallyMeProtoCodecDagCborDecodeRequest()
        request.encoded = Data(bytes)
        defer {
            ReallyMeCodecMemory.clearOwned(&request.encoded)
        }
        var operationResult = try withOwnedBytes(
            deterministicCborOperationRequest(.dagCborDecode(request))
        ) { (request) throws(ReallyMeCodecError) in
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

// SwiftProtobuf recursive messages use copy-on-write storage. Move each oneof
// out of its parent and release the switch source before mutating a byte field.
// A live second owner would make Data allocate a copy and leave the original
// bytes unwiped. The serialized request/result buffer has a separate wipe;
// managed-runtime ownership outside this tree cannot be guaranteed here.
private func clearProtoValue(_ value: inout ReallyMeProtoCodecDeterministicCborValue) {
    var detachedValue: ReallyMeProtoCodecDeterministicCborValue.OneOf_Value? = nil
    swap(&value.value, &detachedValue)
    switch detachedValue {
    case .bytesValue(var bytes):
        detachedValue = nil
        ReallyMeCodecMemory.clearOwned(&bytes.value)
    case .arrayValue(var array):
        detachedValue = nil
        for index in array.values.indices {
            clearProtoValue(&array.values[index])
        }
        array.values.removeAll(keepingCapacity: false)
    case .mapValue(var map):
        detachedValue = nil
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
) throws(ReallyMeCodecError) -> [UInt8] {
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
) throws(ReallyMeCodecError) -> ReallyMeDeterministicCborInteger {
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
) throws(ReallyMeCodecError) -> ReallyMeDeterministicCborMapKey {
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
) throws(ReallyMeCodecError) -> ReallyMeDeterministicCborValue {
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
            try value.entries.map { (entry) throws(ReallyMeCodecError) in
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
