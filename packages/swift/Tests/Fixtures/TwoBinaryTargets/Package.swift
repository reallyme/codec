// swift-tools-version: 6.3
// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import PackageDescription

let package = Package(
    name: "CodecBinaryConsumer",
    platforms: [.macOS(.v13), .iOS(.v16)],
    targets: [
        .executableTarget(
            name: "BinaryConsumer",
            dependencies: ["ReallyMeCodecFFI", "CSecp256k1"]
        ),
        .binaryTarget(name: "ReallyMeCodecFFI", path: "ReallyMeCodecFFI.xcframework"),
        .binaryTarget(name: "CSecp256k1", path: "CSecp256k1.xcframework"),
    ]
)
