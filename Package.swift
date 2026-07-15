// swift-tools-version: 6.0
// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

// Root manifest for `reallyme-codec`.
//
// SwiftPM and Xcode only read `Package.swift` at the repository root when a
// package is consumed by URL, e.g.
//
//     .package(url: "https://github.com/reallyme/codec", from: "0.1.20")
//     .product(name: "ReallyMeCodec", package: "codec")
//
// The Swift sources live under `packages/swift/` to keep symmetry with the
// other language lanes; this manifest points its targets there explicitly so
// there is a single source of truth.

import PackageDescription
import Foundation

let ffiArtifactChecksumPlaceholder =
    "0000000000000000000000000000000000000000000000000000000000000000"
let ffiArtifactChecksum = "7c2b191d0e7393d8d2b0a16b383427a2a5ede0a2292f288834aa4bcc86e1ecd6"
let ffiArtifactVersion = "0.1.20"
let ffiArtifactLocalPathOverride = ""
let hasReleasedFfiArtifact = ffiArtifactChecksum != ffiArtifactChecksumPlaceholder
let useRuntimeFfiProvider =
    ProcessInfo.processInfo.environment["REALLYME_CODEC_SWIFTPM_RUNTIME_FFI"] == "1"

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

packageTargets.append(
    .target(
        name: "ReallyMeCodec",
        dependencies: codecTargetDependencies,
        path: "packages/swift/Sources/ReallyMeCodec",
        swiftSettings: codecSwiftSettings
    )
)
packageTargets.append(
    .target(
        name: "ReallyMeCodecProto",
        dependencies: [
            .product(name: "SwiftProtobuf", package: "swift-protobuf"),
        ],
        path: "gen/swift"
    )
)
packageTargets.append(
    .testTarget(
        name: "ReallyMeCodecTests",
        dependencies: [
            "ReallyMeCodec",
            "ReallyMeCodecProto",
        ],
        path: "packages/swift/Tests/ReallyMeCodecTests"
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
        .library(
            name: "ReallyMeCodecProto",
            targets: ["ReallyMeCodecProto"]
        ),
    ],
    dependencies: [
        .package(
            url: "https://github.com/apple/swift-protobuf.git",
            from: "1.30.0"
        ),
    ],
    targets: packageTargets
)
