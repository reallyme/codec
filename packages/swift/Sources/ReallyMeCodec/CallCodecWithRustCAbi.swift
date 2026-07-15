// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

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

private typealias CodecBoolFunction = @convention(c) (
    UInt32,
    UnsafePointer<UInt8>?,
    Int,
    UnsafePointer<UInt8>?,
    Int,
    UnsafeMutablePointer<Int32>?
) -> Int32

#if REALLYME_CODEC_LINKED_FFI
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
    private let processProtoFunction: CodecProcessFunction
    private let boolFunction: CodecBoolFunction

    #if REALLYME_CODEC_LINKED_FFI
    init() {
        library = nil
        processFunction = rmCodecProcessLinked
        processProtoFunction = rmCodecProcessProtoLinked
        boolFunction = rmCodecProcessBoolLinked
    }
    #endif

    init(library: ReallyMeCodecRustCAbiLibrary) throws {
        self.library = library
        processFunction = try library.loadFunction("rm_codec_process", as: CodecProcessFunction.self)
        processProtoFunction = try library.loadFunction("rm_codec_process_proto", as: CodecProcessFunction.self)
        boolFunction = try library.loadFunction("rm_codec_process_bool", as: CodecBoolFunction.self)
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

    func processProto(operation: UInt32, first: [UInt8], second: [UInt8] = [], third: [UInt8] = []) throws -> [UInt8] {
        let output = try processWithStatus(
            processProtoFunction,
            operation: operation,
            first: first,
            second: second,
            third: third
        )
        if output.status == ReallyMeCodecRustCAbiStatus.protoError {
            throw ReallyMeCodecError.invalidInput
        }
        return output.bytes
    }

    func processProtoResult(
        operation: UInt32,
        first: [UInt8],
        second: [UInt8] = [],
        third: [UInt8] = []
    ) throws -> ReallyMeCodecProtoResult {
        let output = try processWithStatus(
            processProtoFunction,
            operation: operation,
            first: first,
            second: second,
            third: third
        )
        return ReallyMeCodecProtoResult(
            status: output.status == ReallyMeCodecRustCAbiStatus.protoError ? .codecError : .result,
            bytes: output.bytes
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
        if firstStatus != ReallyMeCodecRustCAbiStatus.bufferTooSmall {
            try ReallyMeCodecRustCAbiStatus.throwIfError(firstStatus)
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
        try ReallyMeCodecRustCAbiStatus.throwIfError(status)
        if producedLength < output.count {
            output.removeSubrange(producedLength..<output.count)
        }
        return (status, output)
    }

    func processBool(operation: UInt32, first: [UInt8], second: [UInt8] = []) throws -> Bool {
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
        return result != 0
    }
}
