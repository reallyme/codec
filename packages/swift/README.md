<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# ReallyMeCodec Swift

`ReallyMeCodec` is the Swift SDK facade for
[ReallyMe Codec](https://github.com/reallyme/codec), for Apple platforms. It
does not reimplement codecs in Swift; all operations delegate to the Rust codec
C ABI so Rust remains the source of truth.

The manifest sits at the repository root (`Package.swift`) so SwiftPM can add it
by Git URL; the source lives under `packages/swift` with the other language SDKs.

## Install

```swift
.package(
    url: "https://github.com/reallyme/codec",
    from: "0.1.22"
)
```

```swift
.product(name: "ReallyMeCodec", package: "codec")
```

Applications that store or receive ReallyMe codec protobuf identifiers can add
the proto product at the same boundary:

```swift
.product(name: "ReallyMeCodecProto", package: "codec")
```

## Quick Start

Public SwiftPM releases include the Rust FFI `.xcframework` binary target, so
applications can construct the codec directly:

```swift
import ReallyMeCodec

let codec = try ReallyMeCodec()

let encoded = try codec.base64urlEncode([1, 2, 3])
let decoded = try codec.base64urlDecode(encoded)
```

`ReallyMeCodec` exposes PEM armor, lowercase hex, base64/base64url, multibase,
multicodec, multikey, DAG-CBOR, CID helpers, and JCS. The package does not
silently fall back to local Swift implementations.

PEM input, output, and decoded JSON use `[UInt8]` rather than `String` so
callers can clear private-key armor promptly with their own memory policy.

Local development builds can still pass an explicit Rust ABI library:

```sh
cargo build --locked -p reallyme-codec-ffi
```

```swift
let codecAbi = try ReallyMeCodecRustCAbiLibrary(path: "/path/to/libreallyme_codec_ffi.dylib")
let codec = try ReallyMeCodec(rustCAbiLibrary: codecAbi)
```

## Test

```sh
cargo build --locked -p reallyme-codec-ffi
REALLYME_CODEC_FFI_LIBRARY_PATH="$PWD/target/debug/libreallyme_codec_ffi.dylib" swift test
```
