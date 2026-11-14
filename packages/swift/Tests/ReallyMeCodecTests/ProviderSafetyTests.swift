// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#if canImport(CryptoKit)
    import CryptoKit
#endif
import Foundation
@testable import ReallyMeCodec
import XCTest

extension ReallyMeCodecTests {
    func testPublicValueReflectionRedactsStoredBytes() {
        let marker = "sensitive-reflection-marker"
        let value = ReallyMeDeterministicCborValue.text(marker)
        let entry = ReallyMeDeterministicCborMapEntry(key: .text(marker), value: value)
        let parsed = ReallyMeParsedMultikey(
            codecName: marker,
            algorithmName: marker,
            publicKey: Array(marker.utf8),
            expectedPublicKeyLength: nil
        )
        let metadata = ReallyMeMulticodecMetadata(
            name: marker,
            algorithmName: marker,
            tag: .key,
            keyMaterialKind: .publicKey,
            prefix: Array(marker.utf8),
            expectedKeyLength: nil
        )
        for secretBearingValue: Any in [value, entry, parsed, metadata] {
            var output = ""
            dump(secretBearingValue, to: &output)
            XCTAssertFalse(output.contains(marker))
        }
    }

    #if canImport(CryptoKit) && os(macOS)
        func testDigestPinnedLoaderRefusesMismatchedLibraries() throws {
            let path = try Self.libraryPath()
            let contents = try XCTUnwrap(FileManager.default.contents(atPath: path))
            let digest = Array(SHA256.hash(data: contents))

            let library = try ReallyMeCodecRustCAbiLibrary(path: path, expectedSHA256: digest)
            _ = try ReallyMeCodec(rustCAbiLibrary: library)

            var tampered = digest
            tampered[0] ^= 0x01
            XCTAssertThrowsError(
                try ReallyMeCodecRustCAbiLibrary(path: path, expectedSHA256: tampered)
            ) { error in
                XCTAssertEqual(error as? ReallyMeCodecError, .dynamicLibraryLoadFailed)
            }
            XCTAssertThrowsError(
                try ReallyMeCodecRustCAbiLibrary(path: path, expectedSHA256: Array(digest.prefix(31)))
            ) { error in
                XCTAssertEqual(error as? ReallyMeCodecError, .dynamicLibraryNotFound)
            }
        }

        func testLoaderRefusesDyldLibraryPathSubstitution() throws {
            let path = try Self.libraryPath()
            let resolvedPath = URL(fileURLWithPath: path).resolvingSymlinksInPath()
            let contents = try XCTUnwrap(FileManager.default.contents(atPath: resolvedPath.path))
            let digest = Array(SHA256.hash(data: contents))

            let searchDirectory = FileManager.default.temporaryDirectory
                .appendingPathComponent("reallyme-codec-dyld-\(UUID().uuidString)", isDirectory: true)
            try FileManager.default.createDirectory(at: searchDirectory, withIntermediateDirectories: false)
            defer { try? FileManager.default.removeItem(at: searchDirectory) }
            let substitute = searchDirectory.appendingPathComponent(resolvedPath.lastPathComponent)
            XCTAssertTrue(FileManager.default.createFile(atPath: substitute.path, contents: Data([0x00])))

            let previous = ProcessInfo.processInfo.environment["DYLD_LIBRARY_PATH"]
            setenv("DYLD_LIBRARY_PATH", searchDirectory.path, 1)
            defer {
                if let previous {
                    setenv("DYLD_LIBRARY_PATH", previous, 1)
                } else {
                    unsetenv("DYLD_LIBRARY_PATH")
                }
            }

            XCTAssertThrowsError(
                try ReallyMeCodecRustCAbiLibrary(path: path, expectedSHA256: digest)
            ) { error in
                XCTAssertEqual(error as? ReallyMeCodecError, .dynamicLibraryLoadFailed)
            }
            XCTAssertThrowsError(try ReallyMeCodecRustCAbiLibrary(path: path)) { error in
                XCTAssertEqual(error as? ReallyMeCodecError, .dynamicLibraryLoadFailed)
            }

            try FileManager.default.removeItem(at: substitute)
            let library = try ReallyMeCodecRustCAbiLibrary(path: path, expectedSHA256: digest)
            _ = try ReallyMeCodec(rustCAbiLibrary: library)
        }
    #endif

    func testProviderLoadingFailsClosed() throws {
        Self.assertCodecError(
            .dynamicLibraryNotFound,
            try ReallyMeCodecRustCAbiLibrary(path: "/tmp/reallyme-codec-missing-library.dylib")
        )
    }

    func testOwnedMemoryWipesUseTheEntireMutableRegion() {
        var bytes: [UInt8] = [0xA5, 0x5A, 0xFF]
        ReallyMeCodecMemory.clearOwned(&bytes)
        XCTAssertEqual(bytes, [0, 0, 0])

        let source = Data([0x11, 0x22, 0x33, 0x44])
        var slice = source[1..<3]
        ReallyMeCodecMemory.clearOwned(&slice)
        XCTAssertEqual(Array(slice), [0, 0])
        XCTAssertEqual(Array(source), [0x11, 0x22, 0x33, 0x44])
    }

    func testAbiVersionMismatchFailsClosed() throws {
        try ReallyMeCodecRustCAbiProvider.requireCompatibleAbiVersion(6)
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireCompatibleAbiVersion(0)
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireCompatibleAbiVersion(5)
        )
    }

    func testPackageVersionMismatchFailsClosed() throws {
        try ReallyMeCodecRustCAbiProvider.requireCompatiblePackageVersion(
            major: 0,
            minor: 4,
            patch: 0
        )
        let rejectedVersions: [(UInt32, UInt32, UInt32)] = [(0, 3, 0), (0, 4, 1), (1, 4, 0), (0, 3, 6)]
        for version in rejectedVersions {
            Self.assertCodecError(
                .providerFailure,
                try ReallyMeCodecRustCAbiProvider.requireCompatiblePackageVersion(
                    major: version.0,
                    minor: version.1,
                    patch: version.2
                )
            )
        }
    }

    func testProviderSuppliedOperationResponseLimitFailsClosed() throws {
        XCTAssertEqual(
            try ReallyMeCodecRustCAbiProvider.requireValidFfiLimit(67_108_864),
            67_108_864
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireValidFfiLimit(0)
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireValidFfiLimit(-1)
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireValidFfiLimit(UInt(Int.max) + 1)
        )
        XCTAssertEqual(
            try ReallyMeCodecRustCAbiProvider.requireValidOperationResponseLimit(
                16_781_312,
                maxFfiOutputLength: 67_108_864
            ),
            16_781_312
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireValidOperationResponseLimit(
                0,
                maxFfiOutputLength: 67_108_864
            )
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireValidOperationResponseLimit(
                -1,
                maxFfiOutputLength: 67_108_864
            )
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireValidOperationResponseLimit(
                67_108_865,
                maxFfiOutputLength: 67_108_864
            )
        )
        Self.assertCodecError(
            .providerFailure,
            try ReallyMeCodecRustCAbiProvider.requireValidOperationResponseLimit(
                UInt(Int.max) + 1,
                maxFfiOutputLength: 67_108_864
            )
        )
    }

    func testOversizedOperationRequestsReturnTheSameTypedEnvelope() throws {
        let codec = try Self.configuredCodec()
        let exactBinary = try ReallyMeProtoCodecOperationResponse(
            serializedBytes: codec.processOperation([UInt8](repeating: 0, count: 16_781_312))
        )
        XCTAssertEqual(exactBinary.error.boundary.reason, .boundaryMalformedProtobuf)
        let exactJson = try ReallyMeProtoCodecOperationResponse(
            serializedBytes: codec.processOperationJson([UInt8](repeating: 0, count: 22_373_720))
        )
        XCTAssertEqual(exactJson.error.boundary.reason, .boundaryMalformedJson)

        for excess in [1, 16] {
            let binary = try codec.processOperation([UInt8](repeating: 0, count: 16_781_312 + excess))
            let json = try codec.processOperationJson([UInt8](repeating: 0, count: 22_373_720 + excess))
            for responseBytes in [binary, json] {
                let response = try ReallyMeProtoCodecOperationResponse(serializedBytes: responseBytes)
                guard case .error(let error)? = response.outcome else {
                    XCTFail("expected boundary error envelope")
                    return
                }
                XCTAssertEqual(error.boundary.reason, .boundaryResourceLimitExceeded)
                XCTAssertEqual(error.origin, .caller)
            }
        }
    }

    func testProviderErrorOriginsAreAttributedDeterministically() {
        var boundary = ReallyMeProtoCodecBoundaryError()
        boundary.reason = .boundaryMalformedProtobuf
        var callerError = ReallyMeProtoCodecError()
        callerError.boundary = boundary
        callerError.origin = .caller
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecError(callerError),
            .invalidInput
        )

        var backend = ReallyMeProtoCodecBackendError()
        backend.reason = .backendInternal
        var providerError = ReallyMeProtoCodecError()
        providerError.backend = backend
        providerError.origin = .provider
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecError(providerError),
            .providerFailure
        )

        var mismatchedOrigin = callerError
        mismatchedOrigin.origin = .provider
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecError(mismatchedOrigin),
            .providerFailure
        )

        var unrecognizedReason = ReallyMeProtoCodecBoundaryError()
        unrecognizedReason.reason = .UNRECOGNIZED(699)
        var unknownError = ReallyMeProtoCodecError()
        unknownError.boundary = unrecognizedReason
        unknownError.origin = .caller
        XCTAssertEqual(
            ReallyMeCodecRustCAbiProvider.errorForCodecError(unknownError),
            .providerFailure
        )
    }

    func testSensitiveGeneratedMultikeyRequestFormattingIsRedacted() {
        var request = ReallyMeProtoCodecMultikeyParseRequest()
        request.multikey = "zSensitiveMultikey"
        var otherRequest = ReallyMeProtoCodecMultikeyParseRequest()
        otherRequest.multikey = "zDifferentSensitiveMultikey"

        XCTAssertEqual(
            request.debugDescription,
            "ReallyMeProtoCodecMultikeyParseRequest(<redacted>)"
        )
        XCTAssertEqual(
            request.textFormatString(),
            "ReallyMeProtoCodecMultikeyParseRequest(<redacted>)"
        )
        XCTAssertEqual(Mirror(reflecting: request).children.count, 0)
        var hasher = Hasher()
        request.hash(into: &hasher)
        let redactedHash = hasher.finalize()
        var otherHasher = Hasher()
        otherRequest.hash(into: &otherHasher)
        XCTAssertEqual(redactedHash, otherHasher.finalize())
    }
}
