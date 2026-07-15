# ReallyMeCodec Swift

Swift codecs for Apple platforms.

## Install

```swift
.package(
    url: "https://github.com/reallyme/codec",
    from: "0.1.20"
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

Local development builds can still pass an explicit Rust ABI library:

```sh
cargo build -p reallyme-codec-ffi
```

```swift
let codecAbi = try ReallyMeCodecRustCAbiLibrary(path: "/path/to/libreallyme_codec_ffi.dylib")
let codec = try ReallyMeCodec(rustCAbiLibrary: codecAbi)
```

## Test

```sh
cargo build -p reallyme-codec-ffi
REALLYME_CODEC_FFI_LIBRARY_PATH="$PWD/target/debug/libreallyme_codec_ffi.dylib" swift test
```

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
