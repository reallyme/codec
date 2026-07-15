// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

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
    public let bytes: [UInt8]

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
        provider = ReallyMeCodecRustCAbiProvider()
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
        try provider.process(operation: CodecOperation.base64Decode, first: bytes(text))
    }

    public func base64urlEncode(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.base64urlEncode, first: bytes))
    }

    public func base64urlDecode(_ text: String) throws -> [UInt8] {
        try provider.process(operation: CodecOperation.base64urlDecode, first: bytes(text))
    }

    public func bytesToLowerHex(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.lowerHexEncode, first: bytes))
    }

    public func lowerHexToBytes(_ text: String) throws -> [UInt8] {
        try provider.process(operation: CodecOperation.lowerHexDecode, first: bytes(text))
    }

    public func base58btcEncode(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.base58btcEncode, first: bytes))
    }

    public func base58btcDecode(_ text: String) throws -> [UInt8] {
        try provider.process(operation: CodecOperation.base58btcDecode, first: bytes(text))
    }

    public func multibaseBase58btcEncode(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.multibaseBase58btcEncode, first: bytes))
    }

    public func multibaseBase64urlEncode(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.multibaseBase64urlEncode, first: bytes))
    }

    public func multibaseDecode(_ text: String) throws -> [UInt8] {
        try provider.process(operation: CodecOperation.multibaseDecode, first: bytes(text))
    }

    public func multicodecPrefixForName(_ name: String) throws -> String {
        try text(provider.process(operation: CodecOperation.multicodecPrefixForName, first: bytes(name)))
    }

    public func multicodecPrefixForNameProto(_ name: String) throws -> [UInt8] {
        try provider.processProto(operation: CodecOperation.multicodecPrefixForName, first: bytes(name))
    }

    public func multicodecPrefixForNameProtoResult(_ name: String) throws -> ReallyMeCodecProtoResult {
        try provider.processProtoResult(operation: CodecOperation.multicodecPrefixForName, first: bytes(name))
    }

    public func multicodecLookupPrefix(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.multicodecLookupPrefix, first: bytes))
    }

    public func multicodecLookupPrefixProto(_ bytes: [UInt8]) throws -> [UInt8] {
        try provider.processProto(operation: CodecOperation.multicodecLookupPrefix, first: bytes)
    }

    public func multicodecLookupPrefixProtoResult(_ bytes: [UInt8]) throws -> ReallyMeCodecProtoResult {
        try provider.processProtoResult(operation: CodecOperation.multicodecLookupPrefix, first: bytes)
    }

    public func multicodecStripPrefix(_ bytes: [UInt8]) throws -> [UInt8] {
        try provider.process(operation: CodecOperation.multicodecStripPrefix, first: bytes)
    }

    public func multicodecTable() throws -> String {
        try text(provider.process(operation: CodecOperation.multicodecTable, first: []))
    }

    public func multicodecTableProto() throws -> [UInt8] {
        try provider.processProto(operation: CodecOperation.multicodecTable, first: [])
    }

    public func multicodecTableProtoResult() throws -> ReallyMeCodecProtoResult {
        try provider.processProtoResult(operation: CodecOperation.multicodecTable, first: [])
    }

    public func multikeyEncode(codecName: String, publicKey: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.multikeyEncode, first: bytes(codecName), second: publicKey))
    }

    public func multikeyParse(_ multikey: String) throws -> String {
        try text(provider.process(operation: CodecOperation.multikeyParse, first: bytes(multikey)))
    }

    public func multikeyParseProto(_ multikey: String) throws -> [UInt8] {
        try provider.processProto(operation: CodecOperation.multikeyParse, first: bytes(multikey))
    }

    public func multikeyParseProtoResult(_ multikey: String) throws -> ReallyMeCodecProtoResult {
        try provider.processProtoResult(operation: CodecOperation.multikeyParse, first: bytes(multikey))
    }

    public func requireSupportedMulticodec(_ name: String) throws {
        _ = try provider.process(operation: CodecOperation.requireSupportedMulticodec, first: bytes(name))
    }

    public func bindingTypeMatchesCodec(bindingType: String, codecName: String) throws -> Bool {
        try provider.processBool(
            operation: CodecBoolOperation.bindingTypeMatchesCodec,
            first: bytes(bindingType),
            second: bytes(codecName)
        )
    }

    public func validateKeyBinding(bindingType: String, algorithm: String?, multikey: String) throws {
        _ = try provider.process(
            operation: CodecOperation.validateKeyBinding,
            first: bytes(bindingType),
            second: bytes(algorithm ?? ""),
            third: bytes(multikey)
        )
    }

    public func dagCborEncode(taggedJson: String) throws -> [UInt8] {
        try provider.process(operation: CodecOperation.dagCborEncode, first: bytes(taggedJson))
    }

    public func dagCborDecode(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.dagCborDecode, first: bytes))
    }

    public func dagCborComputeCid(_ bytes: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.dagCborComputeCid, first: bytes))
    }

    public func dagCborVerifyCid(cid: String, bytes payload: [UInt8]) throws -> String {
        try text(provider.process(operation: CodecOperation.dagCborVerifyCid, first: bytes(cid), second: payload))
    }

    public func dagCborVerifyCidProto(cid: String, bytes payload: [UInt8]) throws -> [UInt8] {
        try provider.processProto(operation: CodecOperation.dagCborVerifyCid, first: bytes(cid), second: payload)
    }

    public func dagCborVerifyCidProtoResult(cid: String, bytes payload: [UInt8]) throws -> ReallyMeCodecProtoResult {
        try provider.processProtoResult(operation: CodecOperation.dagCborVerifyCid, first: bytes(cid), second: payload)
    }

    public func dagCborSha256ContentHash(_ bytes: [UInt8]) throws -> [UInt8] {
        try provider.process(operation: CodecOperation.dagCborSha256ContentHash, first: bytes)
    }

    public func dagCborMultihash(_ bytes: [UInt8]) throws -> [UInt8] {
        try provider.process(operation: CodecOperation.dagCborMultihash, first: bytes)
    }

    public func isValidCidString(_ cid: String) throws -> Bool {
        try provider.processBool(operation: CodecBoolOperation.isValidCidString, first: bytes(cid))
    }

    public func tryParseCid(_ cid: String) throws -> String? {
        do {
            return try text(provider.process(operation: CodecOperation.tryParseCid, first: bytes(cid)))
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
        try text(provider.process(operation: CodecOperation.canonicalizeJson, first: bytes(json)))
    }

    public func decodePem(_ pem: String, optionsJson: String = "") throws -> String {
        try text(provider.process(operation: CodecOperation.pemDecode, first: bytes(pem), second: bytes(optionsJson)))
    }

    public func decodePemProto(_ pem: String, optionsJson: String = "") throws -> [UInt8] {
        try provider.processProto(operation: CodecOperation.pemDecode, first: bytes(pem), second: bytes(optionsJson))
    }

    public func decodePemProtoResult(_ pem: String, optionsJson: String = "") throws -> ReallyMeCodecProtoResult {
        try provider.processProtoResult(operation: CodecOperation.pemDecode, first: bytes(pem), second: bytes(optionsJson))
    }

    public func encodePem(label: String, der: [UInt8], optionsJson: String = "") throws -> String {
        try text(provider.process(operation: CodecOperation.pemEncode, first: bytes(label), second: der, third: bytes(optionsJson)))
    }
}

private func bytes(_ text: String) -> [UInt8] {
    Array(text.utf8)
}

private func text(_ bytes: [UInt8]) throws -> String {
    guard let value = String(bytes: bytes, encoding: .utf8) else {
        throw ReallyMeCodecError.providerFailure
    }
    return value
}
