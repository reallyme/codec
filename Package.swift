// swift-tools-version: 6.3
// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Root manifest for `reallyme-codec`.
//
// SwiftPM and Xcode only read `Package.swift` at the repository root when a
// package is consumed by URL, e.g.
//
//     .package(url: "https://github.com/reallyme/codec", from: "0.3.0")
//     .product(name: "ReallyMeCodec", package: "codec")
//
// The Swift sources live under `packages/swift/` to keep symmetry with the
// other language lanes; this manifest points its targets there explicitly so
// there is a single source of truth.

import PackageDescription
import Foundation

let ffiArtifactChecksumPlaceholder =
    "0000000000000000000000000000000000000000000000000000000000000000"
let ffiArtifactChecksum = "9ccc23f11af17ca292ad14734135b048c6fc76ea0ff51547ea551b335afbe569"
let ffiArtifactVersion = "0.3.0"
let ffiArtifactLocalPathOverride = ""
let packageVersion = "0.3.0"
let hasReleasedFfiArtifact =
    ffiArtifactChecksum != ffiArtifactChecksumPlaceholder && ffiArtifactVersion == packageVersion
let useRuntimeFfiProvider =
    ProcessInfo.processInfo.environment["REALLYME_CODEC_SWIFTPM_RUNTIME_FFI"] == "1"
let runtimeFfiLibraryPath =
    ProcessInfo.processInfo.environment["REALLYME_CODEC_FFI_LIBRARY_PATH"] ?? ""
var runtimeFfiPathIsDirectory: ObjCBool = false
let runtimeFfiPathExists = FileManager.default.fileExists(
    atPath: runtimeFfiLibraryPath,
    isDirectory: &runtimeFfiPathIsDirectory
)

// The runtime override is for explicit local integration tests. A stray
// environment flag must not silently remove the binary dependency from a
// consumer's package graph.
if useRuntimeFfiProvider &&
    (runtimeFfiLibraryPath.isEmpty ||
     !runtimeFfiPathExists || runtimeFfiPathIsDirectory.boolValue) {
    fputs("REALLYME_CODEC_SWIFTPM_RUNTIME_FFI requires an existing FFI library file.\n", stderr)
    exit(1)
}

var codecTargetDependencies: [Target.Dependency] = []
var codecSwiftSettings: [SwiftSetting] = []
var packageTargets: [Target] = []

if hasReleasedFfiArtifact && !useRuntimeFfiProvider {
    codecTargetDependencies.append("ReallyMeCodecFFI")
    codecSwiftSettings.append(.define("REALLYME_CODEC_LINKED_FFI"))
    if ffiArtifactLocalPathOverride.isEmpty {
        packageTargets.append(
            .binaryTarget(
                name: "ReallyMeCodecFFI",
                url: "https://github.com/reallyme/codec/releases/download/v\(ffiArtifactVersion)/ReallyMeCodecFFI.xcframework.zip",
                checksum: ffiArtifactChecksum
            )
        )
    } else {
        packageTargets.append(
            .binaryTarget(
                name: "ReallyMeCodecFFI",
                path: ffiArtifactLocalPathOverride
            )
        )
    }
}

codecTargetDependencies.append(
    .product(name: "SwiftProtobuf", package: "swift-protobuf")
)

packageTargets.append(
    .target(
        name: "ReallyMeCodec",
        dependencies: codecTargetDependencies,
        path: "packages/swift/Sources/ReallyMeCodec",
        swiftSettings: codecSwiftSettings
    )
)
packageTargets.append(
    .testTarget(
        name: "ReallyMeCodecTests",
        dependencies: ["ReallyMeCodec"],
        path: "packages/swift/Tests/ReallyMeCodecTests",
        swiftSettings: codecSwiftSettings
    )
)

let package = Package(
    name: "reallyme-codec",
    platforms: [
        .macOS(.v13),
        .iOS(.v16),
    ],
    products: [
        .library(
            name: "ReallyMeCodec",
            targets: ["ReallyMeCodec"]
        ),
    ],
    dependencies: [
        .package(
            url: "https://github.com/apple/swift-protobuf.git",
            from: "1.38.1"
        ),
    ],
    targets: packageTargets
)
