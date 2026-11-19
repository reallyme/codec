// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
@testable import ReallyMeCodec
import SwiftProtobuf
import XCTest

final class GeneralCborReceiptOptionsTests: XCTestCase {
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
