// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
@testable import ReallyMeCodec
import SwiftProtobuf
import XCTest

final class GeneralCborReceiptOptionsTests: XCTestCase {
    func testExplicitTwoMiBBudgetPassesNativeTransportWithoutChangingDefaults() throws {
        let codec = try ReallyMeCodecTests.configuredCodec()
        let encodedLength = 2_097_152
        var encoded = Data([0x5a, 0x00, 0x1f, 0xff, 0xfb])
        encoded.append(Data(repeating: 0xa5, count: encodedLength - encoded.count))
        defer { encoded.resetBytes(in: encoded.startIndex..<encoded.endIndex) }
        var source = ReallyMeProtoCodecGeneralCborSourceRequest()
        source.encoded = encoded
        var request = ReallyMeProtoCodecApiOperationRequest()
        request.operation = .generalCborValidateExact(source)
        let rejected = try ReallyMeProtoCodecApiOperationResponse(
            serializedBytes: codec.processOperation(Array(request.serializedData()))
        )
        guard case .error(let error)? = rejected.outcome,
            case .generalCbor(let detail)? = error.error
        else { XCTFail("expected default input limit"); return }
        XCTAssertEqual(detail.detail, .inputLimit)

        var options = ReallyMeProtoCodecGeneralCborOptions()
        options.serialization = .received
        options.lengths = .definiteOnly
        options.floats = .all
        options.ordering = .received
        options.inputBytes = UInt64(encodedLength)
        options.byteStringBytes = UInt64(encodedLength)
        source.options = options
        request.operation = .generalCborValidateExact(source)
        let accepted = try ReallyMeProtoCodecApiOperationResponse(
            serializedBytes: codec.processOperation(Array(request.serializedData()))
        )
        guard case .result(let result)? = accepted.outcome,
            case .generalCborValidateExact(let frame)? = result.result
        else { XCTFail("expected raised input budget"); return }
        XCTAssertEqual(frame.received.length, UInt64(encodedLength))
        XCTAssertEqual(frame.options.inputBytes, UInt64(encodedLength))
        XCTAssertEqual(frame.options.outputBytes, 1_048_576)
    }

    func testCollectionOptionsAreSharedAndStandaloneOptionsRemainPresent() throws {
        let codec = try ReallyMeCodecTests.configuredCodec()
        var source = ReallyMeProtoCodecGeneralCborSourceRequest()
        source.encoded = Data([0, 1])
        var request = ReallyMeProtoCodecApiOperationRequest()
        request.operation = .generalCborCursor(source)
        let response = try ReallyMeProtoCodecApiOperationResponse(
            serializedBytes: codec.processOperation(Array(request.serializedData()))
        )
        guard case .result(let result)? = response.outcome,
            case .generalCborCursor(let frames)? = result.result
        else { XCTFail("expected cursor receipts"); return }
        XCTAssertTrue(frames.hasOptions)
        XCTAssertEqual(frames.options.entries, 4096)
        XCTAssertEqual(frames.options.nodes, 65536)
        XCTAssertEqual(frames.values.count, 2)
        XCTAssertEqual(frames.consumedBytes, 2)
        XCTAssertTrue(frames.values.allSatisfy { !$0.hasOptions })

        source.encoded = Data([0])
        request.operation = .generalCborFrameExact(source)
        let standalone = try ReallyMeProtoCodecApiOperationResponse(
            serializedBytes: codec.processOperation(Array(request.serializedData()))
        )
        guard case .result(let result)? = standalone.outcome,
            case .generalCborFrameExact(let frame)? = result.result
        else { XCTFail("expected standalone receipt"); return }
        XCTAssertTrue(frame.hasOptions)
        XCTAssertEqual(frame.options, frames.options)
    }
}
