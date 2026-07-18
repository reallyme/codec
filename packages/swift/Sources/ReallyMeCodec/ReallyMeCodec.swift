// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import Foundation
import ReallyMeCodecProto
import SwiftProtobuf

private enum CodecOperation {
    static let base64Encode: UInt32 = 1
    static let base64Decode: UInt32 = 2
    static let base64urlEncode: UInt32 = 3
    static let base64urlDecode: UInt32 = 4
    static let lowerHexEncode: UInt32 = 5
    static let lowerHexDecode: UInt32 = 6
    static let base58btcEncode: UInt32 = 7
    static let base58btcDecode: UInt32 = 8
    static let multibaseBase58btcEncode: UInt32 = 9
    static let multibaseBase64urlEncode: UInt32 = 10
    static let multibaseDecode: UInt32 = 11
    static let multicodecStripPrefix: UInt32 = 14
    static let multikeyEncode: UInt32 = 16
    static let requireSupportedMulticodec: UInt32 = 18
    static let dagCborComputeCid: UInt32 = 21
    static let dagCborSha256ContentHash: UInt32 = 23
    static let dagCborMultihash: UInt32 = 24
    static let tryParseCid: UInt32 = 25
    static let dagCborCodecCode: UInt32 = 26
    static let canonicalizeJson: UInt32 = 27
    static let validateKeyBinding: UInt32 = 30
}

private enum CodecBoolOperation {
    static let bindingTypeMatchesCodec: UInt32 = 1
    static let isValidCidString: UInt32 = 2
}

/// Swift facade for ReallyMe codec operations backed by the Rust codec crates.
public struct ReallyMeCodec: Sendable {
    let provider: ReallyMeCodecRustCAbiProvider

    /// Creates a codec backed by the Rust FFI library linked through the
    /// SwiftPM binary target shipped with the public package.
    public init() throws {
        #if REALLYME_CODEC_LINKED_FFI
        provider = try ReallyMeCodecRustCAbiProvider()
        #else
        throw ReallyMeCodecError.providerFailure
        #endif
    }

    public init(rustCAbiLibrary: ReallyMeCodecRustCAbiLibrary) throws {
        provider = try ReallyMeCodecRustCAbiProvider(library: rustCAbiLibrary)
    }

    public func base64Encode(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.base64Encode, first: bytes))
    }

    public func base64Decode(_ text: String) throws -> [UInt8] {
        try withTextBytes(text, maxFfiInputLength: provider.ffiInputLimit) { encoded in
            try provider.process(operation: CodecOperation.base64Decode, first: encoded)
        }
    }

    public func base64urlEncode(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.base64urlEncode, first: bytes))
    }

    public func base64urlDecode(_ text: String) throws -> [UInt8] {
        try withTextBytes(text, maxFfiInputLength: provider.ffiInputLimit) { encoded in
            try provider.process(operation: CodecOperation.base64urlDecode, first: encoded)
        }
    }

    public func bytesToLowerHex(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.lowerHexEncode, first: bytes))
    }

    public func lowerHexToBytes(_ text: String) throws -> [UInt8] {
        try withTextBytes(text, maxFfiInputLength: provider.ffiInputLimit) { encoded in
            try provider.process(operation: CodecOperation.lowerHexDecode, first: encoded)
        }
    }

    public func base58btcEncode(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.base58btcEncode, first: bytes))
    }

    public func base58btcDecode(_ text: String) throws -> [UInt8] {
        try withTextBytes(text, maxFfiInputLength: provider.ffiInputLimit) { encoded in
            try provider.process(operation: CodecOperation.base58btcDecode, first: encoded)
        }
    }

    public func multibaseBase58btcEncode(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.multibaseBase58btcEncode, first: bytes))
    }

    public func multibaseBase64urlEncode(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.multibaseBase64urlEncode, first: bytes))
    }

    public func multibaseDecode(_ text: String) throws -> [UInt8] {
        try withTextBytes(text, maxFfiInputLength: provider.ffiInputLimit) { encoded in
            try provider.process(operation: CodecOperation.multibaseDecode, first: encoded)
        }
    }

    public func multicodecPrefixForName(_ name: String) throws -> ReallyMeMulticodecMetadata {
        let operationResult = try withOwnedBytes(
            multicodecPrefixForNameOperationRequestBytes(name, maxFfiInputLength: provider.ffiInputLimit)
        ) { request in
            try processGeneratedOperation(request: request)
        }
        guard case .multicodecPrefixForName(let result)? = operationResult.result else {
            throw ReallyMeCodecError.providerFailure
        }
        return try sdkMulticodecMetadata(from: result)
    }

    public func multicodecLookupPrefix(_ bytes: [UInt8]) throws -> ReallyMeMulticodecLookupResult {
        let operationResult = try withOwnedBytes(
            multicodecLookupPrefixOperationRequestBytes(bytes, maxFfiInputLength: provider.ffiInputLimit)
        ) { request in
            try processGeneratedOperation(request: request)
        }
        guard case .multicodecLookupPrefix(let result)? = operationResult.result else {
            throw ReallyMeCodecError.providerFailure
        }
        return try sdkMulticodecLookupResult(from: result)
    }

    public func multicodecStripPrefix(_ bytes: [UInt8]) throws -> [UInt8] {
        try provider.process(operation: CodecOperation.multicodecStripPrefix, first: bytes)
    }

    public func multicodecTable() throws -> ReallyMeMulticodecTable {
        let operationResult = try withOwnedBytes(
            multicodecTableOperationRequestBytes(maxFfiInputLength: provider.ffiInputLimit)
        ) { request in
            try processGeneratedOperation(request: request)
        }
        guard case .multicodecTable(let result)? = operationResult.result else {
            throw ReallyMeCodecError.providerFailure
        }
        return try sdkMulticodecTable(from: result)
    }

    public func multikeyEncode(codecName: String, publicKey: [UInt8]) throws -> String {
        try withTextBytes(codecName, maxFfiInputLength: provider.ffiInputLimit) { encodedCodecName in
            try text(provider.process(operation: CodecOperation.multikeyEncode, first: encodedCodecName, second: publicKey))
        }
    }

    public func multikeyParse(_ multikey: String) throws -> ReallyMeParsedMultikey {
        var operationResult = try withOwnedBytes(
            multikeyParseOperationRequestBytes(multikey, maxFfiInputLength: provider.ffiInputLimit)
        ) { request in
            try processGeneratedOperation(request: request)
        }
        guard case .multikeyParse(var result)? = operationResult.result else {
            throw ReallyMeCodecError.providerFailure
        }
        operationResult.result = nil
        defer {
            ReallyMeCodecMemory.clearOwned(&result.publicKey)
        }
        return try sdkParsedMultikey(from: result)
    }

    public func requireSupportedMulticodec(_ name: String) throws {
        try withTextBytes(name, maxFfiInputLength: provider.ffiInputLimit) { encoded in
            _ = try provider.process(operation: CodecOperation.requireSupportedMulticodec, first: encoded)
        }
    }

    public func bindingTypeMatchesCodec(bindingType: String, codecName: String) throws -> Bool {
        try withTextBytes(
            bindingType,
            codecName,
            maxFfiInputLength: provider.ffiInputLimit
        ) { encodedBindingType, encodedCodecName in
            try provider.processBool(
                operation: CodecBoolOperation.bindingTypeMatchesCodec,
                first: encodedBindingType,
                second: encodedCodecName
            )
        }
    }

    public func validateKeyBinding(bindingType: String, algorithm: String?, multikey: String) throws {
        try withTextBytes(
            bindingType,
            algorithm ?? "",
            multikey,
            maxFfiInputLength: provider.ffiInputLimit
        ) {
            encodedBindingType,
            encodedAlgorithm,
            encodedMultikey in
            _ = try provider.process(
                operation: CodecOperation.validateKeyBinding,
                first: encodedBindingType,
                second: encodedAlgorithm,
                third: encodedMultikey
            )
        }
    }

    public func dagCborComputeCid(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.dagCborComputeCid, first: bytes))
    }

    public func dagCborVerifyCid(
        cid: String,
        bytes payload: [UInt8]
    ) throws -> ReallyMeDagCborCidVerification {
        let operationResult = try withOwnedBytes(
            dagCborVerifyCidOperationRequestBytes(
                cid: cid,
                payload: payload,
                maxFfiInputLength: provider.ffiInputLimit
            )
        ) { request in
            try processGeneratedOperation(request: request)
        }
        guard case .dagCborVerifyCid(let result)? = operationResult.result else {
            throw ReallyMeCodecError.providerFailure
        }
        return try sdkDagCborCidVerification(from: result)
    }

    public func dagCborSha256ContentHash(_ bytes: [UInt8]) throws -> [UInt8] {
        try provider.process(operation: CodecOperation.dagCborSha256ContentHash, first: bytes)
    }

    public func dagCborMultihash(_ bytes: [UInt8]) throws -> [UInt8] {
        try provider.process(operation: CodecOperation.dagCborMultihash, first: bytes)
    }

    public func isValidCidString(_ cid: String) throws -> Bool {
        try withTextBytes(cid, maxFfiInputLength: provider.ffiInputLimit) { encoded in
            try provider.processBool(operation: CodecBoolOperation.isValidCidString, first: encoded)
        }
    }

    public func tryParseCid(_ cid: String) throws -> String? {
        do {
            return try withTextBytes(cid, maxFfiInputLength: provider.ffiInputLimit) { encoded in
                try text(provider.process(operation: CodecOperation.tryParseCid, first: encoded))
            }
        } catch let error as ReallyMeCodecError where error == .invalidInput {
            return nil
        }
    }

    public func dagCborCodecCode() throws -> UInt32 {
        let value = try text(provider.process(operation: CodecOperation.dagCborCodecCode, first: []))
        guard let code = UInt32(value) else {
            throw ReallyMeCodecError.providerFailure
        }
        return code
    }

    public func canonicalizeJson(_ json: String) throws -> String {
        try withTextBytes(json, maxFfiInputLength: provider.ffiInputLimit) { encoded in
            try text(provider.process(operation: CodecOperation.canonicalizeJson, first: encoded))
        }
    }

    /// Executes a generated request through the fully discriminated response
    /// contract used by structured SDK convenience methods.
    ///
    /// The serialized response is an SDK-owned transient and is wiped on every
    /// path. Exact operation-result selection remains the caller's
    /// responsibility so a provider cannot substitute a different valid
    /// generated variant.
    func processGeneratedOperation(request: [UInt8]) throws -> ReallyMeProtoCodecOperationResult {
        var responseBytes = try provider.processOperation(request: request)
        defer {
            ReallyMeCodecMemory.clearOwned(&responseBytes)
        }
        let response: ReallyMeProtoCodecOperationResponse
        do {
            var options = SwiftProtobuf.BinaryDecodingOptions()
            // Recursive CBOR values expand through generated Value, Map, and
            // MapEntry wrappers. The transport parser must admit the documented
            // semantic maximum; provider-tree validation below still enforces
            // the tighter operation-specific depth and node budgets.
            options.messageDepthLimit = maxDeterministicCborProtoMessageDepth
            response = try ReallyMeProtoCodecOperationResponse(
                serializedBytes: responseBytes,
                options: options
            )
        } catch {
            throw ReallyMeCodecError.providerFailure
        }
        guard response.unknownFields.data.isEmpty else {
            throw ReallyMeCodecError.providerFailure
        }
        switch response.outcome {
        case .result(let result):
            guard result.unknownFields.data.isEmpty, result.result != nil else {
                throw ReallyMeCodecError.providerFailure
            }
            return result
        case .error(let error):
            throw ReallyMeCodecRustCAbiProvider.errorForCodecError(error)
        case nil:
            throw ReallyMeCodecError.providerFailure
        }
    }

    /// Executes one binary generated `CodecOperationRequest` and returns a
    /// binary, fully discriminated `CodecOperationResponse`.
    public func processOperation(_ request: [UInt8]) throws -> [UInt8] {
        try provider.processOperation(request: request)
    }

    /// Executes the generated ProtoJSON view of `CodecOperationRequest`.
    ///
    /// JSON is request-only; the returned bytes are the same binary result
    /// response returned by `processOperation(_:)`.
    public func processOperationJson(_ requestJson: [UInt8]) throws -> [UInt8] {
        try provider.processOperationJson(request: requestJson)
    }

    /// Decodes wipeable UTF-8 PEM armor bytes into a typed SDK owner.
    public func decodePem(
        _ pem: [UInt8],
        options: ReallyMePemDecodeOptions = ReallyMePemDecodeOptions()
    ) throws -> ReallyMePemDocument {
        var operationResult = try withOwnedBytes(
            pemDecodeOperationRequestBytes(
                pem: pem,
                options: options,
                maxFfiInputLength: provider.ffiInputLimit
            )
        ) { request in
            try processGeneratedOperation(request: request)
        }
        guard case .pemDecode(var result)? = operationResult.result else {
            throw ReallyMeCodecError.providerFailure
        }
        operationResult.result = nil
        defer {
            ReallyMeCodecMemory.clearOwned(&result.der)
        }
        return try sdkPemDocument(from: result)
    }

    /// Encodes DER into wipeable UTF-8 PEM armor bytes.
    public func encodePem(
        label: ReallyMePemLabel,
        der: [UInt8],
        options: ReallyMePemEncodeOptions = ReallyMePemEncodeOptions()
    ) throws -> [UInt8] {
        var operationResult = try withOwnedBytes(
            pemEncodeOperationRequestBytes(
                label: label,
                der: der,
                options: options,
                maxFfiInputLength: provider.ffiInputLimit
            )
        ) { request in
            try processGeneratedOperation(request: request)
        }
        guard case .pemEncode(var result)? = operationResult.result else {
            throw ReallyMeCodecError.providerFailure
        }
        operationResult.result = nil
        defer {
            ReallyMeCodecMemory.clearOwned(&result.pem)
        }
        try requireNoStructuredProviderUnknownFields(result.unknownFields)
        return Array(result.pem)
    }
}

private func bytes(_ text: String, maxFfiInputLength: Int) throws -> [UInt8] {
    try requireBoundaryAggregate([text.utf8.count], maxFfiInputLength: maxFfiInputLength)
    return Array(text.utf8)
}

/// Limits the lifetime of a mutable UTF-8 copy created from an immutable
/// Swift string. The caller still owns the original string, but the codec must
/// wipe every additional buffer it creates on both success and failure paths.
private func withTextBytes<T>(
    _ text: String,
    maxFfiInputLength: Int,
    _ body: ([UInt8]) throws -> T
) throws -> T {
    var encoded = try bytes(text, maxFfiInputLength: maxFfiInputLength)
    defer {
        ReallyMeCodecMemory.clearOwned(&encoded)
    }
    return try body(encoded)
}

private func withTextBytes<T>(
    _ first: String,
    _ second: String,
    maxFfiInputLength: Int,
    _ body: ([UInt8], [UInt8]) throws -> T
) throws -> T {
    try withTextBytes(first, maxFfiInputLength: maxFfiInputLength) { firstBytes in
        try withTextBytes(second, maxFfiInputLength: maxFfiInputLength) { secondBytes in
            try body(firstBytes, secondBytes)
        }
    }
}

private func withTextBytes<T>(
    _ first: String,
    _ second: String,
    _ third: String,
    maxFfiInputLength: Int,
    _ body: ([UInt8], [UInt8], [UInt8]) throws -> T
) throws -> T {
    try withTextBytes(first, second, maxFfiInputLength: maxFfiInputLength) { firstBytes, secondBytes in
        try withTextBytes(third, maxFfiInputLength: maxFfiInputLength) { thirdBytes in
            try body(firstBytes, secondBytes, thirdBytes)
        }
    }
}

/// Limits the lifetime of an SDK-created serialized request. The consuming
/// parameter transfers the fresh array owner into this scope, and the closure
/// receives only a borrow that ends before the non-elidable wipe runs.
func withOwnedBytes<T>(
    _ bytes: consuming [UInt8],
    _ body: ([UInt8]) throws -> T
) rethrows -> T {
    var ownedBytes = consume bytes
    defer {
        ReallyMeCodecMemory.clearOwned(&ownedBytes)
    }
    return try body(ownedBytes)
}

private func text(_ bytes: consuming [UInt8]) throws -> String {
    var ownedBytes = consume bytes
    defer {
        ReallyMeCodecMemory.clearOwned(&ownedBytes)
    }
    guard let value = String(bytes: ownedBytes, encoding: .utf8) else {
        throw ReallyMeCodecError.providerFailure
    }
    return value
}

private func operationRequest(
    _ operation: ReallyMeProtoCodecOperationRequest.OneOf_Operation,
    maxFfiInputLength: Int
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
    guard serialized.count <= maxFfiInputLength else {
        throw ReallyMeCodecError.invalidInput
    }
    return Array(serialized)
}

private func multicodecPrefixForNameOperationRequestBytes(
    _ name: String,
    maxFfiInputLength: Int
) throws -> [UInt8] {
    try requireBoundaryAggregate([name.utf8.count], maxFfiInputLength: maxFfiInputLength)
    var request = ReallyMeProtoCodecMulticodecPrefixForNameRequest()
    request.name = name
    return try operationRequest(.multicodecPrefixForName(request), maxFfiInputLength: maxFfiInputLength)
}

private func multicodecLookupPrefixOperationRequestBytes(
    _ bytes: [UInt8],
    maxFfiInputLength: Int
) throws -> [UInt8] {
    try requireBoundaryAggregate([bytes.count], maxFfiInputLength: maxFfiInputLength)
    var request = ReallyMeProtoCodecMulticodecLookupPrefixRequest()
    request.value = Data(bytes)
    defer {
        ReallyMeCodecMemory.clearOwned(&request.value)
    }
    return try operationRequest(.multicodecLookupPrefix(request), maxFfiInputLength: maxFfiInputLength)
}

private func multicodecTableOperationRequestBytes(maxFfiInputLength: Int) throws -> [UInt8] {
    try operationRequest(
        .multicodecTable(ReallyMeProtoCodecMulticodecTableRequest()),
        maxFfiInputLength: maxFfiInputLength
    )
}

private func multikeyParseOperationRequestBytes(
    _ multikey: String,
    maxFfiInputLength: Int
) throws -> [UInt8] {
    try requireBoundaryAggregate([multikey.utf8.count], maxFfiInputLength: maxFfiInputLength)
    var request = ReallyMeProtoCodecMultikeyParseRequest()
    request.multikey = multikey
    return try operationRequest(.multikeyParse(request), maxFfiInputLength: maxFfiInputLength)
}

private func dagCborVerifyCidOperationRequestBytes(
    cid: String,
    payload: [UInt8],
    maxFfiInputLength: Int
) throws -> [UInt8] {
    try requireBoundaryAggregate([cid.utf8.count, payload.count], maxFfiInputLength: maxFfiInputLength)
    var request = ReallyMeProtoCodecDagCborVerifyCidRequest()
    request.cid = cid
    request.payload = Data(payload)
    defer {
        ReallyMeCodecMemory.clearOwned(&request.payload)
    }
    return try operationRequest(.dagCborVerifyCid(request), maxFfiInputLength: maxFfiInputLength)
}

private func pemDecodeOperationRequestBytes(
    pem: [UInt8],
    options: ReallyMePemDecodeOptions,
    maxFfiInputLength: Int
) throws -> [UInt8] {
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

private func pemEncodeOperationRequestBytes(
    label: ReallyMePemLabel,
    der: [UInt8],
    options: ReallyMePemEncodeOptions,
    maxFfiInputLength: Int
) throws -> [UInt8] {
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
    switch options.lineEnding {
    case .lf:
        protoOptions.lineEnding = .lf
    case .crlf:
        protoOptions.lineEnding = .crlf
    case nil:
        protoOptions.lineEnding = .unspecified
    }
    request.options = protoOptions
    return try operationRequest(.pemEncode(request), maxFfiInputLength: maxFfiInputLength)
}

func requireBoundaryAggregate(_ lengths: [Int], maxFfiInputLength: Int) throws {
    guard isBoundaryAggregateValid(lengths, maxFfiInputLength: maxFfiInputLength) else {
        throw ReallyMeCodecError.invalidInput
    }
}

private func isBoundaryAggregateValid(_ lengths: [Int], maxFfiInputLength: Int) -> Bool {
    var aggregate = 0
    for length in lengths {
        let (next, overflow) = aggregate.addingReportingOverflow(length)
        guard !overflow, next <= maxFfiInputLength else {
            return false
        }
        aggregate = next
    }
    return true
}
