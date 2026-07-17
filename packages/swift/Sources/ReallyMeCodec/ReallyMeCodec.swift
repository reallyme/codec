// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import Foundation
import ReallyMeCodecProto

private let maxCodecFfiInputBytes = 1_048_576

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
    static let multicodecPrefixForName: UInt32 = 12
    static let multicodecLookupPrefix: UInt32 = 13
    static let multicodecStripPrefix: UInt32 = 14
    static let multicodecTable: UInt32 = 15
    static let multikeyEncode: UInt32 = 16
    static let multikeyParse: UInt32 = 17
    static let requireSupportedMulticodec: UInt32 = 18
    static let dagCborEncode: UInt32 = 19
    static let dagCborDecode: UInt32 = 20
    static let dagCborComputeCid: UInt32 = 21
    static let dagCborVerifyCid: UInt32 = 22
    static let dagCborSha256ContentHash: UInt32 = 23
    static let dagCborMultihash: UInt32 = 24
    static let tryParseCid: UInt32 = 25
    static let dagCborCodecCode: UInt32 = 26
    static let canonicalizeJson: UInt32 = 27
    static let pemDecode: UInt32 = 28
    static let pemEncode: UInt32 = 29
    static let validateKeyBinding: UInt32 = 30
}

private enum CodecBoolOperation {
    static let bindingTypeMatchesCodec: UInt32 = 1
    static let isValidCidString: UInt32 = 2
}

public enum ReallyMeCodecProtoStatus: Sendable {
    case result
    case codecError
}

public struct ReallyMeCodecProtoResult: Sendable {
    public let status: ReallyMeCodecProtoStatus
    public var bytes: [UInt8]

    public var isCodecError: Bool {
        status == .codecError
    }

    public init(status: ReallyMeCodecProtoStatus, bytes: [UInt8]) {
        self.status = status
        self.bytes = bytes
    }
}

/// Swift facade for ReallyMe codec operations backed by the Rust codec crates.
public struct ReallyMeCodec: Sendable {
    private let provider: ReallyMeCodecRustCAbiProvider

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
        try withTextBytes(text) { encoded in
            try provider.process(operation: CodecOperation.base64Decode, first: encoded)
        }
    }

    public func base64urlEncode(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.base64urlEncode, first: bytes))
    }

    public func base64urlDecode(_ text: String) throws -> [UInt8] {
        try withTextBytes(text) { encoded in
            try provider.process(operation: CodecOperation.base64urlDecode, first: encoded)
        }
    }

    public func bytesToLowerHex(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.lowerHexEncode, first: bytes))
    }

    public func lowerHexToBytes(_ text: String) throws -> [UInt8] {
        try withTextBytes(text) { encoded in
            try provider.process(operation: CodecOperation.lowerHexDecode, first: encoded)
        }
    }

    public func base58btcEncode(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.base58btcEncode, first: bytes))
    }

    public func base58btcDecode(_ text: String) throws -> [UInt8] {
        try withTextBytes(text) { encoded in
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
        try withTextBytes(text) { encoded in
            try provider.process(operation: CodecOperation.multibaseDecode, first: encoded)
        }
    }

    public func multicodecPrefixForName(_ name: String) throws -> String {
        try withTextBytes(name) { encoded in
            try text(provider.process(operation: CodecOperation.multicodecPrefixForName, first: encoded))
        }
    }

    public func multicodecPrefixForNameProto(_ name: String) throws -> [UInt8] {
        try withOwnedBytes(multicodecPrefixForNameProtoRequest(name)) { request in
            try provider.processProto(request: request)
        }
    }

    public func multicodecPrefixForNameProtoResult(_ name: String) throws -> ReallyMeCodecProtoResult {
        guard isBoundaryAggregateValid([name.utf8.count]) else {
            return try boundaryResourceLimitResult()
        }
        return try withOwnedBytes(multicodecPrefixForNameProtoRequest(name)) { request in
            try provider.processProtoResult(request: request)
        }
    }

    public func multicodecLookupPrefix(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.multicodecLookupPrefix, first: bytes))
    }

    public func multicodecLookupPrefixProto(_ bytes: [UInt8]) throws -> [UInt8] {
        try withOwnedBytes(multicodecLookupPrefixProtoRequest(bytes)) { request in
            try provider.processProto(request: request)
        }
    }

    public func multicodecLookupPrefixProtoResult(_ bytes: [UInt8]) throws -> ReallyMeCodecProtoResult {
        guard isBoundaryAggregateValid([bytes.count]) else {
            return try boundaryResourceLimitResult()
        }
        return try withOwnedBytes(multicodecLookupPrefixProtoRequest(bytes)) { request in
            try provider.processProtoResult(request: request)
        }
    }

    public func multicodecStripPrefix(_ bytes: [UInt8]) throws -> [UInt8] {
        try provider.process(operation: CodecOperation.multicodecStripPrefix, first: bytes)
    }

    public func multicodecTable() throws -> String {
        try text(provider.process(operation: CodecOperation.multicodecTable, first: []))
    }

    public func multicodecTableProto() throws -> [UInt8] {
        try withOwnedBytes(multicodecTableProtoRequest()) { request in
            try provider.processProto(request: request)
        }
    }

    public func multicodecTableProtoResult() throws -> ReallyMeCodecProtoResult {
        try withOwnedBytes(multicodecTableProtoRequest()) { request in
            try provider.processProtoResult(request: request)
        }
    }

    public func multikeyEncode(codecName: String, publicKey: [UInt8]) throws -> String {
        try withTextBytes(codecName) { encodedCodecName in
            try text(provider.process(operation: CodecOperation.multikeyEncode, first: encodedCodecName, second: publicKey))
        }
    }

    public func multikeyParse(_ multikey: String) throws -> String {
        try withTextBytes(multikey) { encoded in
            try text(provider.process(operation: CodecOperation.multikeyParse, first: encoded))
        }
    }

    public func multikeyParseProto(_ multikey: String) throws -> [UInt8] {
        try withOwnedBytes(multikeyParseProtoRequest(multikey)) { request in
            try provider.processProto(request: request)
        }
    }

    public func multikeyParseProtoResult(_ multikey: String) throws -> ReallyMeCodecProtoResult {
        guard isBoundaryAggregateValid([multikey.utf8.count]) else {
            return try boundaryResourceLimitResult()
        }
        return try withOwnedBytes(multikeyParseProtoRequest(multikey)) { request in
            try provider.processProtoResult(request: request)
        }
    }

    public func requireSupportedMulticodec(_ name: String) throws {
        try withTextBytes(name) { encoded in
            _ = try provider.process(operation: CodecOperation.requireSupportedMulticodec, first: encoded)
        }
    }

    public func bindingTypeMatchesCodec(bindingType: String, codecName: String) throws -> Bool {
        try withTextBytes(bindingType, codecName) { encodedBindingType, encodedCodecName in
            try provider.processBool(
                operation: CodecBoolOperation.bindingTypeMatchesCodec,
                first: encodedBindingType,
                second: encodedCodecName
            )
        }
    }

    public func validateKeyBinding(bindingType: String, algorithm: String?, multikey: String) throws {
        try withTextBytes(bindingType, algorithm ?? "", multikey) {
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

    public func dagCborEncode(taggedJson: String) throws -> [UInt8] {
        try withTextBytes(taggedJson) { encoded in
            try provider.process(operation: CodecOperation.dagCborEncode, first: encoded)
        }
    }

    public func dagCborDecode(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.dagCborDecode, first: bytes))
    }

    public func dagCborComputeCid(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.dagCborComputeCid, first: bytes))
    }

    public func dagCborVerifyCid(cid: String, bytes payload: [UInt8]) throws -> String {
        try withTextBytes(cid) { encodedCid in
            try text(provider.process(operation: CodecOperation.dagCborVerifyCid, first: encodedCid, second: payload))
        }
    }

    public func dagCborVerifyCidProto(cid: String, bytes payload: [UInt8]) throws -> [UInt8] {
        try withOwnedBytes(dagCborVerifyCidProtoRequest(cid: cid, payload: payload)) { request in
            try provider.processProto(request: request)
        }
    }

    public func dagCborVerifyCidProtoResult(cid: String, bytes payload: [UInt8]) throws -> ReallyMeCodecProtoResult {
        guard isBoundaryAggregateValid([cid.utf8.count, payload.count]) else {
            return try boundaryResourceLimitResult()
        }
        return try withOwnedBytes(
            dagCborVerifyCidProtoRequest(cid: cid, payload: payload)
        ) { request in
            try provider.processProtoResult(request: request)
        }
    }

    public func dagCborSha256ContentHash(_ bytes: [UInt8]) throws -> [UInt8] {
        try provider.process(operation: CodecOperation.dagCborSha256ContentHash, first: bytes)
    }

    public func dagCborMultihash(_ bytes: [UInt8]) throws -> [UInt8] {
        try provider.process(operation: CodecOperation.dagCborMultihash, first: bytes)
    }

    public func isValidCidString(_ cid: String) throws -> Bool {
        try withTextBytes(cid) { encoded in
            try provider.processBool(operation: CodecBoolOperation.isValidCidString, first: encoded)
        }
    }

    public func tryParseCid(_ cid: String) throws -> String? {
        do {
            return try withTextBytes(cid) { encoded in
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
        try withTextBytes(json) { encoded in
            try text(provider.process(operation: CodecOperation.canonicalizeJson, first: encoded))
        }
    }

    /// Executes one binary generated `CodecOperationRequest`.
    ///
    /// The returned bytes are always a binary
    /// `CodecProtoResultEnvelope`. Malformed input and operation failures are
    /// represented inside the envelope rather than collapsed into an FFI
    /// error.
    public func processProto(_ request: [UInt8]) throws -> [UInt8] {
        try provider.processProtoEnvelope(request: request)
    }

    /// Executes the generated ProtoJSON view of `CodecOperationRequest`.
    ///
    /// JSON is request-only; the returned bytes are the same binary result
    /// envelope returned by `processProto(_:)`.
    public func processProtoJson(_ requestJson: [UInt8]) throws -> [UInt8] {
        try provider.processProtoJsonEnvelope(requestJson: requestJson)
    }

    /// Decodes wipeable UTF-8 PEM armor bytes into wipeable JSON result bytes.
    public func decodePem(_ pem: [UInt8], optionsJson: String = "") throws -> [UInt8] {
        try withTextBytes(optionsJson) { encodedOptions in
            try provider.process(operation: CodecOperation.pemDecode, first: pem, second: encodedOptions)
        }
    }

    /// Encodes DER into wipeable UTF-8 PEM armor bytes.
    public func encodePem(label: String, der: [UInt8], optionsJson: String = "") throws -> [UInt8] {
        try withTextBytes(label, optionsJson) { encodedLabel, encodedOptions in
            try provider.process(operation: CodecOperation.pemEncode, first: encodedLabel, second: der, third: encodedOptions)
        }
    }
}

private func bytes(_ text: String) throws -> [UInt8] {
    try requireBoundaryAggregate([text.utf8.count])
    return Array(text.utf8)
}

/// Limits the lifetime of a mutable UTF-8 copy created from an immutable
/// Swift string. The caller still owns the original string, but the codec must
/// wipe every additional buffer it creates on both success and failure paths.
private func withTextBytes<T>(
    _ text: String,
    _ body: ([UInt8]) throws -> T
) throws -> T {
    var encoded = try bytes(text)
    defer {
        ReallyMeCodecMemory.clearOwned(&encoded)
    }
    return try body(encoded)
}

private func withTextBytes<T>(
    _ first: String,
    _ second: String,
    _ body: ([UInt8], [UInt8]) throws -> T
) throws -> T {
    try withTextBytes(first) { firstBytes in
        try withTextBytes(second) { secondBytes in
            try body(firstBytes, secondBytes)
        }
    }
}

private func withTextBytes<T>(
    _ first: String,
    _ second: String,
    _ third: String,
    _ body: ([UInt8], [UInt8], [UInt8]) throws -> T
) throws -> T {
    try withTextBytes(first, second) { firstBytes, secondBytes in
        try withTextBytes(third) { thirdBytes in
            try body(firstBytes, secondBytes, thirdBytes)
        }
    }
}

/// Limits the lifetime of an SDK-created serialized request. The consuming
/// parameter transfers the fresh array owner into this scope, and the closure
/// receives only a borrow that ends before the non-elidable wipe runs.
private func withOwnedBytes<T>(
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
    guard serialized.count <= maxCodecFfiInputBytes else {
        throw ReallyMeCodecError.invalidInput
    }
    return Array(serialized)
}

private func multicodecPrefixForNameProtoRequest(_ name: String) throws -> [UInt8] {
    try requireBoundaryAggregate([name.utf8.count])
    var request = ReallyMeProtoCodecMulticodecPrefixForNameRequest()
    request.name = name
    return try operationRequest(.multicodecPrefixForName(request))
}

private func multicodecLookupPrefixProtoRequest(_ bytes: [UInt8]) throws -> [UInt8] {
    try requireBoundaryAggregate([bytes.count])
    var request = ReallyMeProtoCodecMulticodecLookupPrefixRequest()
    request.value = Data(bytes)
    defer {
        ReallyMeCodecMemory.clearOwned(&request.value)
    }
    return try operationRequest(.multicodecLookupPrefix(request))
}

private func multicodecTableProtoRequest() throws -> [UInt8] {
    try operationRequest(.multicodecTable(ReallyMeProtoCodecMulticodecTableRequest()))
}

private func multikeyParseProtoRequest(_ multikey: String) throws -> [UInt8] {
    try requireBoundaryAggregate([multikey.utf8.count])
    var request = ReallyMeProtoCodecMultikeyParseRequest()
    request.multikey = multikey
    return try operationRequest(.multikeyParse(request))
}

private func dagCborVerifyCidProtoRequest(cid: String, payload: [UInt8]) throws -> [UInt8] {
    try requireBoundaryAggregate([cid.utf8.count, payload.count])
    var request = ReallyMeProtoCodecDagCborVerifyCidRequest()
    request.cid = cid
    request.payload = Data(payload)
    defer {
        ReallyMeCodecMemory.clearOwned(&request.payload)
    }
    return try operationRequest(.dagCborVerifyCid(request))
}

private func requireBoundaryAggregate(_ lengths: [Int]) throws {
    guard isBoundaryAggregateValid(lengths) else {
        throw ReallyMeCodecError.invalidInput
    }
}

private func isBoundaryAggregateValid(_ lengths: [Int]) -> Bool {
    var aggregate = 0
    for length in lengths {
        let (next, overflow) = aggregate.addingReportingOverflow(length)
        guard !overflow, next <= maxCodecFfiInputBytes else {
            return false
        }
        aggregate = next
    }
    return true
}

private func boundaryResourceLimitResult() throws -> ReallyMeCodecProtoResult {
    var boundary = ReallyMeProtoCodecBoundaryError()
    boundary.reason = .boundaryResourceLimitExceeded
    var error = ReallyMeProtoCodecError()
    error.boundary = boundary
    var serialized: Data
    do {
        serialized = try error.serializedData()
    } catch {
        throw ReallyMeCodecError.providerFailure
    }
    defer {
        ReallyMeCodecMemory.clearOwned(&serialized)
    }
    return ReallyMeCodecProtoResult(status: .codecError, bytes: Array(serialized))
}
