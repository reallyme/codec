// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

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
    func base64Encode(_ data: Data) throws(ReallyMeCodecError) -> String {
        try withReallyMeDataBytes(data) { (bytes) throws(ReallyMeCodecError) in
            try base64Encode(bytes)
        }
    }

    func base64DecodeData(_ text: String) throws(ReallyMeCodecError) -> Data {
        try dataResult { () throws(ReallyMeCodecError) in
            try base64Decode(text)
        }
    }

    func base64urlEncode(_ data: Data) throws(ReallyMeCodecError) -> String {
        try withReallyMeDataBytes(data) { (bytes) throws(ReallyMeCodecError) in
            try base64urlEncode(bytes)
        }
    }

    func base64urlDecodeData(_ text: String) throws(ReallyMeCodecError) -> Data {
        try dataResult { () throws(ReallyMeCodecError) in
            try base64urlDecode(text)
        }
    }

    func bytesToLowerHex(_ data: Data) throws(ReallyMeCodecError) -> String {
        try withReallyMeDataBytes(data) { (bytes) throws(ReallyMeCodecError) in
            try bytesToLowerHex(bytes)
        }
    }

    func lowerHexToData(_ text: String) throws(ReallyMeCodecError) -> Data {
        try dataResult { () throws(ReallyMeCodecError) in
            try lowerHexToBytes(text)
        }
    }

    func base58btcEncode(_ data: Data) throws(ReallyMeCodecError) -> String {
        try withReallyMeDataBytes(data) { (bytes) throws(ReallyMeCodecError) in
            try base58btcEncode(bytes)
        }
    }

    func base58btcDecodeData(_ text: String) throws(ReallyMeCodecError) -> Data {
        try dataResult { () throws(ReallyMeCodecError) in
            try base58btcDecode(text)
        }
    }

    func multibaseBase58btcEncode(_ data: Data) throws(ReallyMeCodecError) -> String {
        try withReallyMeDataBytes(data) { (bytes) throws(ReallyMeCodecError) in
            try multibaseBase58btcEncode(bytes)
        }
    }

    func multibaseBase64urlEncode(_ data: Data) throws(ReallyMeCodecError) -> String {
        try withReallyMeDataBytes(data) { (bytes) throws(ReallyMeCodecError) in
            try multibaseBase64urlEncode(bytes)
        }
    }

    func multibaseDecodeData(_ text: String) throws(ReallyMeCodecError) -> Data {
        try dataResult { () throws(ReallyMeCodecError) in
            try multibaseDecode(text)
        }
    }

    func multicodecLookupPrefix(_ data: Data) throws(ReallyMeCodecError) -> ReallyMeMulticodecLookupResult {
        try withReallyMeDataBytes(data) { (bytes) throws(ReallyMeCodecError) in
            try multicodecLookupPrefix(bytes)
        }
    }

    func multicodecStripPrefix(_ data: Data) throws(ReallyMeCodecError) -> Data {
        try withReallyMeDataBytes(data) { (bytes) throws(ReallyMeCodecError) in
            try dataResult { () throws(ReallyMeCodecError) in
                try multicodecStripPrefix(bytes)
            }
        }
    }

    func multikeyEncode(codecName: String, publicKey: Data) throws(ReallyMeCodecError) -> String {
        try withReallyMeDataBytes(publicKey) { (bytes) throws(ReallyMeCodecError) in
            try multikeyEncode(codecName: codecName, publicKey: bytes)
        }
    }

    func dagCborEncodeData(_ value: ReallyMeDeterministicCborValue) throws(ReallyMeCodecError) -> Data {
        try dataResult { () throws(ReallyMeCodecError) in
            try dagCborEncode(value)
        }
    }

    func dagCborDecode(_ data: Data) throws(ReallyMeCodecError) -> ReallyMeDeterministicCborValue {
        try withReallyMeDataBytes(data) { (bytes) throws(ReallyMeCodecError) in
            try dagCborDecode(bytes)
        }
    }

    func dagCborComputeCid(_ data: Data) throws(ReallyMeCodecError) -> String {
        try withReallyMeDataBytes(data) { (bytes) throws(ReallyMeCodecError) in
            try dagCborComputeCid(bytes)
        }
    }

    func dagCborVerifyCid(
        cid: String,
        data: Data
    ) throws(ReallyMeCodecError) -> ReallyMeDagCborCidVerification {
        try withReallyMeDataBytes(data) { (bytes) throws(ReallyMeCodecError) in
            try dagCborVerifyCid(cid: cid, bytes: bytes)
        }
    }

    func dagCborSha256ContentHashData(_ data: Data) throws(ReallyMeCodecError) -> Data {
        try withReallyMeDataBytes(data) { (bytes) throws(ReallyMeCodecError) in
            try dataResult { () throws(ReallyMeCodecError) in
                try dagCborSha256ContentHash(bytes)
            }
        }
    }

    func dagCborMultihashData(_ data: Data) throws(ReallyMeCodecError) -> Data {
        try withReallyMeDataBytes(data) { (bytes) throws(ReallyMeCodecError) in
            try dataResult { () throws(ReallyMeCodecError) in
                try dagCborMultihash(bytes)
            }
        }
    }

    func deterministicCborEncodeData(
        _ value: ReallyMeDeterministicCborValue
    ) throws(ReallyMeCodecError) -> Data {
        try dataResult { () throws(ReallyMeCodecError) in
            try deterministicCborEncode(value)
        }
    }

    func deterministicCborDecode(_ data: Data) throws(ReallyMeCodecError) -> ReallyMeDeterministicCborValue {
        try withReallyMeDataBytes(data) { (bytes) throws(ReallyMeCodecError) in
            try deterministicCborDecode(bytes)
        }
    }

    func processOperation(_ request: Data) throws(ReallyMeCodecError) -> Data {
        try withReallyMeDataBytes(request) { (bytes) throws(ReallyMeCodecError) in
            try dataResult { () throws(ReallyMeCodecError) in
                try processOperation(bytes)
            }
        }
    }

    func processOperationJson(_ requestJson: Data) throws(ReallyMeCodecError) -> Data {
        try withReallyMeDataBytes(requestJson) { (bytes) throws(ReallyMeCodecError) in
            try dataResult { () throws(ReallyMeCodecError) in
                try processOperationJson(bytes)
            }
        }
    }

    func decodePem(
        _ pem: Data,
        options: ReallyMePemDecodeOptions = ReallyMePemDecodeOptions()
    ) throws(ReallyMeCodecError) -> ReallyMePemDocument {
        try withReallyMeDataBytes(pem) { (bytes) throws(ReallyMeCodecError) in
            try decodePem(bytes, options: options)
        }
    }

    func encodePemData(
        label: ReallyMePemLabel,
        der: Data,
        options: ReallyMePemEncodeOptions = ReallyMePemEncodeOptions()
    ) throws(ReallyMeCodecError) -> Data {
        try withReallyMeDataBytes(der) { (bytes) throws(ReallyMeCodecError) in
            try dataResult { () throws(ReallyMeCodecError) in
                try encodePem(label: label, der: bytes, options: options)
            }
        }
    }
}

private func withReallyMeDataBytes<T>(
    _ data: Data,
    _ body: ([UInt8]) throws(ReallyMeCodecError) -> T
) throws(ReallyMeCodecError) -> T {
    var bytes = Array(data)
    defer {
        ReallyMeCodecMemory.clearOwned(&bytes)
    }
    return try body(bytes)
}

private func dataResult(_ body: () throws(ReallyMeCodecError) -> [UInt8]) throws(ReallyMeCodecError) -> Data {
    var bytes = try body()
    defer {
        ReallyMeCodecMemory.clearOwned(&bytes)
    }
    return Data(bytes)
}
