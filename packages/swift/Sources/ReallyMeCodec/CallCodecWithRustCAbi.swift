// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation

private let expectedCodecAbiVersion: UInt32 = 6
private let expectedCodecPackageMajor: UInt32 = 0
private let expectedCodecPackageMinor: UInt32 = 3
private let expectedCodecPackagePatch: UInt32 = 1
private let maxProtoJsonRequestLength = 16_082_264

private typealias CodecAbiVersionFunction = @convention(c) () -> UInt32
private typealias CodecSizeLimitFunction = @convention(c) () -> UInt

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

@_silgen_name("rm_codec_package_version_major")
private func rmCodecPackageVersionMajorLinked() -> UInt32

@_silgen_name("rm_codec_package_version_minor")
private func rmCodecPackageVersionMinorLinked() -> UInt32

@_silgen_name("rm_codec_package_version_patch")
private func rmCodecPackageVersionPatchLinked() -> UInt32

@_silgen_name("rm_codec_max_operation_response_bytes")
private func rmCodecMaxOperationResponseBytesLinked() -> UInt

@_silgen_name("rm_codec_max_ffi_input_bytes")
private func rmCodecMaxFfiInputBytesLinked() -> UInt

@_silgen_name("rm_codec_max_ffi_output_bytes")
private func rmCodecMaxFfiOutputBytesLinked() -> UInt

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

@_silgen_name("rm_codec_process_operation_json")
private func rmCodecProcessOperationJsonLinked(
    _ requestPointer: UnsafePointer<UInt8>?,
    _ requestLength: Int,
    _ outputPointer: UnsafeMutablePointer<UInt8>?,
    _ outputCapacity: Int,
    _ producedLength: UnsafeMutablePointer<Int>?
) -> Int32

@_silgen_name("rm_codec_process_operation")
private func rmCodecProcessOperationLinked(
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
    // Keep the dlopen handle alive for as long as the dlsym function pointers
    // can be called; dropping it would let deinit dlclose the loaded image.
    private let library: ReallyMeCodecRustCAbiLibrary?
    private let processFunction: CodecProcessFunction
    private let processOperationFunction: CodecProcessProtoFunction
    private let processOperationJsonFunction: CodecProcessProtoFunction
    private let boolFunction: CodecBoolFunction
    private let maxFfiInputLength: Int
    private let maxFfiOutputLength: Int
    private let maxOperationResponseLength: Int

    #if REALLYME_CODEC_LINKED_FFI
    init() throws {
        try Self.requireCompatibleAbiVersion(rmCodecAbiVersionLinked())
        try Self.requireCompatiblePackageVersion(
            major: rmCodecPackageVersionMajorLinked(),
            minor: rmCodecPackageVersionMinorLinked(),
            patch: rmCodecPackageVersionPatchLinked()
        )
        maxFfiInputLength = try Self.requireValidFfiLimit(rmCodecMaxFfiInputBytesLinked())
        maxFfiOutputLength = try Self.requireValidFfiLimit(rmCodecMaxFfiOutputBytesLinked())
        maxOperationResponseLength = try Self.requireValidOperationResponseLimit(
            rmCodecMaxOperationResponseBytesLinked(),
            maxFfiOutputLength: maxFfiOutputLength
        )
        library = nil
        processFunction = rmCodecProcessLinked
        processOperationFunction = rmCodecProcessOperationLinked
        processOperationJsonFunction = rmCodecProcessOperationJsonLinked
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
        let packageMajorFunction = try library.loadFunction(
            "rm_codec_package_version_major",
            as: CodecAbiVersionFunction.self
        )
        let packageMinorFunction = try library.loadFunction(
            "rm_codec_package_version_minor",
            as: CodecAbiVersionFunction.self
        )
        let packagePatchFunction = try library.loadFunction(
            "rm_codec_package_version_patch",
            as: CodecAbiVersionFunction.self
        )
        try Self.requireCompatiblePackageVersion(
            major: packageMajorFunction(),
            minor: packageMinorFunction(),
            patch: packagePatchFunction()
        )
        let inputLimitFunction = try library.loadFunction(
            "rm_codec_max_ffi_input_bytes",
            as: CodecSizeLimitFunction.self
        )
        let outputLimitFunction = try library.loadFunction(
            "rm_codec_max_ffi_output_bytes",
            as: CodecSizeLimitFunction.self
        )
        maxFfiInputLength = try Self.requireValidFfiLimit(inputLimitFunction())
        maxFfiOutputLength = try Self.requireValidFfiLimit(outputLimitFunction())
        let operationResponseLimitFunction = try library.loadFunction(
            "rm_codec_max_operation_response_bytes",
            as: CodecSizeLimitFunction.self
        )
        maxOperationResponseLength = try Self.requireValidOperationResponseLimit(
            operationResponseLimitFunction(),
            maxFfiOutputLength: maxFfiOutputLength
        )
        processFunction = try library.loadFunction("rm_codec_process", as: CodecProcessFunction.self)
        processOperationFunction = try library.loadFunction(
            "rm_codec_process_operation",
            as: CodecProcessProtoFunction.self
        )
        processOperationJsonFunction = try library.loadFunction(
            "rm_codec_process_operation_json",
            as: CodecProcessProtoFunction.self
        )
        boolFunction = try library.loadFunction("rm_codec_process_bool", as: CodecBoolFunction.self)
    }

    static func requireCompatibleAbiVersion(_ actualVersion: UInt32) throws {
        guard actualVersion == expectedCodecAbiVersion else {
            throw ReallyMeCodecError.providerFailure
        }
    }

    static func requireCompatiblePackageVersion(major: UInt32, minor: UInt32, patch: UInt32) throws {
        guard major == expectedCodecPackageMajor,
              minor == expectedCodecPackageMinor,
              patch == expectedCodecPackagePatch else {
            throw ReallyMeCodecError.providerFailure
        }
    }

    static func requireValidFfiLimit(_ limit: UInt) throws -> Int {
        guard let converted = Int(exactly: limit), converted > 0 else {
            throw ReallyMeCodecError.providerFailure
        }
        return converted
    }

    static func requireValidOperationResponseLimit(_ limit: UInt, maxFfiOutputLength: Int) throws -> Int {
        guard let converted = Int(exactly: limit),
              converted > 0,
              converted <= maxFfiOutputLength else {
            throw ReallyMeCodecError.providerFailure
        }
        return converted
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

    var ffiInputLimit: Int {
        maxFfiInputLength
    }

    func processOperation(request: [UInt8]) throws -> [UInt8] {
        try processOperationResponse(
            request: request,
            maxRequestLength: maxOperationResponseLength,
            function: processOperationFunction
        )
    }

    func processOperationJson(request: [UInt8]) throws -> [UInt8] {
        try processOperationResponse(
            request: request,
            maxRequestLength: maxProtoJsonRequestLength,
            function: processOperationJsonFunction
        )
    }

    static func errorForCodecError(_ codecError: ReallyMeProtoCodecError) -> ReallyMeCodecError {
        guard codecError.unknownFields.data.isEmpty else {
            return .providerFailure
        }
        let expectedOrigin: ReallyMeProtoCodecErrorOrigin
        switch codecError.error {
        case .baseEncoding(let error):
            guard error.unknownFields.data.isEmpty,
                  Self.isKnownReason(error.reason, range: 100...199) else { return .providerFailure }
            expectedOrigin = .caller
        case .pem(let error):
            guard error.unknownFields.data.isEmpty,
                  Self.isKnownReason(error.reason, range: 200...299) else { return .providerFailure }
            expectedOrigin = .caller
        case .multiformat(let error):
            guard error.unknownFields.data.isEmpty,
                  Self.isKnownReason(error.reason, range: 300...399) else { return .providerFailure }
            expectedOrigin = .caller
        case .canonicalization(let error):
            guard error.unknownFields.data.isEmpty,
                  Self.isKnownReason(error.reason, range: 400...499) else { return .providerFailure }
            expectedOrigin = error.reason == .canonicalInternal ? .provider : .caller
        case .backend(let error):
            guard error.unknownFields.data.isEmpty,
                  Self.isKnownReason(error.reason, range: 500...599) else { return .providerFailure }
            expectedOrigin = .provider
        case .boundary(let error):
            guard error.unknownFields.data.isEmpty,
                  Self.isKnownReason(error.reason, range: 600...699) else { return .providerFailure }
            expectedOrigin = .caller
        case nil:
            return .providerFailure
        }
        guard codecError.origin == expectedOrigin else {
            return .providerFailure
        }
        guard expectedOrigin == .caller else { return .providerFailure }
        switch codecError.error {
        case .baseEncoding(let error) where error.reason == .baseNonCanonicalHex:
            return .nonCanonical
        case .multiformat(let error) where error.reason == .multiformatUnknownMulticodec ||
            error.reason == .multiformatInvalidMulticodecPrefix:
            return .unsupportedCodec
        case .canonicalization(let error) where error.reason == .canonicalUnsupportedIpldValue:
            return .unsupportedIpldValue
        case .canonicalization(let error) where Self.isNonCanonicalReason(error.reason):
            return .nonCanonical
        default:
            return .invalidInput
        }
    }

    private static func isNonCanonicalReason(
        _ reason: ReallyMeProtoCodecErrorReason
    ) -> Bool {
        switch reason {
        case .canonicalNonCanonicalCbor,
             .canonicalNonCanonicalJson,
             .canonicalNonMinimalCborInteger,
             .canonicalDuplicateCborMapKey,
             .canonicalCborMapKeysOutOfOrder,
             .canonicalCborTrailingBytes:
            return true
        default:
            return false
        }
    }

    private static func isKnownReason(
        _ reason: ReallyMeProtoCodecErrorReason,
        range: ClosedRange<Int>
    ) -> Bool {
        if case .UNRECOGNIZED = reason {
            return false
        }
        return range.contains(reason.rawValue)
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
        try validateAggregateInputLengths([first.count, second.count, third.count])
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
            producedLength > 0 && producedLength <= maxFfiOutputLength
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

    private func processOperationResponse(
        request: [UInt8],
        maxRequestLength: Int,
        function: CodecProcessProtoFunction
    ) throws -> [UInt8] {
        if request.count > maxRequestLength {
            var boundary = ReallyMeProtoCodecBoundaryError()
            boundary.reason = .boundaryResourceLimitExceeded
            var error = ReallyMeProtoCodecError()
            error.boundary = boundary
            error.origin = .caller
            var response = ReallyMeProtoCodecOperationResponse()
            response.error = error
            return Array(try response.serializedData())
        }
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
        guard producedLength > 0, producedLength <= maxOperationResponseLength else {
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
        try validateAggregateInputLengths([first.count, second.count])
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

    private func validateAggregateInputLengths(_ lengths: [Int]) throws {
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
