# ReallyMeCodec Swift

Swift codecs for Apple platforms.

## Install

```swift
.package(
    url: "https://github.com/reallyme/codec",
    from: "0.2.1"
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

The Swift API includes base encodings, multiformats, DAG-CBOR, CID, PEM and JCS.

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

Encoding canonicalizes map ordering. Decoding rejects duplicate keys, non-canonical input, unsupported types and exceeded limits. DAG-CBOR accepts text map keys only.

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

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
