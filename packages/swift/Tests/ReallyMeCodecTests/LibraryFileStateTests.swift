// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
@testable import ReallyMeCodec
import XCTest

#if canImport(Darwin)
    extension ReallyMeCodecTests {
        func testLibraryFileStateDetectsInPlaceWritesWithRestoredLengthAndMtime() throws {
            let url = FileManager.default.temporaryDirectory
                .appendingPathComponent("reallyme-codec-file-state-\(UUID().uuidString)")
            XCTAssertTrue(FileManager.default.createFile(atPath: url.path, contents: Data([1, 2, 3, 4])))
            defer { try? FileManager.default.removeItem(at: url) }
            let originalDate = Date(timeIntervalSince1970: 1_700_000_000)
            try FileManager.default.setAttributes([.modificationDate: originalDate], ofItemAtPath: url.path)
            let before = try XCTUnwrap(LoadedFileIdentity(path: url.path))
            let writer = try FileHandle(forWritingTo: url)
            try writer.write(contentsOf: Data([4, 3, 2, 1]))
            try writer.close()
            try FileManager.default.setAttributes([.modificationDate: originalDate], ofItemAtPath: url.path)
            let after = try XCTUnwrap(LoadedFileIdentity(path: url.path))
            XCTAssertEqual(before.device, after.device)
            XCTAssertEqual(before.inode, after.inode)
            XCTAssertEqual(before.size, after.size)
            XCTAssertEqual(before.modifiedSeconds, after.modifiedSeconds)
            XCTAssertEqual(before.modifiedNanoseconds, after.modifiedNanoseconds)
            XCTAssertNotEqual(before, after)
        }

        func testLibraryFileStateRejectsMissingFilesAndDirectories() throws {
            let missing = FileManager.default.temporaryDirectory
                .appendingPathComponent("reallyme-codec-missing-state-\(UUID().uuidString)")
            XCTAssertNil(LoadedFileIdentity(path: missing.path))
            XCTAssertNil(LoadedFileIdentity(path: FileManager.default.temporaryDirectory.path))
        }
    }
#endif
