// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
import SwiftProtobuf

private func operationRequest(
    _ operation: ReallyMeProtoCodecOperationRequest.OneOf_Operation,
    maxFfiInputLength: Int
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
    guard serialized.count <= maxFfiInputLength else {
        throw ReallyMeCodecError.invalidInput
    }
    return Array(serialized)
}

func multicodecPrefixForNameOperationRequestBytes(
    _ name: String,
    maxFfiInputLength: Int
) throws(ReallyMeCodecError) -> [UInt8] {
    try requireBoundaryAggregate([name.utf8.count], maxFfiInputLength: maxFfiInputLength)
    var request = ReallyMeProtoCodecMulticodecPrefixForNameRequest()
    request.name = name
    return try operationRequest(.multicodecPrefixForName(request), maxFfiInputLength: maxFfiInputLength)
}

func multicodecLookupPrefixOperationRequestBytes(
    _ bytes: [UInt8],
    maxFfiInputLength: Int
) throws(ReallyMeCodecError) -> [UInt8] {
    try requireBoundaryAggregate([bytes.count], maxFfiInputLength: maxFfiInputLength)
    var request = ReallyMeProtoCodecMulticodecLookupPrefixRequest()
    request.value = Data(bytes)
    defer {
        ReallyMeCodecMemory.clearOwned(&request.value)
    }
    return try operationRequest(.multicodecLookupPrefix(request), maxFfiInputLength: maxFfiInputLength)
}

func multicodecTableOperationRequestBytes(maxFfiInputLength: Int) throws(ReallyMeCodecError) -> [UInt8] {
    try operationRequest(
        .multicodecTable(ReallyMeProtoCodecMulticodecTableRequest()),
        maxFfiInputLength: maxFfiInputLength
    )
}

func multikeyParseOperationRequestBytes(
    _ multikey: String,
    maxFfiInputLength: Int
) throws(ReallyMeCodecError) -> [UInt8] {
    try requireBoundaryAggregate([multikey.utf8.count], maxFfiInputLength: maxFfiInputLength)
    var request = ReallyMeProtoCodecMultikeyParseRequest()
    request.multikey = multikey
    return try operationRequest(.multikeyParse(request), maxFfiInputLength: maxFfiInputLength)
}

func dagCborVerifyCidOperationRequestBytes(
    cid: String,
    payload: [UInt8],
    maxFfiInputLength: Int
) throws(ReallyMeCodecError) -> [UInt8] {
    try requireBoundaryAggregate([cid.utf8.count, payload.count], maxFfiInputLength: maxFfiInputLength)
    var request = ReallyMeProtoCodecDagCborVerifyCidRequest()
    request.cid = cid
    request.payload = Data(payload)
    defer {
        ReallyMeCodecMemory.clearOwned(&request.payload)
    }
    return try operationRequest(.dagCborVerifyCid(request), maxFfiInputLength: maxFfiInputLength)
}

func pemDecodeOperationRequestBytes(
    pem: [UInt8],
    options: ReallyMePemDecodeOptions,
    maxFfiInputLength: Int
) throws(ReallyMeCodecError) -> [UInt8] {
    try requireBoundaryAggregate([pem.count], maxFfiInputLength: maxFfiInputLength)
    var request = ReallyMeProtoCodecPemDecodeRequest()
    request.pem = Data(pem)
    defer {
        ReallyMeCodecMemory.clearOwned(&request.pem)
    }
    var protoOptions = ReallyMeProtoCodecPemDecodeOptions()
    protoOptions.allowedLabels = options.allowedLabels.map { protoPemLabel(from: $0) }
    protoOptions.maxInputLen = options.maxInputLen ?? 0
    protoOptions.maxDerLen = options.maxDerLen ?? 0
    request.options = protoOptions
    return try operationRequest(.pemDecode(request), maxFfiInputLength: maxFfiInputLength)
}

func pemEncodeOperationRequestBytes(
    label: ReallyMePemLabel,
    der: [UInt8],
    options: ReallyMePemEncodeOptions,
    maxFfiInputLength: Int
) throws(ReallyMeCodecError) -> [UInt8] {
    if let lineWidth = options.lineWidth, lineWidth > 76 {
        throw ReallyMeCodecError.invalidInput
    }
    try requireBoundaryAggregate([der.count], maxFfiInputLength: maxFfiInputLength)
    var request = ReallyMeProtoCodecPemEncodeRequest()
    request.label = protoPemLabel(from: label)
    request.der = Data(der)
    defer {
        ReallyMeCodecMemory.clearOwned(&request.der)
    }
    var protoOptions = ReallyMeProtoCodecPemEncodeOptions()
    protoOptions.maxDerLen = options.maxDerLen ?? 0
    protoOptions.lineWidth = options.lineWidth ?? 0
    protoOptions.lineEnding = protoPemLineEnding(from: options.lineEnding)
    request.options = protoOptions
    return try operationRequest(.pemEncode(request), maxFfiInputLength: maxFfiInputLength)
}
