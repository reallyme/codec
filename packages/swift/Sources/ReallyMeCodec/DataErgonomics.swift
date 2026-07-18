// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import Foundation

public extension ReallyMeMulticodecMetadata {
    var prefixData: Data {
        Data(prefix)
    }
}

public extension ReallyMeParsedMultikey {
    var publicKeyData: Data {
        Data(publicKey)
    }
}

public extension ReallyMePemDocument {
    var derData: Data {
        Data(der)
    }
}

public extension ReallyMeCodec {
    func base64Encode(_ data: Data) throws -> String {
        try withReallyMeDataBytes(data) { bytes in
            try base64Encode(bytes)
        }
    }

    func base64DecodeData(_ text: String) throws -> Data {
        try dataResult {
            try base64Decode(text)
        }
    }

    func base64urlEncode(_ data: Data) throws -> String {
        try withReallyMeDataBytes(data) { bytes in
            try base64urlEncode(bytes)
        }
    }

    func base64urlDecodeData(_ text: String) throws -> Data {
        try dataResult {
            try base64urlDecode(text)
        }
    }

    func bytesToLowerHex(_ data: Data) throws -> String {
        try withReallyMeDataBytes(data) { bytes in
            try bytesToLowerHex(bytes)
        }
    }

    func lowerHexToData(_ text: String) throws -> Data {
        try dataResult {
            try lowerHexToBytes(text)
        }
    }

    func base58btcEncode(_ data: Data) throws -> String {
        try withReallyMeDataBytes(data) { bytes in
            try base58btcEncode(bytes)
        }
    }

    func base58btcDecodeData(_ text: String) throws -> Data {
        try dataResult {
            try base58btcDecode(text)
        }
    }

    func multibaseBase58btcEncode(_ data: Data) throws -> String {
        try withReallyMeDataBytes(data) { bytes in
            try multibaseBase58btcEncode(bytes)
        }
    }

    func multibaseBase64urlEncode(_ data: Data) throws -> String {
        try withReallyMeDataBytes(data) { bytes in
            try multibaseBase64urlEncode(bytes)
        }
    }

    func multibaseDecodeData(_ text: String) throws -> Data {
        try dataResult {
            try multibaseDecode(text)
        }
    }

    func multicodecLookupPrefix(_ data: Data) throws -> ReallyMeMulticodecLookupResult {
        try withReallyMeDataBytes(data) { bytes in
            try multicodecLookupPrefix(bytes)
        }
    }

    func multicodecStripPrefix(_ data: Data) throws -> Data {
        try withReallyMeDataBytes(data) { bytes in
            try dataResult {
                try multicodecStripPrefix(bytes)
            }
        }
    }

    func multikeyEncode(codecName: String, publicKey: Data) throws -> String {
        try withReallyMeDataBytes(publicKey) { bytes in
            try multikeyEncode(codecName: codecName, publicKey: bytes)
        }
    }

    func dagCborEncodeData(_ value: ReallyMeDeterministicCborValue) throws -> Data {
        try dataResult {
            try dagCborEncode(value)
        }
    }

    func dagCborDecode(_ data: Data) throws -> ReallyMeDeterministicCborValue {
        try withReallyMeDataBytes(data) { bytes in
            try dagCborDecode(bytes)
        }
    }

    func dagCborComputeCid(_ data: Data) throws -> String {
        try withReallyMeDataBytes(data) { bytes in
            try dagCborComputeCid(bytes)
        }
    }

    func dagCborVerifyCid(
        cid: String,
        data: Data
    ) throws -> ReallyMeDagCborCidVerification {
        try withReallyMeDataBytes(data) { bytes in
            try dagCborVerifyCid(cid: cid, bytes: bytes)
        }
    }

    func dagCborSha256ContentHashData(_ data: Data) throws -> Data {
        try withReallyMeDataBytes(data) { bytes in
            try dataResult {
                try dagCborSha256ContentHash(bytes)
            }
        }
    }

    func dagCborMultihashData(_ data: Data) throws -> Data {
        try withReallyMeDataBytes(data) { bytes in
            try dataResult {
                try dagCborMultihash(bytes)
            }
        }
    }

    func deterministicCborEncodeData(
        _ value: ReallyMeDeterministicCborValue
    ) throws -> Data {
        try dataResult {
            try deterministicCborEncode(value)
        }
    }

    func deterministicCborDecode(_ data: Data) throws -> ReallyMeDeterministicCborValue {
        try withReallyMeDataBytes(data) { bytes in
            try deterministicCborDecode(bytes)
        }
    }

    func processOperation(_ request: Data) throws -> Data {
        try withReallyMeDataBytes(request) { bytes in
            try dataResult {
                try processOperation(bytes)
            }
        }
    }

    func processOperationJson(_ requestJson: Data) throws -> Data {
        try withReallyMeDataBytes(requestJson) { bytes in
            try dataResult {
                try processOperationJson(bytes)
            }
        }
    }

    func decodePem(
        _ pem: Data,
        options: ReallyMePemDecodeOptions = ReallyMePemDecodeOptions()
    ) throws -> ReallyMePemDocument {
        try withReallyMeDataBytes(pem) { bytes in
            try decodePem(bytes, options: options)
        }
    }

    func encodePemData(
        label: ReallyMePemLabel,
        der: Data,
        options: ReallyMePemEncodeOptions = ReallyMePemEncodeOptions()
    ) throws -> Data {
        try withReallyMeDataBytes(der) { bytes in
            try dataResult {
                try encodePem(label: label, der: bytes, options: options)
            }
        }
    }
}

private func withReallyMeDataBytes<T>(
    _ data: Data,
    _ body: ([UInt8]) throws -> T
) rethrows -> T {
    var bytes = Array(data)
    defer {
        ReallyMeCodecMemory.clearOwned(&bytes)
    }
    return try body(bytes)
}

private func dataResult(_ body: () throws -> [UInt8]) throws -> Data {
    var bytes = try body()
    defer {
        ReallyMeCodecMemory.clearOwned(&bytes)
    }
    return Data(bytes)
}
