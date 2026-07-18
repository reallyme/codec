<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# ReallyMeCodec Swift

`ReallyMeCodec` is the Swift SDK facade for
[ReallyMe Codec](https://github.com/reallyme/codec), for Apple platforms. It
does not reimplement codecs in Swift; all operations delegate to the Rust codec
C ABI.

The manifest sits at the repository root (`Package.swift`) so SwiftPM can add it
by Git URL; the source lives under `packages/swift` with the other language SDKs.

## Install

```swift
.package(
    url: "https://github.com/reallyme/codec",
    from: "0.2.0"
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

Deterministic generic CBOR and DAG-CBOR use typed value builders:

```swift
let value = ReallyMeDeterministicCbor.mapText([
    ("b", ReallyMeDeterministicCbor.unsigned(2)),
    ("a", ReallyMeDeterministicCbor.bytes(Data([0, 1, 2]))),
])
let encodedCbor = try codec.deterministicCborEncodeData(value)
let decodedCbor = try codec.deterministicCborDecode(encodedCbor)
let dagCbor = try codec.dagCborEncodeData(ReallyMeDagCbor.mapText([
    ("payload", decodedCbor),
]))
```

The Swift model is deliberately generated-transport-shaped: recursive CBOR
values are serialized into the shared protobuf operation request, decoded with
a raised SwiftProtobuf message-depth limit derived from the documented
semantic nesting cap, validated for unknown fields and resource budgets, and
then handed to Rust for canonical bytes. Encoding canonicalizes map ordering;
decoding rejects duplicate semantic keys, non-canonical input, unsupported
CBOR types, and over-limit values.
The deterministic-CBOR builder supports integer and text map keys; the
`ReallyMeDagCbor` builder intentionally exposes text-key maps only because
DAG-CBOR has the stricter key profile. Both routes use the same generated
protobuf operation contract and bounded SwiftProtobuf depth/resource checks.

PEM input, output, and decoded DER use `[UInt8]` rather than `String` so
callers can clear private-key material promptly with their own memory policy.
Swift-only `Data` overloads are available for common byte boundaries, but
`[UInt8]` remains the canonical cross-language API shape.

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
REALLYME_CODEC_SWIFTPM_RUNTIME_FFI=1 \
REALLYME_CODEC_FFI_LIBRARY_PATH="$PWD/target/debug/libreallyme_codec_ffi.dylib" \
swift test
```
