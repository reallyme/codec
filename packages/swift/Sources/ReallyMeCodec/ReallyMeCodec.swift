// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
import SwiftProtobuf

// The operation envelope has its own transport bound. Scalar C ABI calls use
// the smaller ffiInputLimit, but a valid PEM request can exceed that limit.
private let maxOperationRequestBytes = 10_485_760

private enum CodecOperation {
    static let base64Encode: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.base64Encode.rawValue)
    static let base64Decode: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.base64Decode.rawValue)
    static let base64urlEncode: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.base64URLEncode.rawValue)
    static let base64urlDecode: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.base64URLDecode.rawValue)
    static let lowerHexEncode: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.lowerHexEncode.rawValue)
    static let lowerHexDecode: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.lowerHexDecode.rawValue)
    static let base58btcEncode: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.base58BtcEncode.rawValue)
    static let base58btcDecode: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.base58BtcDecode.rawValue)
    static let multibaseBase58btcEncode: UInt32 = UInt32(
        ReallyMeProtoCodecLegacyScalarOperation.multibaseBase58BtcEncode.rawValue)
    static let multibaseBase64urlEncode: UInt32 = UInt32(
        ReallyMeProtoCodecLegacyScalarOperation.multibaseBase64URLEncode.rawValue)
    static let multibaseDecode: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.multibaseDecode.rawValue)
    static let multicodecStripPrefix: UInt32 = UInt32(
        ReallyMeProtoCodecLegacyScalarOperation.multicodecStripPrefix.rawValue)
    static let multikeyEncode: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.multikeyEncode.rawValue)
    static let requireSupportedMulticodec: UInt32 = UInt32(
        ReallyMeProtoCodecLegacyScalarOperation.requireSupportedMulticodec.rawValue)
    static let dagCborComputeCid: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.dagCborComputeCid.rawValue)
    static let dagCborSha256ContentHash: UInt32 = UInt32(
        ReallyMeProtoCodecLegacyScalarOperation.dagCborSha256ContentHash.rawValue)
    static let dagCborMultihash: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.dagCborMultihash.rawValue)
    static let tryParseCid: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.tryParseCid.rawValue)
    static let dagCborCodecCode: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.dagCborCodecCode.rawValue)
    static let canonicalizeJson: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.canonicalizeJson.rawValue)
    static let validateKeyBinding: UInt32 = UInt32(ReallyMeProtoCodecLegacyScalarOperation.validateKeyBinding.rawValue)
    static let validateKeyBindingNoAlgorithm: UInt32 = UInt32(
        ReallyMeProtoCodecLegacyScalarOperation.validateKeyBindingNoAlgorithm.rawValue)
}

private enum CodecBoolOperation {
    static let bindingTypeMatchesCodec: UInt32 = UInt32(
        ReallyMeProtoCodecLegacyBoolOperation.bindingTypeMatchesCodec.rawValue)
    static let isValidCidString: UInt32 = UInt32(ReallyMeProtoCodecLegacyBoolOperation.isValidCidString.rawValue)
}

/// Swift facade for ReallyMe codec operations backed by the Rust codec crates.
public struct ReallyMeCodec: Sendable {
    let provider: ReallyMeCodecRustCAbiProvider

    /// Creates a codec backed by the Rust FFI library linked through the
    /// SwiftPM binary target shipped with the public package.
    public init() throws(ReallyMeCodecError) {
        #if REALLYME_CODEC_LINKED_FFI
            provider = try ReallyMeCodecRustCAbiProvider()
        #else
            throw ReallyMeCodecError.providerUnavailable
        #endif
    }

    public init(rustCAbiLibrary: ReallyMeCodecRustCAbiLibrary) throws(ReallyMeCodecError) {
        provider = try ReallyMeCodecRustCAbiProvider(library: rustCAbiLibrary)
    }

    public func base64Encode(_ bytes: [UInt8]) throws(ReallyMeCodecError) -> String {
        try text(provider.process(operation: CodecOperation.base64Encode, first: bytes))
    }

    public func base64Decode(_ text: String) throws(ReallyMeCodecError) -> [UInt8] {
        try withTextBytes(text, maxFfiInputLength: provider.ffiInputLimit) { (encoded) throws(ReallyMeCodecError) in
            try provider.process(operation: CodecOperation.base64Decode, first: encoded)
        }
    }

    public func base64urlEncode(_ bytes: [UInt8]) throws(ReallyMeCodecError) -> String {
        try text(provider.process(operation: CodecOperation.base64urlEncode, first: bytes))
    }

    public func base64urlDecode(_ text: String) throws(ReallyMeCodecError) -> [UInt8] {
        try withTextBytes(text, maxFfiInputLength: provider.ffiInputLimit) { (encoded) throws(ReallyMeCodecError) in
            try provider.process(operation: CodecOperation.base64urlDecode, first: encoded)
        }
    }

    public func bytesToLowerHex(_ bytes: [UInt8]) throws(ReallyMeCodecError) -> String {
        try text(provider.process(operation: CodecOperation.lowerHexEncode, first: bytes))
    }

    public func lowerHexToBytes(_ text: String) throws(ReallyMeCodecError) -> [UInt8] {
        try withTextBytes(text, maxFfiInputLength: provider.ffiInputLimit) { (encoded) throws(ReallyMeCodecError) in
            try provider.process(operation: CodecOperation.lowerHexDecode, first: encoded)
        }
    }

    public func base58btcEncode(_ bytes: [UInt8]) throws(ReallyMeCodecError) -> String {
        try text(provider.process(operation: CodecOperation.base58btcEncode, first: bytes))
    }

    public func base58btcDecode(_ text: String) throws(ReallyMeCodecError) -> [UInt8] {
        try withTextBytes(text, maxFfiInputLength: provider.ffiInputLimit) { (encoded) throws(ReallyMeCodecError) in
            try provider.process(operation: CodecOperation.base58btcDecode, first: encoded)
        }
    }

    public func multibaseBase58btcEncode(_ bytes: [UInt8]) throws(ReallyMeCodecError) -> String {
        try text(provider.process(operation: CodecOperation.multibaseBase58btcEncode, first: bytes))
    }

    public func multibaseBase64urlEncode(_ bytes: [UInt8]) throws(ReallyMeCodecError) -> String {
        try text(provider.process(operation: CodecOperation.multibaseBase64urlEncode, first: bytes))
    }

    public func multibaseDecode(_ text: String) throws(ReallyMeCodecError) -> [UInt8] {
        try withTextBytes(text, maxFfiInputLength: provider.ffiInputLimit) { (encoded) throws(ReallyMeCodecError) in
            try provider.process(operation: CodecOperation.multibaseDecode, first: encoded)
        }
    }

    public func multicodecPrefixForName(_ name: String) throws(ReallyMeCodecError) -> ReallyMeMulticodecMetadata {
        let operationResult = try withOwnedBytes(
            multicodecPrefixForNameOperationRequestBytes(name, maxFfiInputLength: maxOperationRequestBytes)
        ) { (request) throws(ReallyMeCodecError) in
            try processGeneratedOperation(request: request)
        }
        guard case .multicodecPrefixForName(let result)? = operationResult.result else {
            throw ReallyMeCodecError.providerFailure
        }
        return try sdkMulticodecMetadata(from: result)
    }

    public func multicodecLookupPrefix(_ bytes: [UInt8]) throws(ReallyMeCodecError) -> ReallyMeMulticodecLookupResult {
        let operationResult = try withOwnedBytes(
            multicodecLookupPrefixOperationRequestBytes(bytes, maxFfiInputLength: maxOperationRequestBytes)
        ) { (request) throws(ReallyMeCodecError) in
            try processGeneratedOperation(request: request)
        }
        guard case .multicodecLookupPrefix(let result)? = operationResult.result else {
            throw ReallyMeCodecError.providerFailure
        }
        return try sdkMulticodecLookupResult(from: result)
    }

    public func multicodecStripPrefix(_ bytes: [UInt8]) throws(ReallyMeCodecError) -> [UInt8] {
        try provider.process(operation: CodecOperation.multicodecStripPrefix, first: bytes)
    }

    public func multicodecTable() throws(ReallyMeCodecError) -> ReallyMeMulticodecTable {
        let operationResult = try withOwnedBytes(
            multicodecTableOperationRequestBytes(maxFfiInputLength: maxOperationRequestBytes)
        ) { (request) throws(ReallyMeCodecError) in
            try processGeneratedOperation(request: request)
        }
        guard case .multicodecTable(let result)? = operationResult.result else {
            throw ReallyMeCodecError.providerFailure
        }
        return try sdkMulticodecTable(from: result)
    }

    public func multikeyEncode(codecName: String, publicKey: [UInt8]) throws(ReallyMeCodecError) -> String {
        try withTextBytes(codecName, maxFfiInputLength: provider.ffiInputLimit) {
            (encodedCodecName) throws(ReallyMeCodecError) in
            try text(
                provider.process(operation: CodecOperation.multikeyEncode, first: encodedCodecName, second: publicKey))
        }
    }

    public func multikeyParse(_ multikey: String) throws(ReallyMeCodecError) -> ReallyMeParsedMultikey {
        var operationResult = try withOwnedBytes(
            multikeyParseOperationRequestBytes(multikey, maxFfiInputLength: maxOperationRequestBytes)
        ) { (request) throws(ReallyMeCodecError) in
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

    public func requireSupportedMulticodec(_ name: String) throws(ReallyMeCodecError) {
        try withTextBytes(name, maxFfiInputLength: provider.ffiInputLimit) { (encoded) throws(ReallyMeCodecError) in
            _ = try provider.process(operation: CodecOperation.requireSupportedMulticodec, first: encoded)
        }
    }

    public func bindingTypeMatchesCodec(bindingType: String, codecName: String) throws(ReallyMeCodecError) -> Bool {
        try withTextBytes(
            bindingType,
            codecName,
            maxFfiInputLength: provider.ffiInputLimit
        ) { (encodedBindingType, encodedCodecName) throws(ReallyMeCodecError) in
            try provider.processBool(
                operation: CodecBoolOperation.bindingTypeMatchesCodec,
                first: encodedBindingType,
                second: encodedCodecName
            )
        }
    }

    public func validateKeyBinding(bindingType: String, algorithm: String?, multikey: String) throws(ReallyMeCodecError)
    {
        if algorithm == "" {
            throw ReallyMeCodecError.invalidInput
        }
        try withTextBytes(
            bindingType,
            algorithm ?? "",
            multikey,
            maxFfiInputLength: provider.ffiInputLimit
        ) { (encodedBindingType, encodedAlgorithm, encodedMultikey) throws(ReallyMeCodecError) in
            _ = try provider.process(
                operation: algorithm == nil
                    ? CodecOperation.validateKeyBindingNoAlgorithm
                    : CodecOperation.validateKeyBinding,
                first: encodedBindingType,
                second: encodedAlgorithm,
                third: encodedMultikey
            )
        }
    }

    public func dagCborComputeCid(_ bytes: [UInt8]) throws(ReallyMeCodecError) -> String {
        try text(provider.process(operation: CodecOperation.dagCborComputeCid, first: bytes))
    }

    public func dagCborVerifyCid(
        cid: String,
        bytes payload: [UInt8]
    ) throws(ReallyMeCodecError) -> ReallyMeDagCborCidVerification {
        let operationResult = try withOwnedBytes(
            dagCborVerifyCidOperationRequestBytes(
                cid: cid,
                payload: payload,
                maxFfiInputLength: maxOperationRequestBytes
            )
        ) { (request) throws(ReallyMeCodecError) in
            try processGeneratedOperation(request: request)
        }
        guard case .dagCborVerifyCid(let result)? = operationResult.result else {
            throw ReallyMeCodecError.providerFailure
        }
        return try sdkDagCborCidVerification(from: result)
    }

    public func dagCborSha256ContentHash(_ bytes: [UInt8]) throws(ReallyMeCodecError) -> [UInt8] {
        try provider.process(operation: CodecOperation.dagCborSha256ContentHash, first: bytes)
    }

    public func dagCborMultihash(_ bytes: [UInt8]) throws(ReallyMeCodecError) -> [UInt8] {
        try provider.process(operation: CodecOperation.dagCborMultihash, first: bytes)
    }

    public func isValidCidString(_ cid: String) throws(ReallyMeCodecError) -> Bool {
        try withTextBytes(cid, maxFfiInputLength: provider.ffiInputLimit) { (encoded) throws(ReallyMeCodecError) in
            try provider.processBool(operation: CodecBoolOperation.isValidCidString, first: encoded)
        }
    }

    public func tryParseCid(_ cid: String) throws(ReallyMeCodecError) -> String? {
        do {
            return try withTextBytes(cid, maxFfiInputLength: provider.ffiInputLimit) {
                (encoded) throws(ReallyMeCodecError) in
                try text(provider.process(operation: CodecOperation.tryParseCid, first: encoded))
            }
        } catch .invalidInput {
            return nil
        }
    }

    public func dagCborCodecCode() throws(ReallyMeCodecError) -> UInt32 {
        let value = try text(provider.process(operation: CodecOperation.dagCborCodecCode, first: []))
        guard let code = UInt32(value) else {
            throw ReallyMeCodecError.providerFailure
        }
        return code
    }

    public func canonicalizeJson(_ json: String) throws(ReallyMeCodecError) -> String {
        try withTextBytes(json, maxFfiInputLength: provider.ffiInputLimit) { (encoded) throws(ReallyMeCodecError) in
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
    func processGeneratedOperation(request: [UInt8]) throws(ReallyMeCodecError) -> ReallyMeProtoCodecOperationResult {
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
    public func processOperation(_ request: [UInt8]) throws(ReallyMeCodecError) -> [UInt8] {
        try provider.processOperation(request: request)
    }

    /// Executes the generated ProtoJSON view of `CodecOperationRequest`.
    ///
    /// JSON is request-only; the returned bytes are the same binary result
    /// response returned by `processOperation(_:)`.
    public func processOperationJson(_ requestJson: [UInt8]) throws(ReallyMeCodecError) -> [UInt8] {
        try provider.processOperationJson(request: requestJson)
    }

    /// Decodes wipeable UTF-8 PEM armor bytes into a typed SDK owner.
    public func decodePem(
        _ pem: [UInt8],
        options: ReallyMePemDecodeOptions = ReallyMePemDecodeOptions()
    ) throws(ReallyMeCodecError) -> ReallyMePemDocument {
        var operationResult = try withOwnedBytes(
            pemDecodeOperationRequestBytes(
                pem: pem,
                options: options,
                maxFfiInputLength: maxOperationRequestBytes
            )
        ) { (request) throws(ReallyMeCodecError) in
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
    ) throws(ReallyMeCodecError) -> [UInt8] {
        var operationResult = try withOwnedBytes(
            pemEncodeOperationRequestBytes(
                label: label,
                der: der,
                options: options,
                maxFfiInputLength: maxOperationRequestBytes
            )
        ) { (request) throws(ReallyMeCodecError) in
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
