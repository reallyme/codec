// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import Foundation
import ReallyMeCodecProto

private let expectedCodecAbiVersion: UInt32 = 2

private typealias CodecAbiVersionFunction = @convention(c) () -> UInt32
private typealias CodecProtoResultLimitFunction = @convention(c) () -> Int

private typealias CodecProcessFunction = @convention(c) (
    UInt32,
    UnsafePointer<UInt8>?,
    Int,
    UnsafePointer<UInt8>?,
    Int,
    UnsafePointer<UInt8>?,
    Int,
    UnsafeMutablePointer<UInt8>?,
    Int,
    UnsafeMutablePointer<Int>?
) -> Int32

private typealias CodecProcessProtoFunction = @convention(c) (
    UnsafePointer<UInt8>?,
    Int,
    UnsafeMutablePointer<UInt8>?,
    Int,
    UnsafeMutablePointer<Int>?
) -> Int32

private typealias CodecBoolFunction = @convention(c) (
    UInt32,
    UnsafePointer<UInt8>?,
    Int,
    UnsafePointer<UInt8>?,
    Int,
    UnsafeMutablePointer<Int32>?
) -> Int32

#if REALLYME_CODEC_LINKED_FFI
@_silgen_name("rm_codec_abi_version")
private func rmCodecAbiVersionLinked() -> UInt32

@_silgen_name("rm_codec_max_proto_result_envelope_bytes")
private func rmCodecMaxProtoResultEnvelopeBytesLinked() -> Int

@_silgen_name("rm_codec_process")
private func rmCodecProcessLinked(
    _ operation: UInt32,
    _ firstPointer: UnsafePointer<UInt8>?,
    _ firstLength: Int,
    _ secondPointer: UnsafePointer<UInt8>?,
    _ secondLength: Int,
    _ thirdPointer: UnsafePointer<UInt8>?,
    _ thirdLength: Int,
    _ outputPointer: UnsafeMutablePointer<UInt8>?,
    _ outputCapacity: Int,
    _ producedLength: UnsafeMutablePointer<Int>?
) -> Int32

@_silgen_name("rm_codec_process_proto")
private func rmCodecProcessProtoLinked(
    _ requestPointer: UnsafePointer<UInt8>?,
    _ requestLength: Int,
    _ outputPointer: UnsafeMutablePointer<UInt8>?,
    _ outputCapacity: Int,
    _ producedLength: UnsafeMutablePointer<Int>?
) -> Int32

@_silgen_name("rm_codec_process_proto_json")
private func rmCodecProcessProtoJsonLinked(
    _ requestPointer: UnsafePointer<UInt8>?,
    _ requestLength: Int,
    _ outputPointer: UnsafeMutablePointer<UInt8>?,
    _ outputCapacity: Int,
    _ producedLength: UnsafeMutablePointer<Int>?
) -> Int32

@_silgen_name("rm_codec_process_bool")
private func rmCodecProcessBoolLinked(
    _ operation: UInt32,
    _ firstPointer: UnsafePointer<UInt8>?,
    _ firstLength: Int,
    _ secondPointer: UnsafePointer<UInt8>?,
    _ secondLength: Int,
    _ resultPointer: UnsafeMutablePointer<Int32>?
) -> Int32
#endif

struct ReallyMeCodecRustCAbiProvider: Sendable {
    private static let maxFfiInputLength = 1_048_576
    private static let maxFfiOutputLength = 67_108_864

    // Keep the dlopen handle alive for as long as the dlsym function pointers
    // can be called; dropping it would let deinit dlclose the loaded image.
    private let library: ReallyMeCodecRustCAbiLibrary?
    private let processFunction: CodecProcessFunction
    private let processProtoFunction: CodecProcessProtoFunction
    private let processProtoJsonFunction: CodecProcessProtoFunction
    private let boolFunction: CodecBoolFunction
    private let maxProtoResultEnvelopeLength: Int

    #if REALLYME_CODEC_LINKED_FFI
    init() throws {
        try Self.requireCompatibleAbiVersion(rmCodecAbiVersionLinked())
        maxProtoResultEnvelopeLength = try Self.requireValidProtoResultEnvelopeLimit(
            rmCodecMaxProtoResultEnvelopeBytesLinked()
        )
        library = nil
        processFunction = rmCodecProcessLinked
        processProtoFunction = rmCodecProcessProtoLinked
        processProtoJsonFunction = rmCodecProcessProtoJsonLinked
        boolFunction = rmCodecProcessBoolLinked
    }
    #endif

    init(library: ReallyMeCodecRustCAbiLibrary) throws {
        self.library = library
        let abiVersionFunction = try library.loadFunction(
            "rm_codec_abi_version",
            as: CodecAbiVersionFunction.self
        )
        try Self.requireCompatibleAbiVersion(abiVersionFunction())
        let protoResultLimitFunction = try library.loadFunction(
            "rm_codec_max_proto_result_envelope_bytes",
            as: CodecProtoResultLimitFunction.self
        )
        maxProtoResultEnvelopeLength = try Self.requireValidProtoResultEnvelopeLimit(
            protoResultLimitFunction()
        )
        processFunction = try library.loadFunction("rm_codec_process", as: CodecProcessFunction.self)
        processProtoFunction = try library.loadFunction("rm_codec_process_proto", as: CodecProcessProtoFunction.self)
        processProtoJsonFunction = try library.loadFunction(
            "rm_codec_process_proto_json",
            as: CodecProcessProtoFunction.self
        )
        boolFunction = try library.loadFunction("rm_codec_process_bool", as: CodecBoolFunction.self)
    }

    static func requireCompatibleAbiVersion(_ actualVersion: UInt32) throws {
        guard actualVersion == expectedCodecAbiVersion else {
            throw ReallyMeCodecError.providerFailure
        }
    }

    static func requireValidProtoResultEnvelopeLimit(_ limit: Int) throws -> Int {
        guard limit > 0, limit <= maxFfiOutputLength else {
            throw ReallyMeCodecError.providerFailure
        }
        return limit
    }

    func process(operation: UInt32, first: [UInt8], second: [UInt8] = [], third: [UInt8] = []) throws -> [UInt8] {
        try processWithFunction(
            processFunction,
            operation: operation,
            first: first,
            second: second,
            third: third
        )
    }

    func processProto(request: [UInt8]) throws -> [UInt8] {
        var output = try processProtoResult(request: request)
        if output.status == .codecError {
            let error = Self.errorForCodecErrorPayload(output.bytes)
            ReallyMeCodecMemory.clearOwned(&output.bytes)
            throw error
        }
        return output.bytes
    }

    static func errorForCodecErrorPayload(_ bytes: [UInt8]) -> ReallyMeCodecError {
        let codecError: ReallyMeProtoCodecError
        do {
            codecError = try ReallyMeProtoCodecError(serializedBytes: bytes)
        } catch {
            return .providerFailure
        }
        switch codecError.error {
        case .baseEncoding(let error):
            return Self.inputErrorOrProviderFailure(error.reason, range: 100...199)
        case .pem(let error):
            return Self.inputErrorOrProviderFailure(error.reason, range: 200...299)
        case .multiformat(let error):
            return Self.inputErrorOrProviderFailure(error.reason, range: 300...399)
        case .canonicalization(let error):
            guard error.reason != .canonicalInternal else {
                return .providerFailure
            }
            return Self.inputErrorOrProviderFailure(error.reason, range: 400...499)
        case .backend:
            return .providerFailure
        case .boundary(let error):
            // Resource exhaustion is produced locally for oversized caller
            // values. Other boundary errors cannot come from a generated SDK
            // request and therefore indicate provider corruption or ABI skew.
            return error.reason == .boundaryResourceLimitExceeded
                ? .invalidInput
                : .providerFailure
        case nil:
            return .providerFailure
        }
    }

    private static func inputErrorOrProviderFailure(
        _ reason: ReallyMeProtoCodecErrorReason,
        range: ClosedRange<Int>
    ) -> ReallyMeCodecError {
        if case .UNRECOGNIZED = reason {
            return .providerFailure
        }
        return range.contains(reason.rawValue) ? .invalidInput : .providerFailure
    }

    func processProtoResult(request: [UInt8]) throws -> ReallyMeCodecProtoResult {
        let output = try processProtoEnvelope(request: request)
        return try Self.decodeProtoResultEnvelope(output)
    }

    static func decodeProtoResultEnvelope(
        _ output: consuming [UInt8]
    ) throws -> ReallyMeCodecProtoResult {
        var ownedOutput = consume output
        defer {
            ReallyMeCodecMemory.clearOwned(&ownedOutput)
        }
        var envelope: ReallyMeProtoCodecProtoResultEnvelope
        do {
            envelope = try ReallyMeProtoCodecProtoResultEnvelope(serializedBytes: ownedOutput)
        } catch {
            throw ReallyMeCodecError.providerFailure
        }
        defer {
            ReallyMeCodecMemory.clearOwned(&envelope.payload)
        }
        let status: ReallyMeCodecProtoStatus
        switch envelope.status {
        case .result:
            status = .result
        case .codecError:
            status = .codecError
        case .unspecified, .UNRECOGNIZED:
            throw ReallyMeCodecError.providerFailure
        }
        return ReallyMeCodecProtoResult(
            status: status,
            bytes: Array(envelope.payload)
        )
    }

    private func processWithFunction(
        _ function: CodecProcessFunction,
        operation: UInt32,
        first: [UInt8],
        second: [UInt8],
        third: [UInt8]
    ) throws -> [UInt8] {
        try processWithStatus(
            function,
            operation: operation,
            first: first,
            second: second,
            third: third
        ).bytes
    }

    private func processWithStatus(
        _ function: CodecProcessFunction,
        operation: UInt32,
        first: [UInt8],
        second: [UInt8],
        third: [UInt8]
    ) throws -> (status: Int32, bytes: [UInt8]) {
        try Self.validateAggregateInputLengths([first.count, second.count, third.count])
        var producedLength = 0
        let firstStatus = first.withUnsafeBufferPointer { firstBuffer in
            second.withUnsafeBufferPointer { secondBuffer in
                third.withUnsafeBufferPointer { thirdBuffer in
                    function(
                        operation,
                        firstBuffer.baseAddress,
                        first.count,
                        secondBuffer.baseAddress,
                        second.count,
                        thirdBuffer.baseAddress,
                        third.count,
                        nil,
                        0,
                        &producedLength
                    )
                }
            }
        }
        if firstStatus != ReallyMeCodecRustCAbiStatus.ok &&
            firstStatus != ReallyMeCodecRustCAbiStatus.bufferTooSmall {
            try ReallyMeCodecRustCAbiStatus.throwIfError(firstStatus)
        }
        let validEmptyProbe = firstStatus == ReallyMeCodecRustCAbiStatus.ok && producedLength == 0
        let validNonEmptyProbe = firstStatus == ReallyMeCodecRustCAbiStatus.bufferTooSmall &&
            producedLength > 0 && producedLength <= Self.maxFfiOutputLength
        guard validEmptyProbe || validNonEmptyProbe else {
            throw ReallyMeCodecError.providerFailure
        }
        if producedLength == 0 {
            return (firstStatus, [])
        }

        var output = [UInt8](repeating: 0, count: producedLength)
        let outputCapacity = output.count
        let status = first.withUnsafeBufferPointer { firstBuffer in
            second.withUnsafeBufferPointer { secondBuffer in
                third.withUnsafeBufferPointer { thirdBuffer in
                    output.withUnsafeMutableBufferPointer { outputBuffer in
                        function(
                            operation,
                            firstBuffer.baseAddress,
                            first.count,
                            secondBuffer.baseAddress,
                            second.count,
                            thirdBuffer.baseAddress,
                            third.count,
                            outputBuffer.baseAddress,
                            outputCapacity,
                            &producedLength
                        )
                    }
                }
            }
        }
        do {
            try ReallyMeCodecRustCAbiStatus.throwIfError(status)
        } catch {
            ReallyMeCodecMemory.clearOwned(&output)
            throw error
        }
        guard producedLength == output.count else {
            ReallyMeCodecMemory.clearOwned(&output)
            throw ReallyMeCodecError.providerFailure
        }
        return (status, output)
    }

    func processProtoEnvelope(request: [UInt8]) throws -> [UInt8] {
        try processProtoEnvelope(
            request: request,
            function: processProtoFunction
        )
    }

    func processProtoJsonEnvelope(requestJson: [UInt8]) throws -> [UInt8] {
        try processProtoEnvelope(
            request: requestJson,
            function: processProtoJsonFunction
        )
    }

    private func processProtoEnvelope(
        request: [UInt8],
        function: CodecProcessProtoFunction
    ) throws -> [UInt8] {
        var producedLength = 0
        let firstStatus = request.withUnsafeBufferPointer { requestBuffer in
            function(
                requestBuffer.baseAddress,
                request.count,
                nil,
                0,
                &producedLength
            )
        }
        guard firstStatus == ReallyMeCodecRustCAbiStatus.bufferTooSmall else {
            try ReallyMeCodecRustCAbiStatus.throwIfError(firstStatus)
            throw ReallyMeCodecError.providerFailure
        }
        guard producedLength > 0, producedLength <= maxProtoResultEnvelopeLength else {
            throw ReallyMeCodecError.providerFailure
        }

        var output = [UInt8](repeating: 0, count: producedLength)
        let outputCapacity = output.count
        let status = request.withUnsafeBufferPointer { requestBuffer in
            output.withUnsafeMutableBufferPointer { outputBuffer in
                function(
                    requestBuffer.baseAddress,
                    request.count,
                    outputBuffer.baseAddress,
                    outputCapacity,
                    &producedLength
                )
            }
        }
        do {
            try ReallyMeCodecRustCAbiStatus.throwIfError(status)
        } catch {
            ReallyMeCodecMemory.clearOwned(&output)
            throw error
        }
        guard producedLength == output.count else {
            ReallyMeCodecMemory.clearOwned(&output)
            throw ReallyMeCodecError.providerFailure
        }
        return output
    }

    func processBool(operation: UInt32, first: [UInt8], second: [UInt8] = []) throws -> Bool {
        try Self.validateAggregateInputLengths([first.count, second.count])
        var result: Int32 = 0
        let status = first.withUnsafeBufferPointer { firstBuffer in
            second.withUnsafeBufferPointer { secondBuffer in
                boolFunction(
                    operation,
                    firstBuffer.baseAddress,
                    first.count,
                    secondBuffer.baseAddress,
                    second.count,
                    &result
                )
            }
        }
        try ReallyMeCodecRustCAbiStatus.throwIfError(status)
        switch result {
        case 0:
            return false
        case 1:
            return true
        default:
            throw ReallyMeCodecError.providerFailure
        }
    }

    private static func validateAggregateInputLengths(_ lengths: [Int]) throws {
        var aggregate = 0
        for length in lengths {
            let (next, overflow) = aggregate.addingReportingOverflow(length)
            guard !overflow, next <= maxFfiInputLength else {
                throw ReallyMeCodecError.invalidInput
            }
            aggregate = next
        }
    }
}
